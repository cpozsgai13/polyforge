//! Actix-web handler for live market data.
//!
//! # Endpoints
//! | Method | Path                    | Description                              |
//! |--------|-------------------------|------------------------------------------|
//! | GET    | /market/{symbol}/quote  | Live bid/ask/last from the quote vendor  |

use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;

use crate::utils::app_state::{AppState, MarketPrice};

// ── Response DTOs ─────────────────────────────────────────────────────────────

/// Error body returned when the upstream vendor fails.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    success: bool,
    message: String,
    provider: String,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// `GET /market/{symbol}/quote`
///
/// Fetches a live quote for `{symbol}` from the configured quote provider
/// (default: Yahoo Finance).  The response is vendor-neutral:
///
/// ```json
/// {
///   "symbol": "AAPL",
///   "bid": 194.50,
///   "ask": 194.55,
///   "last_price": 194.52,
///   "currency": "USD",
///   "market_state": "REGULAR"
/// }
/// ```
///
/// Either `bid` or `ask` may be `null` when the market is closed or the
/// instrument does not have a visible order book (e.g. some indices).
///
/// Returns HTTP 502 if the upstream provider returns an error.
#[get("/quote")]
pub async fn get_live_quote(
    symbol: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let sym = symbol.into_inner().to_uppercase();

    match state.quote_provider.fetch_quote(&sym).await {
        Ok(quote) => {
            state.symbol_requests.with_label_values(&[&sym, "market_quote"]).inc();
            tracing::info!(symbol = %sym, last_price = ?quote.last_price, "market_quote_fetched");
            // Update the in-memory market price so other handlers (e.g.
            // get_best_quote) can use it as a reference-price fallback.
            if let Ok(mut prices) = state.market_prices.lock() {
                prices.insert(sym.clone(), MarketPrice {
                    bid:        quote.bid,
                    ask:        quote.ask,
                    last_price: quote.last_price,
                });
            }
            HttpResponse::Ok().json(quote)
        }
        Err(e) => {
            state.quote_errors_total.with_label_values(&[&sym]).inc();
            tracing::warn!(symbol = %sym, error = %e, "quote_fetch_failed");
            HttpResponse::BadGateway().json(ErrorResponse {
                success:  false,
                message:  e.to_string(),
                provider: state.quote_provider.provider_name().to_string(),
            })
        }
    }
}
