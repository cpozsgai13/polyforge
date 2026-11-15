# ── Stage 1: build ─────────────────────────────────────────────────────────
FROM rust:bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p polyforge-server

# ── Stage 2: runtime ───────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        openssl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/polyforge-server ./

# Generate a self-signed TLS certificate for development
RUN mkdir -p certs data && \
    openssl req -x509 -newkey rsa:2048 \
        -keyout certs/key.pem \
        -out  certs/cert.pem \
        -days 3650 -nodes \
        -subj "/CN=polyforge"

EXPOSE 8443

ENV ADDRESS=0.0.0.0
ENV PORT=8443
ENV DATABASE_URL=sqlite:///app/data/data.db?mode=rwc
ENV TLS_CERT_PATH=/app/certs/cert.pem
ENV TLS_KEY_PATH=/app/certs/key.pem
ENV RUST_LOG=actix_web=info,polyforge_server=info

CMD ["/app/polyforge-server"]
