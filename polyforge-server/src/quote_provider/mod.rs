//! Vendor-agnostic live-quote provider interface.
//!
//! Add a new data source by implementing [`QuoteProvider`] and injecting it
//! into [`AppState`](crate::utils::app_state::AppState).  The server is
//! completely decoupled from any specific vendor.
//!
//! # Provided implementations
//! | Type                  | Feature / crate           | Description                    |
//! |-----------------------|---------------------------|--------------------------------|
//! | [`YahooQuoteProvider`]| `yahoo-finance-rs`        | Yahoo Finance `/v7/finance/quote` |
//!
//! # Switching vendors
//! Set `QUOTE_PROVIDER=yahoo` (default) in `.env`.  To add another vendor:
//! 1. Create `src/quote_provider/<vendor>.rs` implementing [`QuoteProvider`].
//! 2. Match on `QUOTE_PROVIDER` in `main.rs` and inject the new struct.

use async_trait::async_trait;
use serde::Serialize;
use std::fmt;

pub mod yahoo;
pub mod binance;
pub mod kraken;
pub mod router;

pub use yahoo::YahooQuoteProvider;
pub use kraken::KrakenQuoteProvider;
pub use router::RoutingQuoteProvider;

// ── Output type ───────────────────────────────────────────────────────────────

/// Normalized live quote returned by any vendor.
///
/// All fields except `symbol` are `Option` because some vendors / instruments
/// do not supply every field (e.g. crypto has no bid/ask, closed markets have
/// no last price).
#[derive(Debug, Clone, Serialize)]
pub struct LiveQuote {
    /// Instrument symbol (uppercased, e.g. `"AAPL"`).
    pub symbol:       String,
    /// Best bid price.
    pub bid:          Option<f64>,
    /// Best ask price.
    pub ask:          Option<f64>,
    /// Most recent traded price.
    pub last_price:   Option<f64>,
    /// ISO currency code (e.g. `"USD"`).
    pub currency:     Option<String>,
    /// Market session state (`"REGULAR"`, `"PRE"`, `"POST"`, `"CLOSED"`, …).
    pub market_state: Option<String>,
}

// ── Error type ────────────────────────────────────────────────────────────────

/// An opaque error from a quote provider (network failure, unknown symbol, …).
#[derive(Debug)]
pub struct QuoteProviderError(pub String);

impl fmt::Display for QuoteProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for QuoteProviderError {}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Vendor-agnostic interface for fetching live market quotes.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// Actix worker threads via `Arc<dyn QuoteProvider>`.
#[async_trait]
pub trait QuoteProvider: Send + Sync {
    /// Fetch a live quote for a single `symbol`.
    ///
    /// Returns `Err` on network failure or when the symbol is not recognised
    /// by the upstream provider.
    async fn fetch_quote(&self, symbol: &str) -> Result<LiveQuote, QuoteProviderError>;

    /// Provider name for logging / diagnostics (e.g. `"yahoo"`).
    fn provider_name(&self) -> &'static str;
}
