//! The Nightmare Obfuscation Engine.

use nightmare_core::{Language, ObfuscationConfig, Result, SourceFile};
use rand::seq::SliceRandom;
use std::collections::HashMap;

pub mod control_flow;
pub mod dead_code;
pub mod mangler;
pub mod string_encrypt;

pub use control_flow::ControlFlowFlattener;
pub use dead_code::DeadCodeInjector;
pub use mangler::SymbolMangler;
pub use string_encrypt::StringEncryptor;

/// Main obfuscation engine
pub struct ObfuscationEngine {
    config: ObfuscationConfig,
    symbol_map: HashMap<String, String>,
    session_seed: [u8; 32],
}

impl ObfuscationEngine {
    pub fn new(config: ObfuscationConfig) -> Self {
        let session_seed = nightmare_crypto::generate_salt();
        Self {
            config,
            symbol_map: HashMap::new(),
            session_seed,
        }
    }

    /// Obfuscate a single source file
    pub fn obfuscate(&mut self, file: &SourceFile) -> Result<String> {
        let mut content = file.content.clone();

        // String encryption remains opt-in until it can preserve builds without
        // injecting undeclared runtime dependencies.
        if self.config.encrypt_strings {
            let encryptor = StringEncryptor::new(&self.session_seed);
            content = encryptor.encrypt_strings(&content, file.language)?;
        }

        if self.config.rename_identifiers {
            let mut mangler = SymbolMangler::new(&self.session_seed, file.path.to_str());
            content = mangler.mangle_symbols(&content, file.language)?;
            self.symbol_map.extend(mangler.get_mapping());
        }

        if self.config.flatten_control_flow {
            let flattener = ControlFlowFlattener::new(self.config.intensity);
            content = flattener.flatten(&content, file.language)?;
        }

        if self.config.dead_code {
            let injector = DeadCodeInjector::new(self.config.intensity);
            content = injector.inject(&content, file.language)?;
        }

        if self.config.opaque_predicates {
            content = self.add_opaque_predicates(&content, file.language)?;
        }

        Ok(content)
    }

    /// Get the symbol mapping for deobfuscation
    pub fn get_symbol_map(&self) -> &HashMap<String, String> {
        &self.symbol_map
    }

    fn add_opaque_predicates(&self, content: &str, _lang: Language) -> Result<String> {
        Ok(content.to_string())
    }
}

/// Generate unreadable symbol names
pub fn generate_gibberish(seed: &[u8], index: usize, polymorphic: bool) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(seed);
    hasher.update(index.to_le_bytes());

    if polymorphic {
        // Add randomness for polymorphic symbols
        hasher.update(rand::random::<[u8; 8]>());
    }

    let hash = hasher.finalize();

    let confusing = b"oOIl";
    let mut result = String::with_capacity(16);

    for i in 0..16 {
        let byte = hash[i % hash.len()];
        if i % 3 == 0 && byte % 2 == 0 {
            result.push(confusing[(byte as usize) % confusing.len()] as char);
        } else {
            // Use lowercase that looks similar
            let c = (b'a' + (byte % 26)) as char;
            result.push(c);
        }
    }

    if result.starts_with(|c: char| c.is_ascii_digit()) {
        result.insert(0, '_');
    }

    result
}

/// Generate zombie code (functions that look real but do nothing)
pub fn generate_zombie_function(lang: Language) -> String {
    let zombies = match lang {
        Language::Rust => vec![
            r#"fn _z() { let _a = 0xDEADBEEFu32; let _b = _a.wrapping_mul(0xCAFEBABE); }"#,
            r#"fn __() -> bool { (0x5F3759DF >> 1) < 0x5F3759DF }"#,
        ],
        Language::Python => {
            vec!["def _z():\n    _a = 0xDEADBEEF\n    _b = (_a * 0xCAFEBABE) & 0xFFFFFFFF"]
        }
        Language::C | Language::Cpp => {
            vec!["void _z() { unsigned _a = 0xDEADBEEF; unsigned _b = _a * 0xCAFEBABE; }"]
        }
        Language::JavaScript | Language::TypeScript => {
            vec!["const _z = () => { const _a = 0xDEADBEEF; return (_a * 0xCAFEBABE) >>> 0; };"]
        }
        _ => vec![""],
    };

    zombies
        .choose(&mut rand::thread_rng())
        .unwrap_or(&"")
        .to_string()
}
