//! Source file detector for Nightmare.
//!
//! V1 obfuscation support is Rust-only. Non-Rust extensions are detected only
//! for roadmap visibility and should not be treated as supported transforms.

use nightmare_core::{Language, NightmareError, Result, SourceFile};
use sha2::{Digest, Sha256};
use std::path::Path;
use walkdir::WalkDir;

pub struct SourceParser;

impl SourceParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse a directory and return all source files
    pub fn parse_directory(
        &self,
        path: &Path,
        ignore_patterns: &[&str],
    ) -> Result<Vec<SourceFile>> {
        let mut files = Vec::new();

        for entry in WalkDir::new(path) {
            let entry = entry.map_err(|e| NightmareError::Io(e.into()))?;

            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // Check ignore patterns
            let path_str = path.to_string_lossy();
            if ignore_patterns.iter().any(|p| path_str.contains(p)) {
                continue;
            }

            if let Some(file) = self.parse_file(path)? {
                files.push(file);
            }
        }

        Ok(files)
    }

    /// Parse a single file
    pub fn parse_file(&self, path: &Path) -> Result<Option<SourceFile>> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let language = Language::from_extension(ext);

        if language == Language::Unknown {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path).map_err(NightmareError::Io)?;

        let checksum = Self::compute_checksum(&content);

        Ok(Some(SourceFile {
            path: path.to_path_buf(),
            content,
            language,
            checksum,
        }))
    }

    fn compute_checksum(content: &str) -> String {
        let hash = Sha256::digest(content.as_bytes());
        hex::encode(hash)
    }
}

impl Default for SourceParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Get file extensions for a language
pub fn get_extensions(lang: Language) -> Vec<&'static str> {
    match lang {
        Language::Rust => vec!["rs"],
        Language::Python => vec!["py", "pyw"],
        Language::JavaScript => vec!["js", "mjs", "cjs"],
        Language::TypeScript => vec!["ts", "tsx", "mts", "cts"],
        Language::Go => vec!["go"],
        Language::C => vec!["c", "h"],
        Language::Cpp => vec!["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
        Language::Java => vec!["java"],
        Language::Unknown => vec![],
    }
}
