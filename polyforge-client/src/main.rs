//! trading_client — example Rust client for the order-book web server.
//!
//! Run (with the server already started on port 8443):
//!
//! ```
//! cargo run --example trading_client
//! ```
//!
//! You can pass the server URL as the first startup argument.
//! If omitted, `POLYFORGE_CLIENT_BASE_URL` from `.env` is used.
//! The client accepts the server's self-signed certificate by default
//! (set the `TRADING_ACCEPT_INVALID_CERTS` env-var to `"false"` to disable).

use dotenv::dotenv;
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

const DEFAULT_BASE_URL: &str = "https://localhost:8443";

// ── Quote cache ───────────────────────────────────────────────────────────────

/// Snapshot of top-of-book quotes for all subscribed symbols,
/// keyed by symbol in ascending lexicographic order.
///
/// A value of `Ok(q)` means the server returned a valid quote (either side
/// may still be `None` when there are no resting orders).
/// A value of `Err(msg)` means the fetch failed (network error or unknown symbol).
type QuoteCache = BTreeMap<String, Result<BestQuoteResponse, String>>;

// ── DTOs (must mirror the server-side definitions) ────────────────────────────

#[derive(Debug, Serialize)]
struct AddOrderRequest {
    order_id:      u64,
    price:         f64,
    quantity:      u64,
    side:          String,
    order_type:    String,
    instrument_id: u64,
    timestamp_ns:  u64,
}

#[derive(Debug, Serialize)]
struct ModifyOrderRequest {
    order_id:      u64,
    price:         f64,
    quantity:      u64,
    side:          String,
    instrument_id: u64,
    timestamp_ns:  u64,
}

#[derive(Debug, Serialize)]
struct CancelOrderRequest {
    order_id:      u64,
    instrument_id: u64,
}

#[derive(Debug, Deserialize)]
struct AckResponse {
    success: bool,
    message: String,
}

/// One side of the best-bid/ask response (`null` when no resting orders).
#[derive(Debug, Deserialize)]
struct QuoteLevel {
    price:  f64,
    volume: u64,
}

/// Combined best-bid-and-ask response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BestQuoteResponse {
    symbol: String,
    bid:    Option<QuoteLevel>,
    ask:    Option<QuoteLevel>,
}

// ── Client wrapper ────────────────────────────────────────────────────────────

struct TradingClient {
    client: Client,
    base:   String,
}

impl TradingClient {
    fn new(base: &str, accept_invalid_certs: bool) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()?;
        Ok(Self { client, base: base.to_string() })
    }

    fn url(&self, symbol: &str, path: &str) -> String {
        format!("{}/trading/{}/{}", self.base, symbol, path)
    }

    /// Add a new order to the book.
    async fn add_order(
        &self,
        symbol:     &str,
        order_id:   u64,
        price:      f64,
        quantity:   u64,
        side:       &str,
        order_type: &str,
    ) -> Result<AckResponse, reqwest::Error> {
        let body = AddOrderRequest {
            order_id,
            price,
            quantity,
            side:          side.to_string(),
            order_type:    order_type.to_string(),
            instrument_id: 0,
            timestamp_ns:  0,
        };
        self.client
            .post(self.url(symbol, "add_order"))
            .json(&body)
            .send()
            .await?
            .json::<AckResponse>()
            .await
    }

    /// Modify an existing resting order.
    async fn modify_order(
        &self,
        symbol:   &str,
        order_id: u64,
        price:    f64,
        quantity: u64,
        side:     &str,
    ) -> Result<AckResponse, reqwest::Error> {
        let body = ModifyOrderRequest {
            order_id,
            price,
            quantity,
            side:          side.to_string(),
            instrument_id: 0,
            timestamp_ns:  0,
        };
        self.client
            .post(self.url(symbol, "modify_order"))
            .json(&body)
            .send()
            .await?
            .json::<AckResponse>()
            .await
    }

    /// Cancel a resting order.
    async fn cancel_order(
        &self,
        symbol:   &str,
        order_id: u64,
    ) -> Result<AckResponse, reqwest::Error> {
        let body = CancelOrderRequest { order_id, instrument_id: 0 };
        self.client
            .delete(self.url(symbol, "cancel_order"))
            .json(&body)
            .send()
            .await?
            .json::<AckResponse>()
            .await
    }

    /// Query the best bid.
    async fn best_bid(&self, symbol: &str) -> Result<serde_json::Value, reqwest::Error> {
        self.client
            .get(self.url(symbol, "best_bid"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await
    }

    /// Query the best ask.
    async fn best_ask(&self, symbol: &str) -> Result<serde_json::Value, reqwest::Error> {
        self.client
            .get(self.url(symbol, "best_ask"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await
    }

    /// Query best bid and ask in a single round-trip (internal order book).
    ///
    /// Returns a [`BestQuoteResponse`] where either side may be `None` when
    /// there are no resting orders on that side.  Returns `Err` for any
    /// non-2xx HTTP status (e.g. 404 when the symbol is unknown) as well as
    /// network failures.
    async fn best_quote(
        &self,
        symbol: &str,
    ) -> Result<BestQuoteResponse, reqwest::Error> {
        self.client
            .get(self.url(symbol, "best_quote"))
            .send()
            .await?
            .error_for_status()?
            .json::<BestQuoteResponse>()
            .await
    }

    /// Fetch a live quote (bid/ask/last) from the server's upstream vendor
    /// (default: Yahoo Finance).  Returns `Err` for non-2xx responses.
    async fn live_quote(&self, symbol: &str) -> Result<serde_json::Value, reqwest::Error> {
        self.client
            .get(format!("{}/market/{}/quote", self.base, symbol))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await
    }
}

// ── Startup quote fetch ───────────────────────────────────────────────────────

/// Fetch best quotes for every symbol concurrently and return them in a
/// [`QuoteCache`] (`BTreeMap` keyed by symbol for O(log n) lookups and
/// naturally sorted iteration).
async fn fetch_all_quotes(client: &TradingClient, symbols: &[String]) -> QuoteCache {
    let fetches = symbols.iter().map(|sym| async move {
        let result = client
            .best_quote(sym)
            .await
            .map_err(|e| e.to_string());
        (sym.clone(), result)
    });
    join_all(fetches).await.into_iter().collect()
}

fn print_quote_cache(cache: &QuoteCache) {
    // Column interior widths (excluding the surrounding │ and padding):
    //   symbol : 8 chars  →  cell = "  {:<8} " = 11 wide
    //   bid/ask: 22 chars →  cell = "  {:>22} " = 25 wide
    // Total row width: 1 + 11 + 1 + 25 + 1 + 25 + 1 = 65
    println!("┌{:─<11}┬{:─<25}┬{:─<25}┐", "", "", "");
    println!("│  {:<8} │  {:>22} │  {:>22} │", "Symbol", "Bid price / vol", "Ask price / vol");
    println!("├{:─<11}┼{:─<25}┼{:─<25}┤", "", "", "");
    for (sym, result) in cache {
        let (bid_str, ask_str) = match result {
            Err(_) => ("0.0000 x 0".to_string(), "0.0000 x 0".to_string()),
            Ok(q)  => (
                q.bid.as_ref()
                    .map(|b| format!("{:.4} x {}", b.price, b.volume))
                    .unwrap_or_else(|| "(empty)".to_string()),
                q.ask.as_ref()
                    .map(|a| format!("{:.4} x {}", a.price, a.volume))
                    .unwrap_or_else(|| "(empty)".to_string()),
            ),
        };
        println!("│  {:<8} │  {:>22} │  {:>22} │", sym, bid_str, ask_str);
    }
    println!("└{:─<11}┴{:─<25}┴{:─<25}┘", "", "", "");
}

// ── Demo scenario ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env from current directory, then workspace root as a fallback.
    dotenv().ok();
    let workspace_env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env");
    let _ = dotenv::from_path(workspace_env);

    let mut args = env::args().skip(1);
    let base_url = args
        .next()
        .or_else(|| env::var("POLYFORGE_CLIENT_BASE_URL").ok())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    // Parse comma-separated symbol subscription list.
    let symbols: Vec<String> = env::var("POLYFORGE_CLIENT_SYMBOLS")
        .unwrap_or_else(|_| "AAPL".to_string())
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    let accept_invalid = env::var("TRADING_ACCEPT_INVALID_CERTS")
        .unwrap_or_else(|_| "true".to_string())
        != "false";

    let client = TradingClient::new(&base_url, accept_invalid)?;

    // ── Startup snapshot ──────────────────────────────────────────────────────
    println!("Fetching startup quotes for symbols {:?} ({} total)", symbols, symbols.len());
    let mut quote_cache: QuoteCache = fetch_all_quotes(&client, &symbols).await;
    print_quote_cache(&quote_cache);
    println!();
    // Also fetch live vendor quotes for each subscribed symbol.
    println!("=== Live vendor quotes (via server's quote provider) ===");
    for symbol in &symbols {
        match client.live_quote(symbol).await {
            Ok(q)  => println!("  {symbol}: {}", serde_json::to_string(&q).unwrap_or_default()),
            Err(_) => println!("  {symbol}:"),
        }
        match client.best_bid(symbol).await {
            Ok(q)  => println!("    best_bid: {}", serde_json::to_string(&q).unwrap_or_default()),
            Err(_) => println!("    best_bid:"),
        }
        match client.best_ask(symbol).await {
            Ok(q)  => println!("    best_ask: {}", serde_json::to_string(&q).unwrap_or_default()),
            Err(_) => println!("    best_ask:"),
        }
    }
    println!();
    // // ── Demo scenario (runs on the first subscribed symbol) ───────────────────
    // let symbol = symbols.first().cloned().unwrap_or_else(|| "AAPL".to_string());
    // println!("=== Trading Client Demo (symbol: {symbol}) ===\n");

    // // 1. Add a resting bid at $149.50 for 100 shares
    // let r = client.add_order(&symbol, 1001, 149.50, 100, "bid", "gfd").await?;
    // println!("[add_order bid 149.50x100]  success={} msg={}", r.success, r.message);

    // // 2. Add a resting ask at $150.00 for 50 shares
    // let r = client.add_order(&symbol, 1002, 150.00, 50, "ask", "gfd").await?;
    // println!("[add_order ask 150.00x50]   success={} msg={}", r.success, r.message);

    // // 3. Add another resting bid at $149.00
    // let r = client.add_order(&symbol, 1003, 149.00, 200, "bid", "gfd").await?;
    // println!("[add_order bid 149.00x200]  success={} msg={}", r.success, r.message);

    // // 4. Query best bid
    // let quote = client.best_bid(&symbol).await?;
    // println!("[best_bid]  {}", serde_json::to_string_pretty(&quote)?);

    // // 5. Query best ask
    // let quote = client.best_ask(&symbol).await?;
    // println!("[best_ask]  {}", serde_json::to_string_pretty(&quote)?);

    // // 6. Modify order 1001: raise bid to $149.75 for 80 shares
    // let r = client.modify_order(&symbol, 1001, 149.75, 80, "bid").await?;
    // println!("[modify_order 1001→149.75]  success={} msg={}", r.success, r.message);

    // // 7. Query best bid again after modify
    // let quote = client.best_bid(&symbol).await?;
    // println!("[best_bid after modify]  {}", serde_json::to_string_pretty(&quote)?);

    // // 8. Cancel order 1003
    // let r = client.cancel_order(&symbol, 1003).await?;
    // println!("[cancel_order 1003]  success={} msg={}", r.success, r.message);

    // // 9. Add a crossing bid that triggers a match (IOC at market price)
    // let r = client.add_order(&symbol, 1004, 150.50, 30, "bid", "ioc").await?;
    // println!("[add_order IOC bid 150.50x30 — should match ask]  success={} msg={}", r.success, r.message);

    // 10. Final quotes — update cache entry for the demo symbol and redisplay.
    // let updated = client.best_quote(&symbol).await.map_err(|e| e.to_string());
    // quote_cache.insert(symbol.clone(), updated);

    // println!("\nUpdated quote cache after demo:");
    // print_quote_cache(&quote_cache);

    Ok(())
}
