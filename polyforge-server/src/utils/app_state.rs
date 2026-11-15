use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sea_orm::DatabaseConnection;
use polyforge_core::order_book::OrderBook;
use polyforge_core::messages::Price;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use prometheus::{CounterVec, GaugeVec};

use crate::quote_provider::QuoteProvider;

/// Last-known market prices fetched from the live quote provider.
/// Keyed by uppercase symbol. Updated every time `GET /market/{symbol}/quote`
/// is called successfully. Used as a reference-price fallback in handlers that
/// need a mid-price when no resting orders exist.
#[derive(Debug, Clone, Default)]
pub struct MarketPrice {
    pub bid:        Option<f64>,
    pub ask:        Option<f64>,
    pub last_price: Option<f64>,
}

/// Payload broadcast over SSE when a price alert fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub alert_id:  i32,
    pub symbol:    String,
    pub threshold: f64,
    pub direction: String,
    pub label:     Option<String>,
    /// The price that crossed the threshold.
    pub price:     f64,
}

pub struct AppState {
    pub db: DatabaseConnection,
    /// Map of symbol → OrderBook, shared across all Actix worker threads.
    pub order_books: Arc<Mutex<HashMap<String, OrderBook<Price>>>>,
    /// Vendor-agnostic live quote provider (e.g. Yahoo Finance).
    /// Swap the concrete type in `main.rs` to switch data sources.
    pub quote_provider: Arc<dyn QuoteProvider>,
    /// Last prices fetched from the live quote provider, keyed by symbol.
    pub market_prices: Arc<Mutex<HashMap<String, MarketPrice>>>,
    /// Broadcast channel for fired price alerts — SSE subscribers receive events here.
    pub alert_tx: Arc<broadcast::Sender<AlertEvent>>,
    /// Per-symbol request counter: labels are [symbol, operation].
    pub symbol_requests:    Arc<CounterVec>,
    /// Number of resting price levels per symbol+side.
    pub order_book_depth:   Arc<GaugeVec>,
    /// Bid-ask spread (best ask − best bid) per symbol.
    pub order_book_spread:  Arc<GaugeVec>,
    /// Total resting volume per symbol+side.
    pub order_book_volume:  Arc<GaugeVec>,
    /// Cumulative matched trades per symbol.
    pub trades_total:       Arc<CounterVec>,
    /// Failed live quote fetches per symbol.
    pub quote_errors_total: Arc<CounterVec>,
}
