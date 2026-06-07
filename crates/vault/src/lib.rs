//! Vault system for third-party access control
//!
//! Features:
//! - BIP39 seed phrase authentication (8-12 words)
//! - Time-locked decryption
//! - Self-destruct after N failed attempts
//! - Tiered access levels

use nightmare_core::{NightmareError, Result, VaultConfig, VaultEntry};
use nightmare_crypto::bip39::{generate_mnemonic, validate_mnemonic};
use nightmare_crypto::{EncryptedBlob, VaultEncryption, VaultKey};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod access;
pub mod timelock;

pub use access::{AccessLevel, AccessToken};
pub use timelock::Timelock;

/// A secure vault for sensitive code/artifacts
pub struct Vault {
    name: String,
    config: VaultConfig,
    encryption: VaultEncryption,
    timelock: Option<Timelock>,
    _metadata: VaultMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct VaultMetadata {
    created_at: chrono::DateTime<chrono::Utc>,
    access_level: AccessLevel,
    description: String,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Vault {
    /// Create a new vault with generated seed phrase
    pub fn create(
        name: &str,
        config: VaultConfig,
        description: &str,
    ) -> Result<(Self, Vec<String>)> {
        // Generate seed phrase
        let mnemonic = generate_mnemonic(config.word_count)?;

        // Create encryption key from mnemonic
        let vault_key = VaultKey::from_mnemonic(&mnemonic);
        let encryption = VaultEncryption::new(vault_key, config.max_attempts);

        let timelock = if config.timelock_seconds > 0 {
            Some(Timelock::new(config.timelock_seconds))
        } else {
            None
        };

        let metadata = VaultMetadata {
            created_at: chrono::Utc::now(),
            access_level: AccessLevel::Standard,
            description: description.to_string(),
            expires_at: None,
        };

        let vault = Self {
            name: name.to_string(),
            config,
            encryption,
            timelock,
            _metadata: metadata,
        };

        Ok((vault, mnemonic))
    }

    /// Open vault with seed phrase
    pub fn open(
        name: &str,
        config: VaultConfig,
        mnemonic: &[String],
        encrypted_data: &EncryptedBlob,
    ) -> Result<Self> {
        if !validate_mnemonic(mnemonic) {
            return Err(NightmareError::AccessDenied(
                "Invalid seed phrase".to_string(),
            ));
        }

        // Check timelock
        if let Some(ref timelock) = encrypted_data.aad {
            let lock = Timelock::from_string(timelock)?;
            if !lock.is_unlocked() {
                return Err(NightmareError::AccessDenied(format!(
                    "Vault locked until {:?}",
                    lock.unlock_time()
                )));
            }
        }

        let vault_key = VaultKey::from_mnemonic(mnemonic);
        let encryption = VaultEncryption::new(vault_key, config.max_attempts);

        Ok(Self {
            name: name.to_string(),
            config,
            encryption,
            timelock: None,
            _metadata: VaultMetadata {
                created_at: chrono::Utc::now(),
                access_level: AccessLevel::Standard,
                description: String::new(),
                expires_at: None,
            },
        })
    }

    /// Encrypt content for this vault
    pub fn encrypt(&self, content: &[u8]) -> Result<EncryptedBlob> {
        let attempts = Some(self.config.max_attempts);
        self.encryption.encrypt(content, attempts)
    }

    /// Decrypt content from blob
    pub fn decrypt(&self, blob: &mut EncryptedBlob) -> Result<Vec<u8>> {
        self.encryption.decrypt(blob)
    }

    /// Convert to manifest entry
    pub fn to_entry(&self, encrypted_path: PathBuf) -> VaultEntry {
        VaultEntry {
            name: self.name.clone(),
            encrypted_path,
            word_count: self.config.word_count,
            timelock_enabled: self.timelock.is_some(),
            public_key: "vault".to_string(), // Simplified
        }
    }
}

/// Vault manager for multiple vaults
pub struct VaultManager {
    vaults: HashMap<String, Vault>,
}

impl VaultManager {
    pub fn new() -> Self {
        Self {
            vaults: HashMap::new(),
        }
    }

    pub fn create_vault(
        &mut self,
        name: &str,
        config: VaultConfig,
        description: &str,
    ) -> Result<Vec<String>> {
        let (vault, mnemonic) = Vault::create(name, config, description)?;
        self.vaults.insert(name.to_string(), vault);
        Ok(mnemonic)
    }

    pub fn get_vault(&self, name: &str) -> Option<&Vault> {
        self.vaults.get(name)
    }

    pub fn get_vault_mut(&mut self, name: &str) -> Option<&mut Vault> {
        self.vaults.get_mut(name)
    }
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new()
    }
}
