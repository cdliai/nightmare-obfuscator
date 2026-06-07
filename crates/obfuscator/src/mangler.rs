//! Build-preserving symbol mangling.

use nightmare_core::{Language, NightmareError, Result};
use std::collections::{HashMap, HashSet};

pub struct SymbolMangler<'a> {
    seed: &'a [u8],
    file_context: Option<&'a str>,
    mapping: HashMap<String, String>,
    counter: usize,
}

impl<'a> SymbolMangler<'a> {
    pub fn new(seed: &'a [u8], file_context: Option<&'a str>) -> Self {
        Self {
            seed,
            file_context,
            mapping: HashMap::new(),
            counter: 0,
        }
    }

    pub fn mangle_symbols(&mut self, content: &str, lang: Language) -> Result<String> {
        match lang {
            Language::Rust => self.mangle_rust(content),
            _ => Ok(content.to_string()),
        }
    }

    pub fn get_mapping(&self) -> HashMap<String, String> {
        self.mapping.clone()
    }

    fn mangle_rust(&mut self, content: &str) -> Result<String> {
        parse_rust(content)?;

        let tokens = lex_rust(content);
        let candidates = collect_local_bindings(&tokens);
        let safe_candidates = filter_unsafe_bindings(&tokens, candidates);

        for ident in safe_candidates {
            if self.mapping.contains_key(&ident) {
                continue;
            }

            let mangled = self.generate_mangled_name(&ident);
            self.mapping.insert(ident, mangled);
        }

        if self.mapping.is_empty() {
            return Ok(content.to_string());
        }

        let mut result = String::with_capacity(content.len());
        let mut cursor = 0;

        for token in tokens {
            if token.kind != TokenKind::Ident {
                continue;
            }

            let original = &content[token.start..token.end];
            let Some(mangled) = self.mapping.get(original) else {
                continue;
            };

            result.push_str(&content[cursor..token.start]);
            result.push_str(mangled);
            cursor = token.end;
        }

        result.push_str(&content[cursor..]);
        parse_rust(&result)?;
        Ok(result)
    }

    fn generate_mangled_name(&mut self, original: &str) -> String {
        use sha2::{Digest, Sha256};

        self.counter += 1;

        let mut hasher = Sha256::new();
        hasher.update(self.seed);
        hasher.update(original.as_bytes());
        hasher.update(self.counter.to_le_bytes());

        if let Some(ctx) = self.file_context {
            hasher.update(ctx.as_bytes());
        }

        let hash = hasher.finalize();
        let mut name = String::from("_nm_");
        let alphabet = b"abcdefghijklmnopqrstuvwxyz0123456789";

        for byte in hash.iter().take(15) {
            name.push(alphabet[*byte as usize % alphabet.len()] as char);
        }

        name
    }
}

fn parse_rust(content: &str) -> Result<()> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_rust::language())
        .map_err(|e| NightmareError::Parse(e.to_string()))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| NightmareError::Parse("Rust parser returned no tree".to_string()))?;

    if tree.root_node().has_error() {
        return Err(NightmareError::Parse(
            "Rust syntax error detected before/after obfuscation".to_string(),
        ));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Ident,
    Punct,
    Other,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    text: String,
    start: usize,
    end: usize,
}

fn collect_local_bindings(tokens: &[Token]) -> HashSet<String> {
    let mut bindings = HashSet::new();
    let mut i = 0;

    while i < tokens.len() {
        if tokens[i].text == "let" {
            i += 1;
            while i < tokens.len() && matches!(tokens[i].text.as_str(), "mut" | "ref") {
                i += 1;
            }

            if i < tokens.len()
                && tokens[i].kind == TokenKind::Ident
                && is_mangleable_binding(&tokens[i].text)
            {
                bindings.insert(tokens[i].text.clone());
            }
        }

        i += 1;
    }

    bindings
}

fn filter_unsafe_bindings(tokens: &[Token], candidates: HashSet<String>) -> HashSet<String> {
    let mut safe = candidates;

    for (idx, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Ident || !safe.contains(&token.text) {
            continue;
        }

        let prev = previous_token(tokens, idx).map(|t| t.text.as_str());
        let next = next_token(tokens, idx).map(|t| t.text.as_str());

        if matches!(prev, Some(".") | Some("::") | Some("'"))
            || matches!(next, Some(":") | Some("::"))
        {
            safe.remove(&token.text);
        }
    }

    safe
}

fn previous_token(tokens: &[Token], idx: usize) -> Option<&Token> {
    idx.checked_sub(1).and_then(|i| tokens.get(i))
}

fn next_token(tokens: &[Token], idx: usize) -> Option<&Token> {
    tokens.get(idx + 1)
}

fn is_mangleable_binding(name: &str) -> bool {
    name.len() > 2 && name != "_" && !name.starts_with('_') && !RUST_KEYWORDS.contains(&name)
}

fn lex_rust(content: &str) -> Vec<Token> {
    let bytes = content.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if bytes.get(i..i + 2) == Some(b"//") {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        if bytes.get(i..i + 2) == Some(b"/*") {
            i += 2;
            while i + 1 < bytes.len() && bytes.get(i..i + 2) != Some(b"*/") {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            continue;
        }

        if b == b'"' {
            i = skip_quoted(bytes, i, b'"');
            continue;
        }

        if b == b'\'' {
            let start = i;
            i += 1;
            if i < bytes.len() && is_ident_start(bytes[i]) {
                while i < bytes.len() && is_ident_continue(bytes[i]) {
                    i += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Punct,
                    text: "'".to_string(),
                    start,
                    end: start + 1,
                });
                continue;
            }
            i = skip_quoted(bytes, start, b'\'');
            continue;
        }

        if is_ident_start(b) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Ident,
                text: content[start..i].to_string(),
                start,
                end: i,
            });
            continue;
        }

        let start = i;
        let text = if bytes.get(i..i + 2) == Some(b"::") {
            i += 2;
            "::"
        } else {
            i += 1;
            &content[start..i]
        };
        tokens.push(Token {
            kind: if matches!(text, "." | ":" | "::" | "'") {
                TokenKind::Punct
            } else {
                TokenKind::Other
            },
            text: text.to_string(),
            start,
            end: i,
        });
    }

    tokens
}

fn skip_quoted(bytes: &[u8], mut i: usize, quote: u8) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn is_ident_start(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identifier_is_valid_rust() {
        let mut mangler = SymbolMangler::new(b"seed", Some("file.rs"));
        let generated = mangler.generate_mangled_name("secret");
        assert!(generated.starts_with('_'));
        assert!(generated
            .chars()
            .all(|c| c == '_' || c.is_ascii_alphanumeric()));
    }

    #[test]
    fn rust_mangling_preserves_strings_fields_and_imports() {
        let source = r#"
use std::fmt::Debug;

#[derive(Debug)]
struct Item {
    secret_value: i32,
}

fn private(input: i32) -> i32 {
    let secret_value = input + 1;
    let local_total = secret_value + 1;
    println!("local_total secret_value");
    Item { secret_value: local_total }.secret_value
}
"#;

        let mut mangler = SymbolMangler::new(b"seed", Some("file.rs"));
        let output = mangler.mangle_symbols(source, Language::Rust).unwrap();

        assert!(output.contains("use std::fmt::Debug;"));
        assert!(output.contains("#[derive(Debug)]"));
        assert!(output.contains("\"local_total secret_value\""));
        assert!(output.contains("secret_value:"));
        assert!(output.contains(".secret_value"));
        assert!(!output.contains("let local_total"));
        parse_rust(&output).unwrap();
    }
}
