//! Key management for Nightmare encryption

use crate::{derive_key_pbkdf2, derive_key_scrypt, generate_salt};
use nightmare_core::Result;
use sha2::{Digest, Sha256};

/// Master key for owner encryption (32 bytes + salt)
#[derive(Debug, Clone)]
pub struct MasterKey(pub [u8; 32], pub [u8; 32]);

impl MasterKey {
    /// Derive from environment variable or string
    pub fn from_env_or_string(key: &str) -> Result<Self> {
        let salt = generate_salt();

        // Use scrypt for master key (memory hard)
        let key_bytes = derive_key_scrypt(key, &salt)?;

        Ok(Self(key_bytes, salt))
    }

    /// Derive from passphrase with custom salt
    pub fn from_passphrase(passphrase: &str, salt: [u8; 32]) -> Result<Self> {
        let key_bytes = derive_key_scrypt(passphrase, &salt)?;
        Ok(Self(key_bytes, salt))
    }
}

/// Key for vault encryption (derived from BIP39 mnemonic)
#[derive(Debug, Clone)]
pub struct VaultKey(pub [u8; 32], pub [u8; 32]);

impl VaultKey {
    pub fn from_mnemonic(mnemonic: &[String]) -> Self {
        let key = crate::bip39::derive_vault_key(mnemonic);
        let salt = generate_salt();
        Self(key, salt)
    }

    pub fn from_seed(seed: &[u8]) -> Self {
        let hash = Sha256::digest(seed);
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        let salt = generate_salt();
        Self(key, salt)
    }
}

/// Key derivation strategies
pub enum KeyDerivation {
    /// PBKDF2 with SHA256
    Pbkdf2 { iterations: u32 },
    /// Scrypt (memory-hard)
    Scrypt,
    /// Argon2 (modern, recommended)
    Argon2,
}

impl KeyDerivation {
    pub fn derive(&self, password: &str, salt: &[u8]) -> Result<[u8; 32]> {
        match self {
            Self::Pbkdf2 { iterations } => Ok(derive_key_pbkdf2(password, salt, *iterations)),
            Self::Scrypt => derive_key_scrypt(password, salt),
            Self::Argon2 => {
                // Argon2id is the recommended variant but requires extra deps
                // For now, fall back to scrypt
                derive_key_scrypt(password, salt)
            }
        }
    }
}
