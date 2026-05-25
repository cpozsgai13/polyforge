//! Kraken quote provider.
//!
//! Uses the public Kraken REST endpoint — no API key required, US-accessible.
//!
//! | Endpoint                  | Data                                   |
//! |---------------------------|----------------------------------------|
//! | `/0/public/Ticker?pair=X` | bid, ask, last price, volume           |
//!
//! Symbol mapping (Yahoo → Kraken):
//! - `BTC-USD` → `XBTUSD`
//! - `ETH-USD` → `ETHUSD`
//! - Anything else: strip the `-` (e.g. `SOL-USD` → `SOLUSD`)
//!
//! Kraken returns an internal key name in `result` that may differ from the
//! requested pair (e.g. `XBTUSD` → `XXBTZUSD`). We pick the first key in
//! `result` so we don't need a hard-coded map.

use std::collections::HashMap;

use async_trait::async_trait;
use log;
use reqwest::Client;
use serde::Deserialize;
use serde_json;

use super::{LiveQuote, QuoteProvider, QuoteProviderError};

const KRAKEN_BASE: &str = "https://api.kraken.com";

// ── Kraken response shapes ────────────────────────────────────────────────────

/// A single ticker entry. Fields are arrays; we only need index 0.
/// - `a` = ask  [price, whole_lot_volume, lot_volume]
/// - `b` = bid  [price, whole_lot_volume, lot_volume]
/// - `c` = last [price, lot_volume]
#[derive(Debug, Deserialize)]
struct KrakenTicker {
    /// ask: [price, ...]
    a: Vec<String>,
    /// bid: [price, ...]
    b: Vec<String>,
    /// last trade closed: [price, ...]
    c: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct KrakenTickerResponse {
    error:  Vec<String>,
    result: Option<HashMap<String, KrakenTicker>>,
}

// ── Provider struct ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct KrakenQuoteProvider {
    client: Client,
}

impl KrakenQuoteProvider {
    pub fn new() -> Result<Self, QuoteProviderError> {
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map(|client| Self { client })
            .map_err(|e| QuoteProviderError(format!("Kraken HTTP client init failed: {e}")))
    }

    /// Convert a Yahoo-style symbol to a Kraken pair name.
    ///
    /// Examples:
    /// - `BTC-USD` → `XBTUSD`
    /// - `ETH-USD` → `ETHUSD`
    /// - `SOL-USD` → `SOLUSD`
    fn to_kraken_pair(symbol: &str) -> String {
        let upper = symbol.to_uppercase();
        if let Some((base, quote)) = upper.split_once('-') {
            // BTC is special-cased as XBT on Kraken.
            let base = if base == "BTC" { "XBT" } else { base };
            format!("{base}{quote}")
        } else {
            upper
        }
    }
}

// ── QuoteProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl QuoteProvider for KrakenQuoteProvider {
    fn provider_name(&self) -> &'static str {
        "kraken"
    }

    async fn fetch_quote(&self, symbol: &str) -> Result<LiveQuote, QuoteProviderError> {
        let pair = Self::to_kraken_pair(symbol);
        let url  = format!("{KRAKEN_BASE}/0/public/Ticker?pair={pair}");

        log::debug!("Kraken fetch: {url}");

        let body = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| QuoteProviderError(format!("Kraken request failed for '{pair}': {e}")))?
            .text()
            .await
            .map_err(|e| QuoteProviderError(format!("Kraken body read failed for '{pair}': {e}")))?;

        let resp: KrakenTickerResponse = serde_json::from_str(&body)
            .map_err(|e| QuoteProviderError(
                format!("Kraken parse failed for '{pair}': {e} — body: {body}")
            ))?;

        // Surface any API-level errors (e.g. invalid pair).
        if !resp.error.is_empty() {
            return Err(QuoteProviderError(
                format!("Kraken error for '{pair}': {}", resp.error.join(", "))
            ));
        }

        let result = resp.result
            .ok_or_else(|| QuoteProviderError(format!("Kraken: empty result for '{pair}'")))?;

        // Kraken may return the pair under a different internal key (e.g.
        // XBTUSD → XXBTZUSD). Take the first (and only) entry.
        let ticker = result.into_values().next()
            .ok_or_else(|| QuoteProviderError(format!("Kraken: no ticker in result for '{pair}'")))?;

        let parse = |v: &[String]| -> Option<f64> {
            v.first().and_then(|s| s.parse().ok()).filter(|&v: &f64| v > 0.0)
        };

        let quote = LiveQuote {
            symbol:       symbol.to_uppercase(),
            bid:          parse(&ticker.b),
            ask:          parse(&ticker.a),
            last_price:   parse(&ticker.c),
            currency:     Some("USD".to_string()),
            market_state: Some("REGULAR".to_string()),
        };

        log::debug!(
            "Kraken result: {} last={} bid={} ask={}",
            pair,
            quote.last_price.map_or("—".to_string(), |v| format!("{v:.4}")),
            quote.bid.map_or("—".to_string(), |v| format!("{v:.4}")),
            quote.ask.map_or("—".to_string(), |v| format!("{v:.4}")),
        );

        Ok(quote)
    }
}
