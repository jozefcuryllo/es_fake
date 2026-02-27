use axum::{
    body::{Body, Bytes},
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use http_body_util::BodyExt;

pub async fn debug_log(req: Request<Body>, next: Next) -> Result<Response<Body>, StatusCode> {
    let debug = std::env::var("DEBUG").map(|v| v == "true").unwrap_or(false);

    if !debug {
        return Ok(next.run(req).await);
    }

    let (parts, body) = req.into_parts();
    let bytes = buffer_body(body).await?;
    let req_str = String::from_utf8_lossy(&bytes);

    println!("--- DEBUG REQUEST ---");
    println!("Method: {} | Path: {}", parts.method, parts.uri);
    if !req_str.is_empty() {
        println!("Body: {}", req_str);
    }

    let req = Request::from_parts(parts, Body::from(bytes.clone()));
    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    let res_bytes = buffer_body(body).await?;
    let res_str = String::from_utf8_lossy(&res_bytes);

    println!("--- DEBUG RESPONSE ---");
    println!("Status: {}", parts.status);
    if !res_str.is_empty() {
        println!("Body: {}", res_str);
    }
    println!("----------------------");

    Ok(Response::from_parts(parts, Body::from(res_bytes)))
}

async fn buffer_body(body: Body) -> Result<Bytes, StatusCode> {
    body.collect()
        .await
        .map(|c| c.to_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::middleware::from_fn;
    use tower::{Layer, Service, ServiceExt};

    async fn handle_request(
        req: Request<Body>,
    ) -> Result<Response<Body>, std::convert::Infallible> {
        let (_parts, body) = req.into_parts();
        let bytes = body.collect().await.unwrap().to_bytes();

        let res_body = if bytes == "ping" {
            Body::from("pong")
        } else {
            Body::empty()
        };

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(res_body)
            .unwrap())
    }

    #[tokio::test]
    async fn should_pass_through_without_logging_when_debug_disabled() {
        unsafe {
            std::env::remove_var("DEBUG");
        }
        let layer = from_fn(debug_log);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let req = Request::builder().body(Body::from("ping")).unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();
        let body_bytes = buffer_body(res.into_body()).await.unwrap();

        assert_eq!(body_bytes, "pong");
    }

    #[tokio::test]
    async fn should_log_and_preserve_body_when_debug_enabled() {
        unsafe {
            std::env::set_var("DEBUG", "true");
        }
        let layer = from_fn(debug_log);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let req = Request::builder()
            .uri("/test-path")
            .body(Body::from("ping"))
            .unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();
        let body_bytes = buffer_body(res.into_body()).await.unwrap();

        assert_eq!(body_bytes, "pong");
    }

    #[tokio::test]
    async fn should_handle_empty_bodies() {
        unsafe {
            std::env::set_var("DEBUG", "true");
        }
        let layer = from_fn(debug_log);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let req = Request::builder().body(Body::empty()).unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
