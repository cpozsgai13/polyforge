use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use rustls::pki_types::pem::PemObject;
use rustls_pemfile::rsa_private_keys;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs1KeyDer};

pub fn load_key(path: &str) -> Result<PrivatePkcs1KeyDer<'static>, std::io::Error> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let key_result = PrivatePkcs1KeyDer::pem_reader_iter(&mut reader)
        .next()
        .ok_or_else(|| Error::new(ErrorKind::Other, "No RSA key found"))?;

    let key = key_result.map_err(|e| Error::new(ErrorKind::Other, format!("PEM parse error: {e}")))?;    
    Ok(key)
}