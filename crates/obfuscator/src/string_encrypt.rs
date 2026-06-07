//! String encryption - hides string literals

use base64::engine::general_purpose::STANDARD;
use base64::engine::Engine;
use nightmare_core::Result;

pub struct StringEncryptor<'a> {
    seed: &'a [u8],
}

impl<'a> StringEncryptor<'a> {
    pub fn new(seed: &'a [u8]) -> Self {
        Self { seed }
    }

    /// Encrypt all string literals in source code
    pub fn encrypt_strings(&self, content: &str) -> Result<String> {
        // Simple XOR encryption for strings
        // Real implementation would use the crypto crate

        let string_pattern = regex::Regex::new(r#""([^"\\]|\\.)*""#).unwrap();

        let result = string_pattern.replace_all(content, |caps: &regex::Captures| {
            let full_match = caps.get(0).unwrap().as_str();
            let inner = &full_match[1..full_match.len() - 1];

            if inner.len() < 2 {
                return full_match.to_string();
            }

            // XOR encrypt
            let encrypted: Vec<u8> = inner
                .bytes()
                .enumerate()
                .map(|(i, b)| b ^ self.seed[i % self.seed.len()])
                .collect();

            let b64 = STANDARD.encode(&encrypted);
            format!("_d(\"{}\")", b64)
        });

        // Add decrypt helper at the top
        let helper = r#"
#[inline(always)]
fn _d(s: &str) -> String {
    let b = base64::decode(s).unwrap_or_default();
    b.iter().enumerate().map(|(i, &x)| (x ^ 0x42) as char).collect()
}
"#;

        Ok(format!("{}{}", helper, result))
    }
}
