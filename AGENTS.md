# Polyforge – agent instructions (root)

Cargo workspace (`polyforge-core`, `polyforge-pipeline`, `polyforge-server`, `polyforge-client`) plus a separate Vite/React app `polyforge-web` (not a Cargo member). Each sub-project has its own `AGENTS.md` with local conventions; the nearest one wins.

Shared deps (serde, serde_json, tokio, thiserror) are pinned in the root [Cargo.toml](Cargo.toml) and used via `workspace = true`.

## How the pieces fit
- `polyforge-core` is the only source of domain types (order book, prices, messages). Server and pipeline both depend on it.
- `polyforge-server` exposes the order book + live quotes over HTTPS/REST/SSE.
- `polyforge-client` (Rust CLI) and `polyforge-web` (React) are consumers of the server API. **Their DTOs mirror the server's Rust structs by hand**: change all together.
- `polyforge-pipeline` is an independent file/TCP market-data processor built on core; it does not talk to the server.

## Cross-cutting conventions
- Much of the Rust is a port of a C++ order book (comments cite `OrderBook.h`, `MarketDataDefinitions.h`). Keep C++-mirroring names.
- Symbols are always uppercase; symbols containing `-` (e.g. `BTC-USD`) are crypto.
- `.env` is loaded from CWD, then `../.env` relative to each crate's `CARGO_MANIFEST_DIR`.
- Root [last_price.csv](last_price.csv) (`SYMBOL, last, bid, ask`; `#` comments) seeds the server's books and is rewritten on shutdown.

## Common commands
- `cargo build --workspace` (VS Code build tasks exist per crate)
- `cargo test -p polyforge-server` (network tests are `#[ignore]`; add `-- --ignored`)
- Server: `PORT=8443 cargo run -p polyforge-server`; web: `cd polyforge-web && npm run dev` (proxies to `https://localhost:8443`).
