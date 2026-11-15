# polyforge-server

Rust Actix-web server over **rustls (HTTPS only)**; default `127.0.0.1:5050` (Docker and web dev use 8443). Needs `certs/cert.pem` + `certs/key.pem`.

## State and wiring
- `AppState` ([src/utils/app_state.rs](src/utils/app_state.rs)): SQLite (sea-orm 0.12), `order_books: Arc<Mutex<HashMap<symbol, OrderBook<Price>>>>`, `market_prices` fallback cache, `quote_provider`, SSE `alert_tx` broadcast, Prometheus metrics. Uses **std Mutex: never hold across `.await`**.
- Everything is wired in [src/main.rs](src/main.rs): seeding, background tasks (alert checker every 5 s, gauge refresh every 10 s), metrics registry, graceful shutdown.

## Conventions
- Routes: each `src/routes/*_routes.rs` exposes `config(&mut web::ServiceConfig)` and is registered in `main.rs` with `.configure(...)`. Handlers live in `src/routes/handlers/`. Paths: `/trading/{symbol}/…`, `/market/{symbol}/quote`, `/api/alerts…` (SSE at `/api/alerts/events`).
- When adding a top-level path, update the proxy in `polyforge-web/vite.config.ts`.
- Handlers use DTO structs and `AckResponse { success, message }`, call `state.symbol_requests.with_label_values(&[sym, op]).inc()`, and log with `tracing::info!(symbol = %sym, ...)`.
- New metrics: add the field to `AppState` **and** create/register it in `main.rs`.
- Quotes: implement `QuoteProvider` ([src/quote_provider/mod.rs](src/quote_provider/mod.rs), `async_trait`, returns `LiveQuote`/`QuoteProviderError`). `RoutingQuoteProvider` sends symbols containing `-` to Kraken, others to Yahoo. `binance.rs` exists but is not wired in.
- Config: `lazy_static` env constants in [src/utils/constants.rs](src/utils/constants.rs) (`PORT`, `ADDRESS`, `DATABASE_URL`, `TLS_CERT_PATH/KEY_PATH`, `LAST_PRICE_FILE`, `CRYPTO_SYMBOLS`). Read them only after dotenv loads. Relative cert paths resolve against `CARGO_MANIFEST_DIR`; `last_price.csv` against the workspace root.
- DB: migrations in `migration/` (sea-orm-migration). `Migrator::up` runs at startup. Add `mYYYYMMDD_NNNNNN_*.rs` and register it in `migration/src/lib.rs`. Entities in `src/entities/` are hand-maintained.
- `utils/jwt.rs` and user entities exist but no routes use them.

## Workflows
- Run: `PORT=8443 cargo run -p polyforge-server`; Enter in a TTY triggers shutdown and rewrites `last_price.csv` (crypto skipped).
- Tests: `cargo test -p polyforge-server`; network tests are `#[ignore]` (`-- --ignored`). Use stub `QuoteProvider`s (see `yahoo.rs` tests) instead of the network.
- Docker: `docker build -f polyforge-server/Dockerfile .` from the repo root (generates a self-signed cert). Metrics at `/metrics`.
