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
        let safe_candidates = filter_unsafe_bindings(content, &tokens, candidates);

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
            collect_pattern_bindings(tokens, &mut i, &mut bindings);
        }

        i += 1;
    }

    bindings
}

fn collect_pattern_bindings(tokens: &[Token], i: &mut usize, bindings: &mut HashSet<String>) {
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;

    while *i < tokens.len() {
        let token = &tokens[*i];
        match token.text.as_str() {
            "=" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => break,
            "else" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => break,
            "if" if brace_depth == 0 && paren_depth == 0 && bracket_depth == 0 => break,
            "mut" | "ref" | "box" => {}
            "{" => brace_depth += 1,
            "}" => brace_depth = brace_depth.saturating_sub(1),
            "(" => paren_depth += 1,
            ")" => paren_depth = paren_depth.saturating_sub(1),
            "[" => bracket_depth += 1,
            "]" => bracket_depth = bracket_depth.saturating_sub(1),
            _ if token.kind == TokenKind::Ident && is_mangleable_binding(&token.text) => {
                let prev = previous_token(tokens, *i).map(|t| t.text.as_str());
                let next = next_token(tokens, *i).map(|t| t.text.as_str());
                if !is_pattern_constructor(&token.text, next)
                    && !matches!(prev, Some(".") | Some("::") | Some("'"))
                    && !matches!(next, Some(":") | Some("::"))
                {
                    bindings.insert(token.text.clone());
                }
            }
            _ => {}
        }

        *i += 1;
    }
}

fn filter_unsafe_bindings(
    content: &str,
    tokens: &[Token],
    candidates: HashSet<String>,
) -> HashSet<String> {
    let mut safe = candidates;

    safe.retain(|candidate| !appears_as_format_placeholder(content, candidate));

    for (idx, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Ident || !safe.contains(&token.text) {
            continue;
        }

        let prev = previous_token(tokens, idx).map(|t| t.text.as_str());
        let next = next_token(tokens, idx).map(|t| t.text.as_str());

        if PRIMITIVE_TYPES.contains(&token.text.as_str())
            || matches!(prev, Some(".") | Some("::") | Some("'"))
            || matches!(next, Some(":") | Some("::"))
            || is_shorthand_field_context(prev, next)
        {
            safe.remove(&token.text);
        }
    }

    safe
}

fn appears_as_format_placeholder(content: &str, candidate: &str) -> bool {
    let prefix = format!("{{{candidate}");
    content.match_indices(&prefix).any(|(idx, _)| {
        matches!(
            content.as_bytes().get(idx + prefix.len()),
            Some(b'}' | b':' | b'?')
        )
    })
}

fn previous_token(tokens: &[Token], idx: usize) -> Option<&Token> {
    idx.checked_sub(1).and_then(|i| tokens.get(i))
}

fn next_token(tokens: &[Token], idx: usize) -> Option<&Token> {
    tokens.get(idx + 1)
}

fn is_mangleable_binding(name: &str) -> bool {
    name.len() > 2
        && name != "_"
        && !name.starts_with('_')
        && starts_like_local_binding(name)
        && !RUST_KEYWORDS.contains(&name)
}

fn starts_like_local_binding(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_lowercase())
        .unwrap_or(false)
}

fn is_pattern_constructor(name: &str, next: Option<&str>) -> bool {
    matches!(next, Some("(") | Some("{")) && name.chars().next().is_some_and(char::is_uppercase)
}

fn is_shorthand_field_context(prev: Option<&str>, next: Option<&str>) -> bool {
    matches!(prev, Some("{") | Some(",")) && matches!(next, Some(",") | Some("}"))
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

        if let Some(next) = skip_raw_string(bytes, i) {
            i = next;
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
                if i < bytes.len() && bytes[i] == b'\'' {
                    i = skip_quoted(bytes, start, b'\'');
                    continue;
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

fn skip_raw_string(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    if bytes.get(i) == Some(&b'b') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'r') {
        return None;
    }
    i += 1;

    let mut hashes = 0usize;
    while bytes.get(i) == Some(&b'#') {
        hashes += 1;
        i += 1;
    }
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    i += 1;

    while i < bytes.len() {
        if bytes[i] == b'"' {
            let mut end = i + 1;
            let mut matched = 0usize;
            while matched < hashes && bytes.get(end) == Some(&b'#') {
                matched += 1;
                end += 1;
            }
            if matched == hashes {
                return Some(end);
            }
        }
        i += 1;
    }

    Some(bytes.len())
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

const PRIMITIVE_TYPES: &[&str] = &[
    "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
    "u128", "usize", "f32", "f64",
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

    #[test]
    fn rust_mangling_preserves_enum_variant_patterns_and_constructors() {
        let source = r#"
fn option_status(value: Option<String>) -> Result<Option<String>, String> {
    let Some(private_value) = value else {
        return Ok(None);
    };
    if private_value.is_empty() {
        return Err("empty".to_string());
    }
    Ok(Some(private_value))
}
"#;

        let mut mangler = SymbolMangler::new(b"seed", Some("file.rs"));
        let output = mangler.mangle_symbols(source, Language::Rust).unwrap();

        assert!(output.contains("let Some("));
        assert!(output.contains("Ok(None)"));
        assert!(output.contains("Err("));
        assert!(output.contains("Ok(Some("));
        assert!(!output.contains("let private_value"));
        parse_rust(&output).unwrap();
    }

    #[test]
    fn rust_mangling_preserves_struct_shorthand_and_primitive_types() {
        let source = r#"
struct SourceFile {
    path: String,
    content: String,
}

fn build(path: String, content: String, byte: u8) -> SourceFile {
    let mut buffer: u128 = 0;
    let local_total = buffer + byte as u128;
    buffer = local_total;
    SourceFile {
        path,
        content,
    }
}
"#;

        let mut mangler = SymbolMangler::new(b"seed", Some("file.rs"));
        let output = mangler.mangle_symbols(source, Language::Rust).unwrap();

        assert!(output.contains("path,"));
        assert!(output.contains("content,"));
        assert!(output.contains("u128"));
        assert!(!output.contains("let local_total"));
        parse_rust(&output).unwrap();
    }

    #[test]
    fn rust_mangling_skips_raw_strings_and_updates_all_local_uses() {
        let source = r##"
fn replacement(symbol: &str, encrypted: &str, key: u8) -> String {
    let local_secret = "secret";
    let result = format!(
        r#"{{ static {symbol}: std::sync::OnceLock<String> = std::sync::OnceLock::new(); [{encrypted}] ^ {key} ^ {local_secret} }}"#
    );
    result
}

fn gibberish(hash: &[u8]) -> String {
    let mut result = String::new();
    for i in 0..16 {
        let byte = hash[i % hash.len()];
        let c = (b'a' + (byte % 26)) as char;
        result.push(c);
    }
    result
}
"##;

        let mut mangler = SymbolMangler::new(b"seed", Some("file.rs"));
        let output = mangler.mangle_symbols(source, Language::Rust).unwrap();

        assert!(output.contains("{symbol}"));
        assert!(output.contains("{encrypted}"));
        assert!(output.contains("{key}"));
        assert!(output.contains("{local_secret}"));
        assert!(output.contains("let local_secret"));
        assert!(!output.contains("byte % 26"));
        assert!(!output.contains("result.push"));
        parse_rust(&output).unwrap();
    }
}
