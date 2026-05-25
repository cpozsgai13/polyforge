//! Binance quote provider.
//!
//! Uses two public Binance REST endpoints (no API key required):
//!
//! | Endpoint                          | Data                        |
//! |-----------------------------------|-----------------------------|
//! | `/api/v3/ticker/bookTicker`       | Best bid price + qty, best ask price + qty |
//! | `/api/v3/ticker/price`            | Last traded price           |
//!
//! Crypto symbols must use Binance's convention: `BTC-USD` → `BTCUSDT`
//! (hyphens removed, trailing `USD` → `USDT`).

use async_trait::async_trait;
use log;
use reqwest::Client;
use serde::Deserialize;
use serde_json;

use super::{LiveQuote, QuoteProvider, QuoteProviderError};

const BINANCE_BASE: &str = "https://api.binance.com";

// ── Binance response shapes ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BookTicker {
    #[serde(rename = "bidPrice")]
    bid_price: String,
    #[serde(rename = "bidQty")]
    bid_qty: String,
    #[serde(rename = "askPrice")]
    ask_price: String,
    #[serde(rename = "askQty")]
    ask_qty: String,
}

#[derive(Debug, Deserialize)]
struct PriceTicker {
    price: String,
}

/// Binance API error body (returned when the symbol is invalid or the
/// request is malformed).
#[derive(Debug, Deserialize)]
struct BinanceError {
    code: i32,
    msg:  String,
}

// ── Helper: extract JSON or surface the Binance error message ─────────────────

async fn parse_or_binance_error<T: for<'de> Deserialize<'de>>(
    resp: reqwest::Response,
    context: &str,
) -> Result<T, QuoteProviderError> {
    let status = resp.status();
    let body = resp.text().await
        .map_err(|e| QuoteProviderError(format!("{context}: failed to read body: {e}")))?;

    if !status.is_success() {
        // Try to decode as a Binance error envelope first for a nicer message.
        if let Ok(err) = serde_json::from_str::<BinanceError>(&body) {
            return Err(QuoteProviderError(
                format!("{context}: Binance error {}: {}", err.code, err.msg)
            ));
        }
        return Err(QuoteProviderError(
            format!("{context}: HTTP {status}, body: {body}")
        ));
    }

    serde_json::from_str::<T>(&body)
        .map_err(|e| QuoteProviderError(format!("{context}: JSON parse error: {e} — body: {body}")))
}

/// Quote provider backed by Binance public REST API.
///
/// No API key is required for the endpoints used.
#[derive(Debug, Clone)]
pub struct BinanceQuoteProvider {
    client: Client,
}

impl BinanceQuoteProvider {
    pub fn new() -> Result<Self, QuoteProviderError> {
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map(|client| Self { client })
            .map_err(|e| QuoteProviderError(format!("Binance HTTP client init failed: {e}")))
    }

    /// Convert a Yahoo-style symbol to a Binance pair.
    ///
    /// Examples:
    /// - `BTC-USD`  → `BTCUSDT`
    /// - `ETH-USD`  → `ETHUSDT`
    /// - `SOL-USDT` → `SOLUSDT`
    fn to_binance_symbol(symbol: &str) -> String {
        let upper = symbol.to_uppercase();
        // Strip the hyphen; replace trailing USD with USDT if not already USDT.
        if let Some((base, quote)) = upper.split_once('-') {
            let quote = if quote == "USD" { "USDT" } else { quote };
            format!("{base}{quote}")
        } else {
            upper
        }
    }
}

// ── QuoteProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl QuoteProvider for BinanceQuoteProvider {
    fn provider_name(&self) -> &'static str {
        "binance"
    }

    async fn fetch_quote(&self, symbol: &str) -> Result<LiveQuote, QuoteProviderError> {
        let bin_sym = Self::to_binance_symbol(symbol);

        // Fire both requests concurrently.
        let book_url  = format!("{BINANCE_BASE}/api/v3/ticker/bookTicker?symbol={bin_sym}");
        let price_url = format!("{BINANCE_BASE}/api/v3/ticker/price?symbol={bin_sym}");

        log::debug!("Binance fetch: bookTicker {book_url} + price {price_url} for {bin_sym}");

        let (book_res, price_res) = tokio::join!(
            self.client.get(&book_url).send(),
            self.client.get(&price_url).send(),
        );

        log::debug!("Binance responses: bookTicker={:?} price={:?}", book_res, price_res);

        let book_resp = book_res
            .map_err(|e| QuoteProviderError(format!("Binance bookTicker request failed for '{bin_sym}': {e}")))?;
        let price_resp = price_res
            .map_err(|e| QuoteProviderError(format!("Binance price request failed for '{bin_sym}': {e}")))?;

        let book: BookTicker = parse_or_binance_error(
            book_resp, &format!("bookTicker({bin_sym})")
        ).await?;
        let price: PriceTicker = parse_or_binance_error(
            price_resp, &format!("price({bin_sym})")
        ).await?;

        let parse = |s: &str| -> Option<f64> { s.parse().ok().filter(|&v: &f64| v > 0.0) };

        let quote = LiveQuote {
            symbol:       symbol.to_uppercase(),
            bid:          parse(&book.bid_price),
            ask:          parse(&book.ask_price),
            last_price:   parse(&price.price),
            currency:     Some("USD".to_string()),
            market_state: Some("REGULAR".to_string()),
        };

        log::debug!(
            "Binance result: {} last={} bid={} ask={}",
            bin_sym,
            quote.last_price.map_or("—".to_string(), |v| format!("{v:.4}")),
            quote.bid.map_or("—".to_string(), |v| format!("{v:.4}")),
            quote.ask.map_or("—".to_string(), |v| format!("{v:.4}")),
        );

        Ok(quote)
    }
}
