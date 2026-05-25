


use std::env;

use lazy_static::lazy_static;

lazy_static!{
    pub static ref ADDRESS: String = set_address();
    pub static ref DATABASE_URL: String = set_database_url();
    pub static ref SECRET: String = set_secret();
    pub static ref PORT: u16 = set_port();
    pub static ref MAX_FILE_SIZE: u64 = set_max_file_size();
    pub static ref TLS_CERT_PATH: String = set_cert_path();
    pub static ref TLS_KEY_PATH: String = set_key_path();
    pub static ref LAST_PRICE_FILE: String =
        env::var("LAST_PRICE_FILE").unwrap_or_else(|_| "last_price.csv".to_string());
    /// Comma-separated list of crypto symbols to seed at startup via Binance.
    /// E.g. `BTC-USD,ETH-USD,SOL-USD`
    pub static ref CRYPTO_SYMBOLS: Vec<String> = env::var("CRYPTO_SYMBOLS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
}

fn set_cert_path() -> String {
    dotenv::dotenv().ok();
    env::var("TLS_CERT_PATH").unwrap_or("certs/cert.pem".to_string())
}

fn set_key_path() -> String {
    dotenv::dotenv().ok();
    env::var("TLS_KEY_PATH").unwrap_or("certs/key.pem".to_string())
}

fn set_address() -> String {
    dotenv::dotenv().ok();
    env::var("ADDRESS").unwrap_or("127.0.0.1".to_string())
}

fn set_database_url() -> String {
    dotenv::dotenv().ok();
    env::var("DATABASE_URL").unwrap_or("sqlite://data.db?mode=rwc".to_string())
}

fn set_secret() -> String {
    dotenv::dotenv().ok();
    env::var("SECRET").unwrap_or("SECRET".to_string())
}


fn set_port() -> u16 {
    dotenv::dotenv().ok();
    env::var("PORT")
    .unwrap_or("5050".to_owned())
    .parse::<u16>()
    .expect("Can't parse the port")
}

fn set_max_file_size() -> u64 {
    dotenv::dotenv().ok();
    env::var("MAX_FILE_SIZE")
    .unwrap_or("10485760".to_owned())
    .parse::<u64>()
    .expect("Can't parse the max file size")
}