//! Dead code injector - adds fake functions that do nothing

use nightmare_core::{Language, Result};
use rand::seq::SliceRandom;

pub struct DeadCodeInjector {
    intensity: u8,
}

impl DeadCodeInjector {
    pub fn new(intensity: u8) -> Self {
        Self { intensity }
    }

    pub fn inject(&self, content: &str, lang: Language) -> Result<String> {
        let num_injections = (self.intensity / 2).max(1);
        let mut result = content.to_string();

        for index in 0..num_injections {
            let zombie = self.generate_zombie(lang, index);
            // Insert at random positions or at end
            result.push('\n');
            result.push_str(&zombie);
        }

        // Insert opaque branches
        result = self.insert_opaque_branches(&result, lang)?;

        Ok(result)
    }

    fn generate_zombie(&self, lang: Language, index: u8) -> String {
        if lang == Language::Rust {
            return format!(
                r#"
#[allow(dead_code)]
fn _nm_dead_{index}(a: u64) -> u64 {{
    let b = a.wrapping_mul(0x5FE6EB50C7B537A9);
    let c = b ^ (b >> 22);
    c.wrapping_add(0x9E3779B97F4A7C15)
}}
"#
            );
        }

        let zombies = match lang {
            Language::Rust => vec![""],
            Language::Python => vec![
                r#"
def _0o0(a):
    b = (a * 0x5FE6EB50C7B537A9) & 0xFFFFFFFFFFFFFFFF
    c = b ^ (b >> 22)
    return (c + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
"#,
                r#"
_O0l1 = bytes([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE])
"#,
            ],
            Language::C | Language::Cpp => vec![
                r#"
static unsigned long _0o0(unsigned long a) {
    unsigned long b = a * 0x5FE6EB50C7B537A9ULL;
    unsigned long c = b ^ (b >> 22);
    return c + 0x9E3779B97F4A7C15ULL;
}
"#,
                r#"
static const unsigned char _O0l1[] = {0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE};
"#,
            ],
            Language::JavaScript | Language::TypeScript => vec![
                r#"
const _0o0 = (a) => {
    const b = (a * BigInt("0x5FE6EB50C7B537A9")) & BigInt("0xFFFFFFFFFFFFFFFF");
    const c = b ^ (b >> BigInt(22));
    return (c + BigInt("0x9E3779B97F4A7C15")) & BigInt("0xFFFFFFFFFFFFFFFF");
};
"#,
                r#"
const _O0l1 = new Uint8Array([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]);
"#,
            ],
            _ => vec![""],
        };

        zombies
            .choose(&mut rand::thread_rng())
            .unwrap_or(&"")
            .to_string()
    }

    fn insert_opaque_branches(&self, content: &str, lang: Language) -> Result<String> {
        // Opaque predicates: conditions that are always true/false
        // but require symbolic execution to prove

        match lang {
            Language::Rust => {
                // Replace simple if statements with opaque versions
                Ok(content.to_string())
            }
            _ => Ok(content.to_string()),
        }
    }
}
