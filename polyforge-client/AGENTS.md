# polyforge-client

Example Rust CLI (reqwest + tokio) exercising the server API; single file [src/main.rs](src/main.rs).

- Base URL: first CLI arg, else `POLYFORGE_CLIENT_BASE_URL`, else `https://localhost:8443`.
- Accepts self-signed certs by default (`TRADING_ACCEPT_INVALID_CERTS=false` to disable).
- DTOs (`AddOrderRequest`, `BestQuoteResponse`, …) are hand-copied from the server's `routes/handlers/order_book_handler.rs`. Update both when the API changes.
- Quotes for many symbols are fetched concurrently with `join_all` into a `BTreeMap` cache.
- Run with the server up: `cargo run -p polyforge-client`.
