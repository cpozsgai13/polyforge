//! Actix-web handlers for order-book trading operations.
//!
//! Each handler operates on the per-symbol `OrderBook<Price>` stored in
//! `AppState::order_books`.  A symbol is created on first use.
//!
//! # Endpoints
//! | Method | Path                               | Description           |
//! |--------|------------------------------------|-----------------------|
//! | POST   | /trading/{symbol}/add_order        | Add a new order       |
//! | POST   | /trading/{symbol}/modify_order     | Modify an order       |
//! | DELETE | /trading/{symbol}/cancel_order     | Cancel an order       |
//! | GET    | /trading/{symbol}/best_bid         | Best bid quote        |
//! | GET    | /trading/{symbol}/best_ask         | Best ask quote        |
//! | GET    | /trading/{symbol}/best_quote       | Best bid + ask quote  |

use actix_web::{delete, get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use polyforge_core::messages::{ModifyOrderMsg, Price};
use polyforge_core::order::new_order_ptr;
use polyforge_core::order_book::OrderBook;
use polyforge_core::types::{OrderType, Side};

use crate::utils::app_state::AppState;

// ── Shared helper ─────────────────────────────────────────────────────────────

fn parse_side(s: &str) -> Option<Side> {
    match s.to_ascii_lowercase().as_str() {
        "bid" | "buy"  => Some(Side::Bid),
        "ask" | "sell" => Some(Side::Ask),
        _              => None,
    }
}

fn parse_order_type(s: &str) -> Option<OrderType> {
    match s.to_ascii_lowercase().as_str() {
        "gfd" | "good_for_day" => Some(OrderType::Gfd),
        "ioc" | "immediate_or_cancel" => Some(OrderType::Ioc),
        "market" => Some(OrderType::Market),
        _ => None,
    }
}

// ── Request / Response DTOs ───────────────────────────────────────────────────

/// Request body for adding a new order.
#[derive(Debug, Deserialize)]
pub struct AddOrderRequest {
    /// Unique order identifier (u64).
    pub order_id: u64,
    /// Price as a decimal number (e.g. 123.456789).
    pub price: f64,
    /// Number of units to trade.
    pub quantity: u64,
    /// `"bid"` / `"buy"` or `"ask"` / `"sell"`.
    pub side: String,
    /// `"gfd"`, `"ioc"`, or `"market"`.
    pub order_type: String,
    /// Instrument ID (defaults to 0 if omitted).
    #[serde(default)]
    pub instrument_id: u64,
    /// Timestamp in nanoseconds (defaults to 0 if omitted).
    #[serde(default)]
    pub timestamp_ns: u64,
}

/// Request body for modifying a resting order.
#[derive(Debug, Deserialize)]
pub struct ModifyOrderRequest {
    pub order_id: u64,
    pub price: f64,
    pub quantity: u64,
    pub side: String,
    #[serde(default)]
    pub instrument_id: u64,
    #[serde(default)]
    pub timestamp_ns: u64,
}

/// Request body for cancelling an order.
#[derive(Debug, Deserialize)]
pub struct CancelOrderRequest {
    pub order_id: u64,
    #[serde(default)]
    pub instrument_id: u64,
}

/// A price level quote (price + volume).
#[derive(Debug, Serialize)]
pub struct QuoteResponse {
    pub symbol: String,
    pub price: f64,
    pub volume: u64,
}

/// Generic acknowledgement response.
#[derive(Debug, Serialize)]
pub struct AckResponse {
    pub success: bool,
    pub message: String,
}

/// A single side of a top-of-book quote (price + volume).
#[derive(Debug, Serialize)]
pub struct QuoteLevel {
    pub price:  f64,
    pub volume: u64,
}

/// Response for the combined best-bid-and-ask query.
///
/// Either side may be `null` when there are no resting orders on that side.
#[derive(Debug, Serialize)]
pub struct BestQuoteResponse {
    pub symbol: String,
    pub bid:    Option<QuoteLevel>,
    pub ask:    Option<QuoteLevel>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// `POST /trading/{symbol}/add_order`
#[post("/add_order")]
pub async fn add_order(
    symbol: web::Path<String>,
    body: web::Json<AddOrderRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let side = match parse_side(&body.side) {
        Some(s) => s,
        None => return HttpResponse::BadRequest().json(AckResponse {
            success: false,
            message: format!("Unknown side '{}'", body.side),
        }),
    };

    let order_type = match parse_order_type(&body.order_type) {
        Some(t) => t,
        None => return HttpResponse::BadRequest().json(AckResponse {
            success: false,
            message: format!("Unknown order_type '{}'", body.order_type),
        }),
    };

    let price = Price::from(body.price);
    let order_ptr = new_order_ptr(
        order_type,
        side,
        body.order_id,
        price,
        body.quantity,
        body.timestamp_ns,
    );

    let mut books = state.order_books.lock().unwrap();
    let book = books
        .entry(symbol.clone())
        .or_insert_with(|| OrderBook::new(symbol.as_str()));

    let ok = book.add_order(order_ptr);

    if ok {
        HttpResponse::Ok().json(AckResponse {
            success: true,
            message: format!("Order {} added to book '{}'", body.order_id, symbol),
        })
    } else {
        HttpResponse::Conflict().json(AckResponse {
            success: false,
            message: format!(
                "Order {} could not be added (duplicate or unmatched IOC)",
                body.order_id
            ),
        })
    }
}

/// `POST /trading/{symbol}/modify_order`
#[post("/modify_order")]
pub async fn modify_order(
    symbol: web::Path<String>,
    body: web::Json<ModifyOrderRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let side = match parse_side(&body.side) {
        Some(s) => s,
        None => return HttpResponse::BadRequest().json(AckResponse {
            success: false,
            message: format!("Unknown side '{}'", body.side),
        }),
    };

    let msg = ModifyOrderMsg {
        order_id:       body.order_id,
        instrument_id:  body.instrument_id,
        price:          Price::from(body.price),
        side,
        quantity:       body.quantity,
        update_time_ns: body.timestamp_ns,
    };

    let mut books: std::sync::MutexGuard<std::collections::HashMap<String, OrderBook<Price>>> = state.order_books.lock().unwrap();
    match books.get_mut(symbol.as_str()) {
        None => HttpResponse::NotFound().json(AckResponse {
            success: false,
            message: format!("Symbol '{}' not found", symbol),
        }),
        Some(book) => {
            let ok = book.update_order(&msg);
            if ok {
                HttpResponse::Ok().json(AckResponse {
                    success: true,
                    message: format!("Order {} modified in '{}'", body.order_id, symbol),
                })
            } else {
                HttpResponse::NotFound().json(AckResponse {
                    success: false,
                    message: format!("Order {} not found in '{}'", body.order_id, symbol),
                })
            }
        }
    }
}

/// `DELETE /trading/{symbol}/cancel_order`
#[delete("/cancel_order")]
pub async fn cancel_order(
    symbol: web::Path<String>,
    body: web::Json<CancelOrderRequest>,
    state: web::Data<AppState>,
) -> impl Responder {
    let mut books: std::sync::MutexGuard<std::collections::HashMap<String, OrderBook<Price>>> = state.order_books.lock().unwrap();
    match books.get_mut(symbol.as_str()) {
        None => HttpResponse::NotFound().json(AckResponse {
            success: false,
            message: format!("Symbol '{}' not found", symbol),
        }),
        Some(book) => {
            let ok = book.cancel_order(body.order_id);
            if ok {
                HttpResponse::Ok().json(AckResponse {
                    success: true,
                    message: format!("Order {} cancelled in '{}'", body.order_id, symbol),
                })
            } else {
                HttpResponse::NotFound().json(AckResponse {
                    success: false,
                    message: format!("Order {} not found in '{}'", body.order_id, symbol),
                })
            }
        }
    }
}

/// `GET /trading/{symbol}/best_bid`
#[get("/best_bid")]
pub async fn get_best_bid(
    symbol: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let books: std::sync::MutexGuard<std::collections::HashMap<String, OrderBook<Price>>> = state.order_books.lock().unwrap();
    match books.get(symbol.as_str()) {
        None => HttpResponse::NotFound().json(AckResponse {
            success: false,
            message: format!("Symbol '{}' not found", symbol),
        }),
        Some(book) => match book.best_bid_with_volume() {
            None => HttpResponse::Ok().json(AckResponse {
                success: true,
                message: format!("No resting bids in '{}'", symbol),
            }),
            Some((price, vol)) => HttpResponse::Ok().json(QuoteResponse {
                symbol: symbol.clone(),
                price:  price.to_f64(),
                volume: vol,
            }),
        },
    }
}

/// `GET /trading/{symbol}/best_ask`
#[get("/best_ask")]
pub async fn get_best_ask(
    symbol: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let books: std::sync::MutexGuard<std::collections::HashMap<String, OrderBook<Price>>> = state.order_books.lock().unwrap();
    match books.get(symbol.as_str()) {
        None => HttpResponse::NotFound().json(AckResponse {
            success: false,
            message: format!("Symbol '{}' not found", symbol),
        }),
        Some(book) => match book.best_ask_with_volume() {
            None => HttpResponse::Ok().json(AckResponse {
                success: true,
                message: format!("No resting asks in '{}'", symbol),
            }),
            Some((price, vol)) => HttpResponse::Ok().json(QuoteResponse {
                symbol: symbol.clone(),
                price:  price.to_f64(),
                volume: vol,
            }),
        },
    }
}

/// `GET /trading/{symbol}/best_quote`
///
/// Returns the best bid and best ask for a symbol in a single response.
/// Either side will be `null` when no resting orders exist on that side.
/// Returns 404 when the symbol has never been seen by the server.
#[get("/best_quote")]
pub async fn get_best_quote(
    symbol: web::Path<String>,
    state: web::Data<AppState>,
) -> impl Responder {
    let books: std::sync::MutexGuard<std::collections::HashMap<String, OrderBook<Price>>> =
        state.order_books.lock().unwrap();

    match books.get(symbol.as_str()) {
        None => HttpResponse::Ok().json(BestQuoteResponse {
            symbol: symbol.clone(),
            bid:    None,
            ask:    None,
        }),
        Some(book) => {
            let mut bid = book
                .best_bid_with_volume()
                .map(|(price, vol)| QuoteLevel { price: price.to_f64(), volume: vol });

            let mut ask = book
                .best_ask_with_volume()
                .map(|(price, vol)| QuoteLevel { price: price.to_f64(), volume: vol });

            // Fall back to the last live market price when one or both sides
            // of the order book are empty.
            if bid.is_none() || ask.is_none() {
                if let Ok(prices) = state.market_prices.lock() {
                    if let Some(mp) = prices.get(symbol.as_str()) {
                        if bid.is_none() {
                            bid = mp.bid.map(|p| QuoteLevel { price: p, volume: 0 });
                        }
                        if ask.is_none() {
                            ask = mp.ask.map(|p| QuoteLevel { price: p, volume: 0 });
                        }
                    }
                }
            }

            HttpResponse::Ok().json(BestQuoteResponse {
                symbol: symbol.clone(),
                bid,
                ask,
            })
        }
    }
}
