//! Routing quote provider.
//!
//! Dispatches `fetch_quote` to the appropriate backend based on the symbol:
//!
//! | Symbol pattern          | Backend  |
//! |-------------------------|----------|
//! | Contains a `-` (e.g. `BTC-USD`, `ETH-USDT`) | Binance  |
//! | Anything else (e.g. `AAPL`, `MSFT`)          | Yahoo    |

use async_trait::async_trait;
use std::sync::Arc;

use super::{LiveQuote, QuoteProvider, QuoteProviderError};

/// A [`QuoteProvider`] that routes crypto symbols to one provider and
/// equity/index symbols to another.
pub struct RoutingQuoteProvider {
    /// Used for symbols that contain a `-` (crypto pairs like `BTC-USD`).
    pub crypto:  Arc<dyn QuoteProvider>,
    /// Used for all other symbols (equities, ETFs, indices).
    pub equity:  Arc<dyn QuoteProvider>,
}

impl RoutingQuoteProvider {
    pub fn new(equity: Arc<dyn QuoteProvider>, crypto: Arc<dyn QuoteProvider>) -> Self {
        Self { equity, crypto }
    }

    fn is_crypto(symbol: &str) -> bool {
        symbol.contains('-')
    }
}

#[async_trait]
impl QuoteProvider for RoutingQuoteProvider {
    fn provider_name(&self) -> &'static str {
        "router(yahoo+binance)"
    }

    async fn fetch_quote(&self, symbol: &str) -> Result<LiveQuote, QuoteProviderError> {
        if Self::is_crypto(symbol) {
            self.crypto.fetch_quote(symbol).await
        } else {
            self.equity.fetch_quote(symbol).await
        }
    }
}
