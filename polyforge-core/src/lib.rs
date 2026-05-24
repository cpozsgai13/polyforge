//! `polyforge-core` — shared order book engine for the Polyforge platform.
//!
//! This crate is the single source of truth for all domain types used by every
//! other crate in the workspace:
//!
//! - [`price`]       — `FixedPrecisionPrice<T, N>`, `Price4/6/9` aliases
//! - [`price_trait`] — `FixedPrecisionPriceLike` trait
//! - [`types`]       — `OrderId`, `Side`, `OrderType`, `Volume`, etc.
//! - [`order`]       — `Order<P>`, `OrderPtr<P>`, `new_order_ptr`
//! - [`order_queue`] — price-level FIFO queue of orders
//! - [`order_book`]  — full L1/L2 limit order book with matching engine
//! - [`trade`]       — `Trade<P>`, `TradeSide<P>`
//! - [`messages`]    — wire DTOs: `AddOrderMsg`, `ModifyOrderMsg`, etc.
#![allow(dead_code)]

pub mod messages;
pub mod order;
pub mod order_book;
pub mod order_queue;
pub mod price;
pub mod price_trait;
pub mod trade;
pub mod types;
