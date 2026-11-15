# Polyforge

Rust order book, HTTPS server with live quotes/alerts/metrics, CLI client, market-data pipeline and a React UI.

## 1. Components

| Component | Purpose |
|---|---|
| [polyforge-core](polyforge-core/README.md) | Order book engine and shared types |
| [polyforge-server](polyforge-server/README.md) | Actix-web HTTPS REST/SSE server, metrics |
| [polyforge-client](polyforge-client/README.md) | Example CLI using the server API |
| [polyforge-pipeline](polyforge-pipeline/README.md) | File/TCP market-data processor |
| [polyforge-web](polyforge-web/README.md) | React UI |

## 2. Bring up the system step by step

Prerequisites: Rust toolchain, Node 18+, `curl`, and TLS certs at `polyforge-server/certs/cert.pem` and `key.pem`.

Self-signed cert example:
```sh
mkdir -p polyforge-server/certs
openssl req -x509 -newkey rsa:2048 -nodes -days 365 -subj "/CN=localhost" \
  -keyout polyforge-server/certs/key.pem -out polyforge-server/certs/cert.pem
```

**Step 1 – Build and test the workspace**
```sh
cargo build --workspace
cargo test -p polyforge-core
```
Verify: build finishes with no errors; tests pass.

**Step 2 – Start the server**
```sh
PORT=8443 cargo run -p polyforge-server
```
Verify (new terminal):
```sh
curl -k https://localhost:8443/home/test
curl -k https://localhost:8443/metrics | head
```
Both should return 200; the metrics output contains `polyforge_` series.

**Step 3 – Verify the order book and quotes**
```sh
curl -k https://localhost:8443/trading/AAPL/best_quote
curl -k https://localhost:8443/market/AAPL/quote
```
Expect JSON (`bid`/`ask` may be `null` until orders are added; the live quote needs internet).

**Step 4 – Run the CLI client**
```sh
cargo run -p polyforge-client
```
Verify: it prints quotes and acks without connection errors.

**Step 5 – Start the web UI**
```sh
cd polyforge-web && npm install && npm run dev
```
Verify: open http://localhost:5173; quotes appear and update every few seconds.

**Step 6 (optional) – Pipeline**
```sh
cargo run -p polyforge-pipeline -- polyforge-pipeline/config.ini
```
Verify: processor logs messages being applied. It does not talk to the server.

**Shutdown:** press Enter in the server terminal (saves `last_price.csv`).

## 3. Test with curl (key REST endpoints)

```sh
B=https://localhost:8443

# Add orders (side: bid|ask, order_type: gfd|ioc|market)
curl -k -X POST $B/trading/AAPL/add_order -H 'Content-Type: application/json' \
  -d '{"order_id":1,"price":100.50,"quantity":10,"side":"bid","order_type":"gfd"}'
curl -k -X POST $B/trading/AAPL/add_order -H 'Content-Type: application/json' \
  -d '{"order_id":2,"price":101.00,"quantity":5,"side":"ask","order_type":"gfd"}'

# Read the book
curl -k $B/trading/AAPL/best_bid
curl -k $B/trading/AAPL/best_ask
curl -k $B/trading/AAPL/best_quote

# Modify / cancel
curl -k -X POST   $B/trading/AAPL/modify_order -H 'Content-Type: application/json' \
  -d '{"order_id":1,"price":100.75,"quantity":12,"side":"bid"}'
curl -k -X DELETE $B/trading/AAPL/cancel_order -H 'Content-Type: application/json' \
  -d '{"order_id":1}'

# Live quote (Yahoo for equities, Kraken for symbols with '-')
curl -k $B/market/AAPL/quote
curl -k $B/market/BTC-USD/quote

# Alerts
curl -k -X POST $B/api/alerts -H 'Content-Type: application/json' \
  -d '{"symbol":"AAPL","threshold":150,"direction":"above","label":"AAPL > 150"}'
curl -k $B/api/alerts
curl -k -X DELETE $B/api/alerts/1
curl -k -N $B/api/alerts/events        # SSE stream; prints when an alert fires
```

## 4. Changing metrics and alerts

### Metrics
Exposed at `/metrics` (Prometheus format). Existing metrics: `polyforge_symbol_requests_total`, `polyforge_order_book_depth`, `polyforge_order_book_spread`, `polyforge_order_book_volume`, `polyforge_trades_total`, `polyforge_quote_errors_total`.

To add or change a metric:
1. Add the field to `AppState` in [polyforge-server/src/utils/app_state.rs](polyforge-server/src/utils/app_state.rs).
2. In [polyforge-server/src/main.rs](polyforge-server/src/main.rs), create it (`CounterVec`/`GaugeVec` with `prometheus::opts!`), `registry.register(...)` it, and pass it into the `AppState` built in `HttpServer::new`.
3. Update it where the event occurs, e.g. `state.symbol_requests.with_label_values(&[sym, "op"]).inc()` in a handler.
4. Gauge refresh interval (order-book depth/spread/volume): the `from_secs(10)` sleep in the background gauge task in `main.rs`.

Verify: `curl -k https://localhost:8443/metrics | grep polyforge_`.

### Alerts
- Create/delete alerts at runtime via the `/api/alerts` endpoints above (`direction` is `above` or `below`; fires when price `>=`/`<=` threshold).
- Check interval: the `from_secs(5)` sleep in the alert checker task in [polyforge-server/src/main.rs](polyforge-server/src/main.rs). It compares against the last price in `market_prices`.
- Trigger logic (new conditions): the `match alert.direction` block in the same task; also validate the new value in `create_alert` in [polyforge-server/src/routes/alert_routes.rs](polyforge-server/src/routes/alert_routes.rs).
- Event payload: `AlertEvent` in `app_state.rs` (SSE JSON).
- Schema changes: add a migration in `polyforge-server/migration/src/` (`mYYYYMMDD_NNNNNN_*.rs`), register it in `migration/src/lib.rs`, and update `src/entities/price_alert.rs`.
- Keep the web DTOs ([polyforge-web/src/api/types.ts](polyforge-web/src/api/types.ts)) and client DTOs in sync with any API change.
