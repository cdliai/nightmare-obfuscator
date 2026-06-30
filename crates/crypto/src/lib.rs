//! Cryptographic primitives for Nightmare
//!
//! Implements:
//! - AES-256-GCM for owner encryption
//! - ChaCha20-Poly1305 for vault encryption
//! - Scrypt for key derivation
//! - HMAC for integrity verification

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chacha20poly1305::ChaCha20Poly1305;
use nightmare_core::{NightmareError, Result};
use pbkdf2::pbkdf2_hmac;
use scrypt::{scrypt, Params as ScryptParams};
use sha2::Sha256;

pub mod bip39;
pub mod keys;
pub mod signing;

pub use keys::{KeyDerivation, MasterKey, VaultKey};

/// Encrypted blob with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedBlob {
    /// Algorithm used
    pub algorithm: String,
    /// Salt for key derivation
    pub salt: String,
    /// Nonce/IV
    pub nonce: String,
    /// Ciphertext
    pub ciphertext: String,
    /// Authentication tag
    pub tag: String,
    /// Additional authenticated data
    pub aad: Option<String>,
    /// Self-destruct counter (decrements on each decrypt attempt)
    pub attempts_remaining: Option<u32>,
}

/// Primary encryption for owner access
pub struct OwnerEncryption {
    key: MasterKey,
}

impl OwnerEncryption {
    pub fn new(key: MasterKey) -> Self {
        Self { key }
    }

    pub fn encrypt(&self, plaintext: &[u8], aad: Option<&[u8]>) -> Result<EncryptedBlob> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key.0);
        let cipher = Aes256Gcm::new(key);

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let payload = aes_gcm::aead::Payload {
            msg: plaintext,
            aad: aad.unwrap_or(b""),
        };

        let ciphertext = cipher
            .encrypt(&nonce, payload)
            .map_err(|e| NightmareError::Crypto(e.to_string()))?;

        // Split ciphertext and tag for AES-GCM
        let ct_len = ciphertext.len() - 16; // AES-GCM tag is 16 bytes
        let (ct, tag) = ciphertext.split_at(ct_len);

        Ok(EncryptedBlob {
            algorithm: "AES-256-GCM".to_string(),
            salt: hex::encode(self.key.1), // salt used for key derivation
            nonce: hex::encode(nonce.as_slice()),
            ciphertext: BASE64.encode(ct),
            tag: hex::encode(tag),
            aad: aad.map(|a| BASE64.encode(a)),
            attempts_remaining: None,
        })
    }

    pub fn decrypt(&self, blob: &EncryptedBlob, aad: Option<&[u8]>) -> Result<Vec<u8>> {
        if blob.algorithm != "AES-256-GCM" {
            return Err(NightmareError::Crypto(
                "Wrong algorithm for owner decryption".to_string(),
            ));
        }

        let key = Key::<Aes256Gcm>::from_slice(&self.key.0);
        let cipher = Aes256Gcm::new(key);

        let nonce_bytes =
            hex::decode(&blob.nonce).map_err(|e| NightmareError::Crypto(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = BASE64
            .decode(&blob.ciphertext)
            .map_err(|e| NightmareError::Crypto(e.to_string()))?;
        let tag = hex::decode(&blob.tag).map_err(|e| NightmareError::Crypto(e.to_string()))?;

        // Reconstruct ciphertext + tag
        ciphertext.extend_from_slice(&tag);

        let _blob_aad = blob.aad.as_ref().and_then(|a| BASE64.decode(a).ok());
        let payload = aes_gcm::aead::Payload {
            msg: &ciphertext,
            aad: aad.unwrap_or(b""),
        };

        cipher
            .decrypt(nonce, payload)
            .map_err(|e| NightmareError::Crypto(format!("Decryption failed: {}", e)))
    }
}

/// Vault encryption with self-destruct and timelock
pub struct VaultEncryption {
    key: VaultKey,
    max_attempts: u32,
}

impl VaultEncryption {
    pub fn new(key: VaultKey, max_attempts: u32) -> Self {
        Self { key, max_attempts }
    }

    pub fn encrypt(&self, plaintext: &[u8], attempts: Option<u32>) -> Result<EncryptedBlob> {
        let key = chacha20poly1305::Key::from_slice(&self.key.0);
        let cipher = ChaCha20Poly1305::new(key);

        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| NightmareError::Crypto(e.to_string()))?;

        let ct_len = ciphertext.len() - 16;
        let (ct, tag) = ciphertext.split_at(ct_len);

        Ok(EncryptedBlob {
            algorithm: "ChaCha20-Poly1305".to_string(),
            salt: hex::encode(self.key.1),
            nonce: hex::encode(nonce.as_slice()),
            ciphertext: BASE64.encode(ct),
            tag: hex::encode(tag),
            aad: None,
            attempts_remaining: attempts.or(Some(self.max_attempts)),
        })
    }

    pub fn decrypt(&self, blob: &mut EncryptedBlob) -> Result<Vec<u8>> {
        // Check attempts counter
        if let Some(attempts) = blob.attempts_remaining {
            if attempts == 0 {
                return Err(NightmareError::AccessDenied(
                    "Self-destruct triggered: too many failed attempts".to_string(),
                ));
            }
            blob.attempts_remaining = Some(attempts - 1);
        }

        let key = chacha20poly1305::Key::from_slice(&self.key.0);
        let cipher = ChaCha20Poly1305::new(key);

        let nonce_bytes =
            hex::decode(&blob.nonce).map_err(|e| NightmareError::Crypto(e.to_string()))?;
        let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);

        let mut ciphertext = BASE64
            .decode(&blob.ciphertext)
            .map_err(|e| NightmareError::Crypto(e.to_string()))?;
        let tag = hex::decode(&blob.tag).map_err(|e| NightmareError::Crypto(e.to_string()))?;

        ciphertext.extend_from_slice(&tag);

        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| NightmareError::Crypto(format!("Vault decryption failed: {}", e)))
    }
}

/// Derive encryption key from password using PBKDF2
pub fn derive_key_pbkdf2(password: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut key);
    key
}

/// Derive encryption key using Scrypt (memory-hard)
pub fn derive_key_scrypt(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let params = ScryptParams::new(15, 8, 1, 32) // N=2^15, r=8, p=1, len=32
        .map_err(|e| NightmareError::Crypto(e.to_string()))?;

    let mut key = [0u8; 32];
    scrypt(password.as_bytes(), salt, &params, &mut key)
        .map_err(|e| NightmareError::Crypto(e.to_string()))?;

    Ok(key)
}

/// Generate random salt
pub fn generate_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut salt);
    salt
}

/// Generate high-entropy random bytes
pub fn secure_random(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut buf);
    buf
}
