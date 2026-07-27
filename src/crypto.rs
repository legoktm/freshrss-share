use crate::SecretKey;
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, aead::Aead};
use serde::Deserialize;
use std::time;

#[derive(Debug, Deserialize)]
struct EncryptedPayload {
    id: u64,
    #[serde(rename = "t")]
    time: i64,
}

/// A payload that was successfully decrypted and is not yet expired
#[derive(Debug)]
pub(crate) struct DecryptedPayload {
    pub(crate) id: u64,
    /// Whole days until the URL expires, rounded up
    pub(crate) expires_in_days: u64,
}

const DAY: i64 = 24 * 60 * 60;
const MAX_AGE: i64 = 7 * DAY;

pub(crate) fn decrypt_payload(encrypted: &str, secret_key: &SecretKey) -> Result<DecryptedPayload> {
    // Decode base64
    let payload_bytes = BASE64
        .decode(encrypted)
        .context("Failed to decode base64")?;

    // Extract nonce (first 24 bytes) and ciphertext (rest)
    anyhow::ensure!(payload_bytes.len() >= 24, "Payload too short");

    let (nonce_bytes, ciphertext) = payload_bytes.split_at(24);

    // Create cipher with secret key
    let cipher = XChaCha20Poly1305::new(&secret_key.0.into());

    // Create nonce (24 bytes for XChaCha20Poly1305)
    let nonce = nonce_bytes.into();

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .context("Decryption failed")?;

    // Parse JSON
    let payload: EncryptedPayload =
        serde_json::from_slice(&plaintext).context("Failed to parse JSON payload")?;

    // Validate timestamp (reject if older than MAX_AGE)
    let current_time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let age = current_time - payload.time;
    anyhow::ensure!(
        age <= MAX_AGE,
        "Timestamp too old: {} seconds old (max {} days)",
        age,
        MAX_AGE / DAY
    );

    // Round up so a link with any time left says at least "1 day"
    let remaining = (MAX_AGE - age).max(0) as u64;
    Ok(DecryptedPayload {
        id: payload.id,
        expires_in_days: remaining.div_ceil(DAY as u64),
    })
}
