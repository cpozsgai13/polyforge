use std::time::Duration;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use std::{fs::File, io::{BufReader, Error, ErrorKind, Read}};
use actix_web::{middleware::Logger, web, App, HttpServer};
use sea_orm::{ConnectOptions, Database, DbErr};
use utils::app_state::AppState;
use sea_orm::prelude::*;
mod utils;
mod routes;
mod entities;
mod tests;
use thiserror::Error;
use axum::{routing::get, Router};
use rustls_pki_types::pem::PemObject;
use rustls::{pki_types::{CertificateDer, PrivateKeyDer}, ServerConfig};
use rustls_pemfile::{certs, rsa_private_keys};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use std::fs;
use std::path::Path;
use tokio::sync::oneshot;

use crate::tests::tests::load_key;
// use rustls::ServerConfig;
// use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, PrivateKeyDer};

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

/// Loads TLS configuration from certificate and key file paths
pub fn load_tls_config<P: AsRef<Path>>(cert_path: P, key_path: P) -> Result<ServerConfig, std::io::Error> {
    // Read and wrap certificate chain

    let cert_bytes = fs::read(&cert_path)?;
    
    let cert_chain: Vec<CertificateDer> = CertificateDer::pem_slice_iter(&cert_bytes)
    .filter_map(Result::ok)
    .collect();

    //let cert_chain: Vec<CertificateDer> = CertificateDer::pem_slice_iter(&cert_bytes).collect();

    // let cert_chain: Vec<CertificateDer> = pem::parse_many(&cert_bytes)
    //     .filter_map(|r| match r {
    //         Ok(pem) if pem.tag == "CERTIFICATE" => Some(CertificateDer::from(pem.contents)),
    //         _ => None,
    //     })
    //     .collect();    
    // let cert_bytes = fs::read(&cert_path)
    //     .map_err(|e| Error::new(ErrorKind::Other, format!("Failed to read cert file: {e}")))?;
    // let cert_chain = vec![CertificateDer::from(cert_bytes as Vec<u8>)];

    // Build server config
    let file = File::open(&key_path)?;
    let mut reader = BufReader::new(file);

    let private_key: PrivateKeyDer = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| Error::new(ErrorKind::Other, format!("PEM parse error: {e}")))?
        .ok_or_else(|| Error::new(ErrorKind::Other, "No private key found in key file"))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)
        .map_err(|e| Error::new(ErrorKind::Other, format!("TLS config build error: {e}")))?;

    Ok(config)
}

fn start_key_listener(tx: oneshot::Sender<()>) {
    std::thread::spawn(move || {
        println!("Press Enter to shut down...");
        let _ = std::io::stdin().read(&mut [0u8]).unwrap(); // Wait for any input
        let _ = tx.send(());
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUST_LOG").is_none() {
        unsafe { std::env::set_var("RUST_LOG", "actix_web=info") };
    }

    let port: u16 = (*utils::constants::PORT).clone();
    let address: String = (*utils::constants::ADDRESS).clone();
    let database_url: String = (*utils::constants::DATABASE_URL).clone();
    let cert_path: String = (*utils::constants::TLS_CERT_PATH).clone();
    let key_path: String = (*utils::constants::TLS_KEY_PATH).clone();

    dotenv::dotenv().ok();
    env_logger::init();
    rustls::crypto::ring::default_provider()
    .install_default()
    .expect("Failed to install rustls crypto provider");


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
    let users = entities::users::Entity::find().all(&db).await.map_err(|err| MainError::Db(err))?;

    for user in users {
       println!("Loaded User: {}", user.name);
    }
    
    let order_books = Arc::new(Mutex::new(HashMap::new()));

    let(tx, rx) = oneshot::channel();
    start_key_listener(tx);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db: db.clone(),
                order_books: order_books.clone(),
            }))
            .wrap(Logger::default())
            .configure(routes::home_routes::config)
            .configure(routes::order_book_routes::config)
    })
    .bind_rustls_0_23("0.0.0.0:8443", config)?
    .run();

    let shutdown = server.handle();
    tokio::spawn(async move {
        rx.await.ok();
        shutdown.stop(true).await;
    });

    let _ = server.await;
    Ok(())

}