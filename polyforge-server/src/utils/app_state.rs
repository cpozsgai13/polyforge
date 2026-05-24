use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sea_orm::DatabaseConnection;
use polyforge_core::order_book::OrderBook;
use polyforge_core::messages::Price;

pub struct AppState {
    pub db: DatabaseConnection,
    /// Map of symbol → OrderBook, shared across all Actix worker threads.
    pub order_books: Arc<Mutex<HashMap<String, OrderBook<Price>>>>,
}