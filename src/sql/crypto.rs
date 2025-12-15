use aes_gcm::{Aes256Gcm, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use rand::RngCore;
use std::{fs, io};
use crate::sql::DbConnection;

const CONN_FILE: &str = "connections.json.enc";
const KEY_FILE: &str = "connections.key";
const NONCE_LEN: usize = 12;

fn load_or_create_key() -> io::Result<Vec<u8>> {
    if let Ok(k) = fs::read(KEY_FILE) {
        if k.len() == 32 {
            return Ok(k);
        }
    }
    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    fs::write(KEY_FILE, &key)?;
    Ok(key)
}

pub fn encrypt_and_save(connections: &[DbConnection]) -> io::Result<()> {
    let key = load_or_create_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "bad key length"))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let json = serde_json::to_vec(connections)?;
    let ciphertext = cipher.encrypt(nonce, json.as_ref())
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "encryption failure"))?;

    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    fs::write(CONN_FILE, blob)?;
    Ok(())
}

pub fn load_and_decrypt() -> Vec<DbConnection> {
    let data = fs::read(CONN_FILE).unwrap_or_default();
    if data.len() <= NONCE_LEN {
        return Vec::new();
    }

    let key = match load_or_create_key() {
        Ok(k) => k,
        Err(_) => return Vec::new(),
    };
    let cipher = Aes256Gcm::new_from_slice(&key).expect("bad key length");

    let nonce_bytes = &data[..NONCE_LEN];
    let ciphertext = &data[NONCE_LEN..];
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext: Vec<u8> = match cipher.decrypt(nonce, ciphertext) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    serde_json::from_slice(&plaintext).unwrap_or_default()
}
