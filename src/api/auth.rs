use crate::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use base64::{Engine as _, engine::general_purpose};
use std::sync::Arc;

pub async fn basic_auth(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.auth_enabled {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Basic ") => {
            let credential_part = &header[6..];
            if let Ok(decoded) = general_purpose::STANDARD.decode(credential_part) {
                if let Ok(decoded_str) = String::from_utf8(decoded) {
                    let mut parts = decoded_str.splitn(2, ':');
                    let username = parts.next().unwrap_or("");
                    let password = parts.next().unwrap_or("");

                    if username == state.auth_user && password == state.auth_password {
                        return Ok(next.run(req).await);
                    }
                }
            }
        }
        _ => {}
    }

    if std::env::var("DEBUG").map(|v| v == "true").unwrap_or(false) {
        println!("--- AUTH FAILED ---");
        println!("Path: {}", req.uri());
    }

    Err(StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::store::InMemoryStore;
    use axum::middleware::from_fn_with_state;
    use tower::{Layer, Service, ServiceExt};

    fn setup_state(enabled: bool) -> Arc<AppState> {
        Arc::new(AppState {
            store: InMemoryStore::new(),
            auth_user: "elastic".to_string(),
            auth_password: "password123".to_string(),
            auth_enabled: enabled,
        })
    }

    async fn handle_request(_req: Request<Body>) -> Result<Response, std::convert::Infallible> {
        Ok(Response::new(Body::empty()))
    }

    #[tokio::test]
    async fn should_allow_anonymous_when_auth_disabled() {
        let state = setup_state(false);
        let layer = from_fn_with_state(state, basic_auth);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let req = Request::builder().body(Body::empty()).unwrap();
        let res = service.ready().await.unwrap().call(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn should_return_unauthorized_when_header_missing() {
        let state = setup_state(true);
        let layer = from_fn_with_state(state, basic_auth);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let req = Request::builder().body(Body::empty()).unwrap();
        let res = service.call(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn should_allow_valid_credentials() {
        let state = setup_state(true);
        let layer = from_fn_with_state(state, basic_auth);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let auth = general_purpose::STANDARD.encode("elastic:password123");
        let req = Request::builder()
            .header(header::AUTHORIZATION, format!("Basic {}", auth))
            .body(Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn should_reject_invalid_password() {
        let state = setup_state(true);
        let layer = from_fn_with_state(state, basic_auth);
        let mut service = layer.layer(tower::service_fn(handle_request));

        let auth = general_purpose::STANDARD.encode("elastic:wrong");
        let req = Request::builder()
            .header(header::AUTHORIZATION, format!("Basic {}", auth))
            .body(Body::empty())
            .unwrap();

        let res = service.call(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
