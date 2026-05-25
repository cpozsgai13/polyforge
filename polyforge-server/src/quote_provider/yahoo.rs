//! Yahoo Finance quote provider.
//!
//! Uses [`yahoo-finance-rs`](https://crates.io/crates/yahoo-finance-rs) —
//! `Ticker::quote()` hits Yahoo's `/v7/finance/quote` endpoint and returns a
//! rich snapshot including bid/ask, market state, and last price.
//!
//! [`YfClient`] is cheap to clone (Arc-backed) and serialises crumb refreshes
//! through a single `tokio::Mutex`, so a single shared instance is efficient.

use async_trait::async_trait;
use yfinance::{Ticker, YfClient};

use super::{LiveQuote, QuoteProvider, QuoteProviderError};

// ── Provider struct ───────────────────────────────────────────────────────────

/// Quote provider backed by Yahoo Finance.
///
/// Construct with [`YahooQuoteProvider::new`] or
/// [`YahooQuoteProvider::with_client`] if you need custom session cookies /
/// crumb injection.
#[derive(Debug, Clone)]
pub struct YahooQuoteProvider {
    client: YfClient,
}

impl YahooQuoteProvider {
    /// Build a provider using Yahoo's default authentication flow.
    pub fn new() -> Result<Self, QuoteProviderError> {
        YfClient::new()
            .map(|client| Self { client })
            .map_err(|e| QuoteProviderError(format!("YfClient init failed: {e}")))
    }

    /// Build a provider from an existing [`YfClient`].
    /// Use this when you need to inject browser-session cookies or crumbs.
    pub fn with_client(client: YfClient) -> Self {
        Self { client }
    }
}

// ── QuoteProvider impl ────────────────────────────────────────────────────────
#[async_trait]
impl QuoteProvider for YahooQuoteProvider {
    fn provider_name(&self) -> &'static str {
        "yahoo"
    }

    /// Fetch a live quote from Yahoo Finance.
    ///
    /// Maps `yfinance::Quote` → [`LiveQuote`], picking out the fields that
    /// are part of the vendor-neutral contract.
    async fn fetch_quote(&self, symbol: &str) -> Result<LiveQuote, QuoteProviderError> {
        let ticker = Ticker::new(&self.client, symbol);

        let q = ticker
            .quote()
            .await
            .map_err(|e| QuoteProviderError(format!("Yahoo quote error for '{symbol}': {e}")))?;

        Ok(LiveQuote {
            symbol:       q.symbol,
            bid:          q.bid,
            ask:          q.ask,
            last_price:   q.price,
            currency:     q.currency,
            market_state: q.market_state,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ── Stub provider ─────────────────────────────────────────────────────────

    /// A fake [`QuoteProvider`] that returns pre-canned responses without
    /// hitting the network.  Used to verify the trait contract and downstream
    /// consumers independently of Yahoo Finance.
    struct StubQuoteProvider {
        /// `Some` → return this quote (symbol field overridden with the
        /// requested symbol).  `None` → return an error.
        canned: Option<LiveQuote>,
    }

    impl StubQuoteProvider {
        fn ok(quote: LiveQuote) -> Arc<dyn QuoteProvider> {
            Arc::new(Self { canned: Some(quote) })
        }

        fn err() -> Arc<dyn QuoteProvider> {
            Arc::new(Self { canned: None })
        }
    }

    #[async_trait]
    impl QuoteProvider for StubQuoteProvider {
        fn provider_name(&self) -> &'static str {
            "stub"
        }

        async fn fetch_quote(&self, symbol: &str) -> Result<LiveQuote, QuoteProviderError> {
            match &self.canned {
                Some(q) => Ok(LiveQuote { symbol: symbol.to_string(), ..q.clone() }),
                None    => Err(QuoteProviderError(format!("stub error for '{symbol}'"))),
            }
        }
    }

    fn sample_quote(symbol: &str) -> LiveQuote {
        LiveQuote {
            symbol:       symbol.to_string(),
            bid:          Some(189.50),
            ask:          Some(189.55),
            last_price:   Some(189.52),
            currency:     Some("USD".to_string()),
            market_state: Some("REGULAR".to_string()),
        }
    }

    // ── YahooQuoteProvider ────────────────────────────────────────────────────

    /// `provider_name` must return `"yahoo"` for routing / logging.
    /// `YfClient::new()` only builds an HTTP client — no network request.
    #[test]
    fn yahoo_provider_name_is_yahoo() {
        match YfClient::new() {
            Ok(client) => {
                let provider = YahooQuoteProvider::with_client(client);
                assert_eq!(provider.provider_name(), "yahoo");
            }
            Err(e) => eprintln!("Skipping: YfClient init failed: {e}"),
        }
    }

    // ── StubQuoteProvider ─────────────────────────────────────────────────────

    #[test]
    fn stub_provider_name() {
        let p = StubQuoteProvider::ok(sample_quote("AAPL"));
        assert_eq!(p.provider_name(), "stub");
    }

    #[tokio::test]
    async fn stub_returns_ok_quote() {
        let provider = StubQuoteProvider::ok(sample_quote("AAPL"));
        let q = provider.fetch_quote("AAPL").await.expect("expected Ok quote");
        assert_eq!(q.symbol,                    "AAPL");
        assert_eq!(q.bid,                       Some(189.50));
        assert_eq!(q.ask,                       Some(189.55));
        assert_eq!(q.last_price,                Some(189.52));
        assert_eq!(q.currency.as_deref(),       Some("USD"));
        assert_eq!(q.market_state.as_deref(),   Some("REGULAR"));
    }

    /// The stub must echo back the *requested* symbol, not the canned one.
    #[tokio::test]
    async fn stub_overrides_symbol_with_requested_symbol() {
        let provider = StubQuoteProvider::ok(sample_quote("AAPL"));
        let q = provider.fetch_quote("MSFT").await.expect("expected Ok");
        assert_eq!(q.symbol, "MSFT");
    }

    #[tokio::test]
    async fn stub_propagates_error() {
        let result = StubQuoteProvider::err().fetch_quote("AAPL").await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("AAPL"), "error message should contain symbol; got: {msg}");
    }

    /// `Arc<dyn QuoteProvider>` must be `Send + Sync` for sharing across Actix
    /// worker threads via `AppState`.
    #[test]
    fn quote_provider_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn QuoteProvider>>();
    }

    // ── QuoteProviderError ────────────────────────────────────────────────────

    #[test]
    fn error_display_contains_message() {
        let err = QuoteProviderError("network timeout".to_string());
        assert_eq!(err.to_string(), "network timeout");
    }

    #[test]
    fn error_implements_std_error() {
        fn assert_std_error<E: std::error::Error>() {}
        assert_std_error::<QuoteProviderError>();
    }

    // ── LiveQuote ─────────────────────────────────────────────────────────────

    #[test]
    fn live_quote_all_none_fields_are_valid() {
        let q = LiveQuote {
            symbol: "TEST".to_string(),
            bid: None, ask: None, last_price: None,
            currency: None, market_state: None,
        };
        assert_eq!(q.symbol, "TEST");
        assert!(q.bid.is_none() && q.ask.is_none());
    }

    #[test]
    fn live_quote_serializes_to_json() {
        let q = sample_quote("AAPL");
        let json = serde_json::to_string(&q).expect("serialization failed");
        assert!(json.contains("\"symbol\":\"AAPL\""),        "symbol missing");
        assert!(json.contains("\"bid\":189.5"),              "bid missing");
        assert!(json.contains("\"ask\":189.55"),             "ask missing");
        assert!(json.contains("\"currency\":\"USD\""),       "currency missing");
        assert!(json.contains("\"market_state\":\"REGULAR\""), "market_state missing");
    }

    #[test]
    fn live_quote_serializes_none_fields_as_null() {
        let q = LiveQuote {
            symbol: "X".to_string(),
            bid: None, ask: None, last_price: None,
            currency: None, market_state: None,
        };
        let json = serde_json::to_string(&q).expect("serialization failed");
        assert!(json.contains("\"bid\":null"),  "bid should be null");
        assert!(json.contains("\"ask\":null"),  "ask should be null");
    }

    // ── Live integration test ─────────────────────────────────────────────────

    /// Fetches a real quote from Yahoo Finance.  Skipped in CI — run with:
    /// ```text
    /// cargo test -p polyforge-server -- --ignored yahoo_live_fetch_aapl
    /// ```
    #[tokio::test]
    #[ignore = "requires network access and a valid Yahoo Finance session"]
    async fn yahoo_live_fetch_aapl() {
        let provider = YahooQuoteProvider::new().expect("YfClient should initialise");
        let q = provider.fetch_quote("AAPL").await.expect("live quote should succeed");
        assert_eq!(q.symbol, "AAPL");
        assert!(q.ask.is_some_and(|v| v > 0.0), "ask should be a positive number");
    }
}
