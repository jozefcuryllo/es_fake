FROM rust:1.90-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache tini
WORKDIR /app
COPY --from=builder /app/target/release/es_fake .

ENV ELASTIC_PASSWORD=""

EXPOSE 9200

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["./es_fake"]