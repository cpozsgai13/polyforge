//! trading_client — example Rust client for the order-book web server.
//!
//! Run (with the server already started on port 8443):
//!
//! ```
//! cargo run --example trading_client
//! ```
//!
//! All HTTPS requests are sent to `https://localhost:8443`.
//! The client accepts the server's self-signed certificate by default
//! (set the `TRADING_ACCEPT_INVALID_CERTS` env-var to `"false"` to disable).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const BASE_URL: &str = "https://localhost:8443";

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
}

// ── Demo scenario ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let accept_invalid = env::var("TRADING_ACCEPT_INVALID_CERTS")
        .unwrap_or_else(|_| "true".to_string())
        != "false";

    let client = TradingClient::new(BASE_URL, accept_invalid)?;
    let symbol = "AAPL";

    println!("=== Trading Client Demo (symbol: {symbol}) ===\n");

    // 1. Add a resting bid at $149.50 for 100 shares
    let r = client.add_order(symbol, 1001, 149.50, 100, "bid", "gfd").await?;
    println!("[add_order bid 149.50x100]  success={} msg={}", r.success, r.message);

    // 2. Add a resting ask at $150.00 for 50 shares
    let r = client.add_order(symbol, 1002, 150.00, 50, "ask", "gfd").await?;
    println!("[add_order ask 150.00x50]   success={} msg={}", r.success, r.message);

    // 3. Add another resting bid at $149.00
    let r = client.add_order(symbol, 1003, 149.00, 200, "bid", "gfd").await?;
    println!("[add_order bid 149.00x200]  success={} msg={}", r.success, r.message);

    // 4. Query best bid
    let quote = client.best_bid(symbol).await?;
    println!("[best_bid]  {}", serde_json::to_string_pretty(&quote)?);

    // 5. Query best ask
    let quote = client.best_ask(symbol).await?;
    println!("[best_ask]  {}", serde_json::to_string_pretty(&quote)?);

    // 6. Modify order 1001: raise bid to $149.75 for 80 shares
    let r = client.modify_order(symbol, 1001, 149.75, 80, "bid").await?;
    println!("[modify_order 1001→149.75]  success={} msg={}", r.success, r.message);

    // 7. Query best bid again after modify
    let quote = client.best_bid(symbol).await?;
    println!("[best_bid after modify]  {}", serde_json::to_string_pretty(&quote)?);

    // 8. Cancel order 1003
    let r = client.cancel_order(symbol, 1003).await?;
    println!("[cancel_order 1003]  success={} msg={}", r.success, r.message);

    // 9. Add a crossing bid that triggers a match (IOC at market price)
    let r = client.add_order(symbol, 1004, 150.50, 30, "bid", "ioc").await?;
    println!("[add_order IOC bid 150.50x30 — should match ask]  success={} msg={}", r.success, r.message);

    // 10. Final quotes
    let bid = client.best_bid(symbol).await?;
    let ask = client.best_ask(symbol).await?;
    println!("\nFinal quotes:");
    println!("  best_bid: {}", serde_json::to_string(&bid)?);
    println!("  best_ask: {}", serde_json::to_string(&ask)?);

    Ok(())
}
