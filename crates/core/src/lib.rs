//! Core types and abstractions for the Nightmare Obfuscator

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

pub mod contract;
pub use contract::*;

pub type Result<T> = std::result::Result<T, NightmareError>;

#[derive(Error, Debug)]
pub enum NightmareError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Vault error: {0}")]
    Vault(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),
}

/// Unique identifier for an obfuscation session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Detected source language identifiers.
///
/// V1 obfuscation support is Rust-only. Other variants are roadmap identifiers
/// used for detection and explicit non-support reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    Java,
    Unknown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" => Self::Python,
            "js" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "go" => Self::Go,
            "c" => Self::C,
            "cpp" | "cc" | "cxx" => Self::Cpp,
            "java" => Self::Java,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Unknown => "unknown",
        }
    }

    pub fn is_v1_obfuscation_supported(&self) -> bool {
        matches!(self, Self::Rust)
    }
}

/// A source file to be obfuscated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub language: Language,
    pub checksum: String,
}

/// Obfuscation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfuscationConfig {
    /// Level of obfuscation (1-10)
    pub intensity: u8,
    /// Inject dead code
    pub dead_code: bool,
    /// Flatten control flow
    pub flatten_control_flow: bool,
    /// Encrypt string literals
    pub encrypt_strings: bool,
    /// Rename identifiers
    pub rename_identifiers: bool,
    /// Insert opaque predicates
    pub opaque_predicates: bool,
    /// Generate polymorphic symbols (different per file)
    pub polymorphic_symbols: bool,
    /// Add self-destruct counter
    pub self_destruct: Option<u32>,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            intensity: 5,
            dead_code: true,
            flatten_control_flow: false,
            encrypt_strings: false,
            rename_identifiers: true,
            opaque_predicates: false,
            polymorphic_symbols: true,
            self_destruct: None,
        }
    }
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Master key from environment
    pub master_key: Option<String>,
    /// Algorithm to use
    pub algorithm: EncryptionAlgorithm,
    /// Key derivation iterations
    pub kdf_iterations: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Vault configuration for third-party access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Number of words in seed phrase (8-12)
    pub word_count: u8,
    /// Time lock duration in seconds (0 = no timelock)
    pub timelock_seconds: u64,
    /// Maximum decryption attempts before self-destruct
    pub max_attempts: u32,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            word_count: 12,
            timelock_seconds: 0,
            max_attempts: 5,
        }
    }
}

/// Manifest of an obfuscation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NightmareManifest {
    pub session_id: SessionId,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub owner: ManifestOwner,
    pub project: ManifestProject,
    pub selected_paths: Vec<PathBuf>,
    pub ignored_patterns: Vec<String>,
    pub files: Vec<FileEntry>,
    pub obfuscation_hash: String,
    pub signature_public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub language: Language,
    pub checksum_before: String,
    pub checksum_after: String,
    pub obfuscated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestOwner {
    pub name: String,
    pub contact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestProject {
    pub name: String,
    pub source_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub name: String,
    pub encrypted_path: PathBuf,
    pub word_count: u8,
    pub timelock_enabled: bool,
    pub public_key: String,
}
