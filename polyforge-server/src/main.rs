use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use polyforge_core::order_book::OrderBook;
use polyforge_core::messages::Price;

use std::{fs::File, io::{BufReader, Error, ErrorKind, Read}};
use actix_web::{web, App, HttpServer};
use tracing_actix_web::TracingLogger;
use actix_web_prom::PrometheusMetricsBuilder;
use sea_orm::{ConnectOptions, Database, DbErr};
use migration::MigratorTrait;
use utils::app_state::AppState;
use sea_orm::prelude::*;
use crate::quote_provider::{YahooQuoteProvider, KrakenQuoteProvider, RoutingQuoteProvider};
mod utils;
mod routes;
mod entities;
mod tests;
mod quote_provider;
use thiserror::Error;
use rustls::{pki_types::{CertificateDer, PrivateKeyDer}, ServerConfig};
use rustls_pemfile::certs;
use std::path::Path;
use tokio::sync::oneshot;

#[derive(Error, Debug)]
pub enum MainError {
    #[error("Database error: {0}")]
    Db(#[from] DbErr),
    //Db(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Custom: {0}")]
    Custom(String),
}

impl From<MainError> for std::io::Error {
    fn from(err: MainError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{err}"))
    }
}

// pub fn test() -> Result<(), std::io::Error> {
//     let f = File::open("key.pem")?;
//     let mut reader = BufReader::new(f);

//     let raw_keys = rsa_private_keys(&mut reader)
//         .map_err(|e| Error::new(ErrorKind::Other, format!("PEM parse error: {e}")))?;

//     println!("Found {} key(s)", raw_keys.len());
//     Ok(())    
// }

/// Loads TLS configuration from certificate and key file paths.
///
/// Both paths are resolved relative to the process working directory.
/// Returns a descriptive `io::Error` for every failure mode:
/// - file not found
/// - PEM parse error
/// - empty certificate chain
/// - no private key found
/// - rustls configuration error
pub fn load_tls_config<P: AsRef<Path>>(cert_path: P, key_path: P) -> Result<ServerConfig, std::io::Error> {
    let cert_path = cert_path.as_ref();
    let key_path  = key_path.as_ref();

    // ── Certificate chain ────────────────────────────────────────────────────
    let cert_file = File::open(cert_path)
        .map_err(|e| Error::new(ErrorKind::NotFound,
            format!("Cannot open cert file '{}': {e}", cert_path.display())))?;
    let mut cert_reader = BufReader::new(cert_file);

    let cert_chain: Vec<CertificateDer<'static>> = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| Error::new(ErrorKind::InvalidData,
            format!("Failed to parse certificate '{}': {e}", cert_path.display())))?;

    if cert_chain.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData,
            format!("No certificates found in '{}'", cert_path.display())));
    }

    // ── Private key ──────────────────────────────────────────────────────────
    let key_file = File::open(key_path)
        .map_err(|e| Error::new(ErrorKind::NotFound,
            format!("Cannot open key file '{}': {e}", key_path.display())))?;
    let mut key_reader = BufReader::new(key_file);

    let private_key: PrivateKeyDer = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| Error::new(ErrorKind::InvalidData,
            format!("Failed to parse private key '{}': {e}", key_path.display())))?        
        .ok_or_else(|| Error::new(ErrorKind::InvalidData,
            format!("No private key found in '{}'", key_path.display())))?;

    // ── Build rustls ServerConfig ─────────────────────────────────────────────
    ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|e| Error::new(ErrorKind::InvalidData,
            format!("TLS config error (cert='{}', key='{}'): {e}",
                cert_path.display(), key_path.display())))
}

/// Load `symbol,last_price,best_bid,best_ask` rows from a CSV file, seed an
/// `OrderBook` for each symbol, and populate `market_prices` so the fallback
/// bid/ask is available from the very first request.
/// Lines that are blank or start with `#` are skipped.  Malformed lines print
/// a warning and are skipped.
///
/// Returns the number of symbols successfully loaded.
fn load_last_prices(
    path: &Path,
    books: &mut HashMap<String, OrderBook<Price>>,
    market_prices: &mut HashMap<String, crate::utils::app_state::MarketPrice>,
) -> std::io::Result<usize> {
    use std::io::{BufRead, BufReader};
    let file = File::open(path)
        .map_err(|e| Error::new(ErrorKind::NotFound,
            format!("Cannot open last_price file '{}': {e}", path.display())))?;
    let mut count = 0;
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(4, ',');
        let symbol = match parts.next() {
            Some(s) => s.trim().to_uppercase(),
            None => { eprintln!("last_price.csv line {}: no symbol", line_no + 1); continue; }
        };
        let last_price: f64 = match parts.next().and_then(|s| s.trim().parse().ok()) {
            Some(p) => p,
            None => {
                eprintln!("last_price.csv line {}: cannot parse last_price for '{symbol}'", line_no + 1);
                continue;
            }
        };
        // best_bid and best_ask are optional (older CSV files only have 2 cols).
        let best_bid: Option<f64> = parts.next().and_then(|s| s.trim().parse().ok());
        let best_ask: Option<f64> = parts.next().and_then(|s| s.trim().parse().ok());

        books.entry(symbol.clone()).or_insert_with(|| OrderBook::new(&symbol));
        market_prices.insert(symbol.clone(), crate::utils::app_state::MarketPrice {
            bid:        best_bid.filter(|&v| v > 0.0),
            ask:        best_ask.filter(|&v| v > 0.0),
            last_price: Some(last_price).filter(|&v| v > 0.0),
        });
        println!("  Seeded {symbol} (last={last_price:.4}, bid={}, ask={})",
            best_bid.map_or("—".to_string(), |v| format!("{v:.4}")),
            best_ask.map_or("—".to_string(), |v| format!("{v:.4}")));
        count += 1;
    }
    Ok(count)
}

/// Fetch a live quote for every symbol in `books`, then write a CSV to `path`
/// with columns: SYMBOL, LAST_PRICE, BEST_BID, BEST_ASK.
/// Each column defaults to `0.0000` when the value is unavailable.
async fn save_last_prices(
    path: &Path,
    books: &Arc<Mutex<HashMap<String, OrderBook<Price>>>>,
    quote_provider: &Arc<dyn quote_provider::QuoteProvider>,
    market_prices: &Arc<Mutex<HashMap<String, crate::utils::app_state::MarketPrice>>>,
) -> std::io::Result<()> {
    use std::io::Write;

    // Collect symbols + current best bid/ask under a short-lived lock.
    // Skip crypto symbols (those containing '-') — they never close so there
    // is no meaningful "last session" price to persist; the book will be
    // re-seeded from a fresh Binance quote on the next startup fetch.
    let entries: Vec<(String, f64, f64)> = {
        let books = books.lock().unwrap();
        books.iter()
            .filter(|(sym, _)| !sym.contains('-'))
            .map(|(sym, book)| {
            let bid = book.best_bid_with_volume().map(|(p, _)| p.to_f64()).unwrap_or(0.0);
            let ask = book.best_ask_with_volume().map(|(p, _)| p.to_f64()).unwrap_or(0.0);
            (sym.clone(), bid, ask)
        }).collect()
    };

    // Fetch live last prices concurrently (lock is released above).
    let mut rows: Vec<(String, f64, f64, f64)> = Vec::with_capacity(entries.len());
    for (symbol, bid, ask) in entries {
        let last_price = match quote_provider.fetch_quote(&symbol).await {
            Ok(q)  => q.last_price.unwrap_or(0.0),
            Err(e) => {
                // Fall back to the most recent cached price for this symbol.
                let cached = market_prices
                    .lock()
                    .ok()
                    .and_then(|prices| prices.get(&symbol).and_then(|p| p.last_price))
                    .unwrap_or(0.0);
                if cached > 0.0 {
                    eprintln!("Warning: live quote for {symbol} unavailable at shutdown ({e}); using cached last price {cached:.4}");
                } else {
                    eprintln!("Warning: live quote for {symbol} unavailable at shutdown ({e}); no cache available");
                }
                cached
            }
        };
        rows.push((symbol, last_price, bid, ask));
    }

    let mut file = std::fs::File::create(path)
        .map_err(|e| Error::new(ErrorKind::Other,
            format!("Cannot create '{}': {e}", path.display())))?;
    for (symbol, last_price, bid, ask) in &rows {
        writeln!(file, "{symbol}, {last_price:.4}, {bid:.4}, {ask:.4}")?;
    }
    println!("Saved {} symbol(s) to '{}'", rows.len(), path.display());
    Ok(())
}

fn start_key_listener(tx: oneshot::Sender<()>) {
    use std::io::IsTerminal;
    std::thread::spawn(move || {
        if std::io::stdin().is_terminal() {
            println!("Press Enter to shut down...");
            let _ = std::io::stdin().read(&mut [0u8]);
            let _ = tx.send(());
        } else {
            // Non-TTY (Docker, CI): keep tx alive so the server runs until
            // the process receives SIGTERM / SIGKILL from the container runtime.
            std::thread::park();
        }
    });
}

/// Fetch live quotes from Binance for each crypto symbol concurrently and
/// seed both `order_books` (so the book entry exists) and `market_prices`
/// (so `best_quote` returns a meaningful fallback immediately on startup).
///
/// Errors for individual symbols are logged but do not abort startup.
async fn seed_crypto_symbols(
    symbols: &[String],
    provider: &Arc<dyn quote_provider::QuoteProvider>,
    order_books: &Arc<Mutex<HashMap<String, OrderBook<Price>>>>,
    market_prices: &Arc<Mutex<HashMap<String, crate::utils::app_state::MarketPrice>>>,
) {
    if symbols.is_empty() {
        return;
    }
    println!("Seeding {} crypto symbol(s) from Binance...", symbols.len());

    // Fetch all quotes concurrently via tokio tasks.
    let mut set = tokio::task::JoinSet::new();
    for sym in symbols {
        let sym = sym.clone();
        let prov = provider.clone();
        set.spawn(async move { (sym.clone(), prov.fetch_quote(&sym).await) });
    }

    let mut books  = order_books.lock().unwrap();
    let mut prices = market_prices.lock().unwrap();

    while let Some(res) = set.join_next().await {
        let (sym, result) = res.unwrap_or_else(|e| panic!("seed task panicked: {e}"));
        match result {
            Ok(q) => {
                books.entry(sym.clone()).or_insert_with(|| OrderBook::<Price>::new(&sym));
                prices.insert(sym.clone(), crate::utils::app_state::MarketPrice {
                    bid:        q.bid,
                    ask:        q.ask,
                    last_price: q.last_price,
                });
                println!("  Seeded {sym} (last={}, bid={}, ask={})",
                    q.last_price.map_or("—".into(), |v| format!("{v:.4}")),
                    q.bid.map_or("—".into(), |v| format!("{v:.4}")),
                    q.ask.map_or("—".into(), |v| format!("{v:.4}")));
            }
            Err(e) => eprintln!("  Warning: could not seed {sym}: {e}"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_none() {
        unsafe { std::env::set_var("RUST_LOG", "actix_web=info") };
    }

    // Load .env from CWD first, then try workspace root (when launched from
    // the package directory rather than workspace root).
    dotenv::dotenv().ok();
    let workspace_env = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env");
    let _ = dotenv::from_path(&workspace_env);

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(
                    "actix_web=info,polyforge_server=info,tracing_actix_web=info"
                ))
        )
        .init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Constants are lazy_static; access them AFTER dotenv has populated the
    // environment so they pick up the correct values.
    let port: u16 = (*utils::constants::PORT).clone();
    let address: String = (*utils::constants::ADDRESS).clone();
    let database_url: String = (*utils::constants::DATABASE_URL).clone();
    let cert_path: String = (*utils::constants::TLS_CERT_PATH).clone();
    let key_path: String = (*utils::constants::TLS_KEY_PATH).clone();

    // Resolve cert/key to absolute paths using the package directory baked in
    // at compile time so the server works regardless of the launch CWD.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cert_path = if Path::new(&cert_path).is_absolute() {
        cert_path
    } else {
        manifest_dir.join(&cert_path).to_string_lossy().into_owned()
    };
    let key_path = if Path::new(&key_path).is_absolute() {
        key_path
    } else {
        manifest_dir.join(&key_path).to_string_lossy().into_owned()
    };

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|e| format!("<error: {e}>"));
    println!("CWD: {cwd}");
    println!("Starting Polyforge Server on {}:{}, cert_path: {}, key_path: {}", address, port, cert_path, key_path);

    // Resolve cert/key paths relative to CWD so we can give a clear error
    // when the file doesn't exist before handing off to load_tls_config.
    for (label, path) in [("cert", &cert_path), ("key", &key_path)] {
        let p = Path::new(path);
        if p.exists() {
            println!("  [{label}] {path}  ✓  (abs: {})", p.canonicalize().unwrap_or(p.to_path_buf()).display());
        } else {
            println!("  [{label}] {path}  ✗  NOT FOUND relative to CWD={cwd}");
        }
    }
    println!();

    let config = load_tls_config(cert_path, key_path)?;

    println!("Got TLS config {:?}", config);

    let mut options = ConnectOptions::new(database_url);
    options.max_connections(1);
    options.min_connections(1);
    options.connect_timeout(Duration::from_secs(10));

    let db = Database::connect(options).await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("DB error: {}", e))
    })?;
        
    println!("Connected to database");

    // Apply any pending migrations automatically so the schema is always
    // up-to-date without a manual migration step.
    migration::Migrator::up(&db, None).await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("Migration error: {e}"))
    })?;
    println!("Migrations applied");

    let users = entities::users::Entity::find().all(&db).await.map_err(|err| MainError::Db(err))?;

    for user in users {
       println!("Loaded User: {}", user.name);
    }
    
    let mut books: HashMap<String, OrderBook<Price>> = HashMap::new();
    let mut market_prices_seed: HashMap<String, crate::utils::app_state::MarketPrice> = HashMap::new();

    // Seed order books from last_price.csv (or LAST_PRICE_FILE env var).
    let last_price_path_str = (*utils::constants::LAST_PRICE_FILE).clone();
    let last_price_path = Path::new(&last_price_path_str);
    // Resolve relative paths against workspace root (parent of CARGO_MANIFEST_DIR).
    let last_price_path_buf;  
    let last_price_path = if last_price_path.is_absolute() {
        last_price_path
    } else {
        last_price_path_buf = manifest_dir.join("..").join(last_price_path);
        last_price_path_buf.as_path()
    };
    match load_last_prices(last_price_path, &mut books, &mut market_prices_seed) {
        Ok(n)  => println!("Loaded {n} symbol(s) from '{}'", last_price_path.display()),
        Err(e) => eprintln!("Warning: could not load last_price file: {e}"),
    }
    // Keep an owned copy for use in the shutdown handler.
    let last_price_path_owned = last_price_path.to_path_buf();

    let order_books = Arc::new(Mutex::new(books));

    let yahoo = Arc::new(
        YahooQuoteProvider::new().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?
    );
    let kraken = Arc::new(
        KrakenQuoteProvider::new().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?
    );
    let quote_provider: Arc<dyn quote_provider::QuoteProvider> =
        Arc::new(RoutingQuoteProvider::new(yahoo, kraken));
    println!("Quote provider: {}", quote_provider.provider_name());

    let market_prices: Arc<std::sync::Mutex<std::collections::HashMap<String, crate::utils::app_state::MarketPrice>>> =
        Arc::new(std::sync::Mutex::new(market_prices_seed));

    // Seed crypto order books and market prices from Binance before the server
    // starts accepting connections, so best_quote always has a live fallback.
    let crypto_symbols: Vec<String> = (*utils::constants::CRYPTO_SYMBOLS).clone();
    seed_crypto_symbols(&crypto_symbols, &quote_provider, &order_books, &market_prices).await;

    // ── Price-alert broadcaster ───────────────────────────────────────────────
    // Capacity 64: we won't fire more than a handful of alerts simultaneously.
    let (alert_tx, _) = tokio::sync::broadcast::channel::<utils::app_state::AlertEvent>(64);
    let alert_tx = Arc::new(alert_tx);

    // ── Background alert checker ─────────────────────────────────────────────
    // Polls market_prices every 5 seconds, checks all active DB alerts,
    // broadcasts a fired event and marks the row triggered.
    {
        let db_check        = db.clone();
        let prices_check    = market_prices.clone();
        let tx_check        = alert_tx.clone();
        tokio::spawn(async move {
            use crate::entities::price_alert::{self, Entity as AlertEntity};
            use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let active = match AlertEntity::find()
                    .filter(price_alert::Column::Triggered.eq(false))
                    .all(&db_check)
                    .await
                {
                    Ok(rows) => rows,
                    Err(e) => { tracing::warn!(error = %e, "alert_checker_db_error"); continue; }
                };

                if active.is_empty() { continue; }

                let prices_snap: std::collections::HashMap<String, f64> = {
                    let p = prices_check.lock().unwrap();
                    p.iter()
                        .filter_map(|(sym, mp)| mp.last_price.map(|v| (sym.clone(), v)))
                        .collect()
                };

                for alert in active {
                    let price = match prices_snap.get(&alert.symbol) {
                        Some(&p) => p,
                        None => continue,
                    };

                    let fired = match alert.direction.as_str() {
                        "above" => price >= alert.threshold,
                        "below" => price <= alert.threshold,
                        _ => false,
                    };

                    if fired {
                        tracing::info!(alert_id = alert.id, symbol = %alert.symbol, direction = %alert.direction, threshold = alert.threshold, price, "alert_fired");

                        // Mark triggered in DB.
                        let fired_at = chrono::Utc::now().naive_utc();
                        let mut active_model: price_alert::ActiveModel = alert.clone().into();
                        active_model.triggered    = Set(true);
                        active_model.triggered_at = Set(Some(fired_at));
                        if let Err(e) = active_model.update(&db_check).await {
                            tracing::warn!(alert_id = alert.id, error = %e, "alert_mark_triggered_failed");
                        }

                        // Broadcast to SSE subscribers.
                        let event = utils::app_state::AlertEvent {
                            alert_id:  alert.id,
                            symbol:    alert.symbol.clone(),
                            threshold: alert.threshold,
                            direction: alert.direction.clone(),
                            label:     alert.label.clone(),
                            price,
                        };
                        // Ignore send errors — no subscribers is fine.
                        let _ = tx_check.send(event);
                    }
                }
            }
        });
    }

    let(tx, rx) = oneshot::channel();
    start_key_listener(tx);

    // Clone before the move closure so both the server workers and the
    // shutdown handler share the same underlying Arc.
    let save_books    = order_books.clone();
    let save_provider = quote_provider.clone();
    let save_prices   = market_prices.clone();

    // Build a shared registry so both actix-web-prom and our custom counters
    // are served from the same /metrics endpoint.
    let registry = prometheus::Registry::new();

    let symbol_requests = Arc::new(
        prometheus::CounterVec::new(
            prometheus::opts!(
                "polyforge_symbol_requests_total",
                "Request count broken down by symbol and operation"
            ),
            &["symbol", "operation"],
        ).expect("Failed to create symbol_requests")
    );
    registry.register(Box::new((*symbol_requests).clone())).unwrap();

    let order_book_depth = Arc::new(
        prometheus::GaugeVec::new(
            prometheus::opts!("polyforge_order_book_depth",
                "Number of distinct resting price levels"),
            &["symbol", "side"],
        ).expect("Failed to create order_book_depth")
    );
    registry.register(Box::new((*order_book_depth).clone())).unwrap();

    let order_book_spread = Arc::new(
        prometheus::GaugeVec::new(
            prometheus::opts!("polyforge_order_book_spread",
                "Bid-ask spread: best ask minus best bid"),
            &["symbol"],
        ).expect("Failed to create order_book_spread")
    );
    registry.register(Box::new((*order_book_spread).clone())).unwrap();

    let order_book_volume = Arc::new(
        prometheus::GaugeVec::new(
            prometheus::opts!("polyforge_order_book_volume",
                "Total resting volume per side"),
            &["symbol", "side"],
        ).expect("Failed to create order_book_volume")
    );
    registry.register(Box::new((*order_book_volume).clone())).unwrap();

    let trades_total = Arc::new(
        prometheus::CounterVec::new(
            prometheus::opts!("polyforge_trades_total",
                "Cumulative matched trades per symbol"),
            &["symbol"],
        ).expect("Failed to create trades_total")
    );
    registry.register(Box::new((*trades_total).clone())).unwrap();

    let quote_errors_total = Arc::new(
        prometheus::CounterVec::new(
            prometheus::opts!("polyforge_quote_errors_total",
                "Failed live quote fetches per symbol"),
            &["symbol"],
        ).expect("Failed to create quote_errors_total")
    );
    registry.register(Box::new((*quote_errors_total).clone())).unwrap();

    let prometheus = PrometheusMetricsBuilder::new("polyforge")
        .endpoint("/metrics")
        .registry(registry)
        .build()
        .expect("Failed to build Prometheus metrics middleware");

    // ── Background: refresh order-book state gauges every 10 s ──────────────
    {
        let books_bg  = order_books.clone();
        let depth_bg  = order_book_depth.clone();
        let spread_bg = order_book_spread.clone();
        let volume_bg = order_book_volume.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                let books = books_bg.lock().unwrap();
                for (symbol, book) in books.iter() {
                    depth_bg.with_label_values(&[symbol, "bid"]).set(book.bid_depth() as f64);
                    depth_bg.with_label_values(&[symbol, "ask"]).set(book.ask_depth() as f64);
                    volume_bg.with_label_values(&[symbol, "bid"]).set(book.total_bid_volume() as f64);
                    volume_bg.with_label_values(&[symbol, "ask"]).set(book.total_ask_volume() as f64);
                    if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
                        spread_bg.with_label_values(&[symbol]).set(ask.to_f64() - bid.to_f64());
                    }
                }
            }
        });
    }

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: db.clone(),
                order_books: order_books.clone(),
                quote_provider: quote_provider.clone(),
                market_prices: market_prices.clone(),
                alert_tx: alert_tx.clone(),
                symbol_requests:    symbol_requests.clone(),
                order_book_depth:   order_book_depth.clone(),
                order_book_spread:  order_book_spread.clone(),
                order_book_volume:  order_book_volume.clone(),
                trades_total:       trades_total.clone(),
                quote_errors_total: quote_errors_total.clone(),
            }))
            .wrap(prometheus.clone())
            .wrap(TracingLogger::default())
            .configure(routes::home_routes::config)
            .configure(routes::order_book_routes::config)
            .configure(routes::market_routes::config)
            .configure(routes::alert_routes::config)
    })
            .bind_rustls_0_23(format!("{}:{}", address, port), config)?
    .run();

    let shutdown = server.handle();
    tokio::spawn(async move {
        rx.await.ok();
        println!("Shutting down — saving last prices...");
        if let Err(e) = save_last_prices(&last_price_path_owned, &save_books, &save_provider, &save_prices).await {
            eprintln!("Warning: failed to save last prices: {e}");
        }
        shutdown.stop(true).await;
    });

    let _ = server.await;
    Ok(())

}
