# polyforge-core

Rust library: order book engine and shared domain types. No async, no I/O.

- `OrderBook<P>` ([src/order_book.rs](src/order_book.rs)) is generic over `P: FixedPrecisionPriceLike` ([src/price_trait.rs](src/price_trait.rs)). Prices are `FixedPrecisionPrice<u64, PLACES>` ([src/price.rs](src/price.rs)) with aliases `Price4/6/9`; `messages::Price = Price6` is the default. Conversions between precisions are explicit `From` impls.
- Bids/asks are `BTreeMap<P, OrderQueue>`: best bid = `keys().next_back()`, best ask = `keys().next()`. Per-level volume lives in separate `HashMap<P, Volume>`; keep it in sync when adding matching/cancel paths.
- Orders are `Arc<Mutex<Order>>` (`OrderPtr`, built via `new_order_ptr`). `OrderType`: `Gfd`, `Ioc`, `Market`.
- `add_order`: IOC never rests (returns false if no immediate match); GFD rests then sweeps via `match_orders`. `update_order` = cancel + re-add (loses time priority).
- `add_order` currently `println!`s the book after each order; be aware when adding perf-sensitive code or tests.
- Message DTOs (`AddOrderMsg<P>`, `Packet<P>`, …) live in [src/messages.rs](src/messages.rs); wire encoding is NOT here (see pipeline).
- Keep C++-mirroring names and doc comments referencing the original headers.
- Tests: `cargo test -p polyforge-core`.
