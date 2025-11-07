use crate::SecretKey;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use crypto_secretbox::{XSalsa20Poly1305, aead::Aead, cipher::KeyInit};
use serde::Deserialize;
use std::time;

#[derive(Debug, Deserialize)]
struct EncryptedPayload {
    id: u64,
    #[serde(rename = "t")]
    time: i64,
}

pub(crate) fn decrypt_payload(encrypted: &str, secret_key: &SecretKey) -> Result<u64> {
    // Decode base64
    let payload_bytes = BASE64
        .decode(encrypted)
        .context("Failed to decode base64")?;

    // Extract nonce (first 24 bytes) and ciphertext (rest)
    anyhow::ensure!(payload_bytes.len() >= 24, "Payload too short");

    let (nonce_bytes, ciphertext) = payload_bytes.split_at(24);

    // Create cipher with secret key
    let cipher = XSalsa20Poly1305::new(&secret_key.0.into());

    // Create nonce (24 bytes for XSalsa20Poly1305)
    let nonce = nonce_bytes.into();

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .context("Decryption failed")?;

    // Parse JSON
    let payload: EncryptedPayload =
        serde_json::from_slice(&plaintext).context("Failed to parse JSON payload")?;

    // Validate timestamp (reject if more than 14 days old)
    let current_time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let age = current_time - payload.time;
    const FOURTEEN_DAYS: i64 = 14 * 24 * 60 * 60; // 14 days in seconds
    anyhow::ensure!(
        age <= FOURTEEN_DAYS,
        "Timestamp too old: {} seconds old (max {} days)",
        age,
        14
    );

    Ok(payload.id)
}
