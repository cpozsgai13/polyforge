# polyforge-server

Actix-web HTTPS server: order books, live quotes (Kraken/Yahoo), price alerts (SSE) and Prometheus metrics.

## Build
```sh
cargo build -p polyforge-server
```

## Run
Requires `certs/cert.pem` and `certs/key.pem` (self-signed is fine) in this folder.
```sh
PORT=8443 cargo run -p polyforge-server
```
Press Enter in the terminal for graceful shutdown (rewrites `last_price.csv`).

## Test
```sh
cargo test -p polyforge-server                # offline tests
cargo test -p polyforge-server -- --ignored   # network tests
```

## Docker
```sh
docker build -f polyforge-server/Dockerfile .   # from repo root
```
