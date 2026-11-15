# polyforge-client

Example Rust CLI that exercises the server API.

## Build
```sh
cargo build -p polyforge-client
```

## Run
Start the server first.
```sh
cargo run -p polyforge-client                          # https://localhost:8443
cargo run -p polyforge-client -- https://localhost:8443
```
Env: `POLYFORGE_CLIENT_BASE_URL`, `TRADING_ACCEPT_INVALID_CERTS=false` to enforce cert validation.
