//! Build-preserving Rust string literal hiding.

use nightmare_core::{Language, NightmareError, Result};

pub struct StringEncryptor<'a> {
    seed: &'a [u8],
}

impl<'a> StringEncryptor<'a> {
    pub fn new(seed: &'a [u8]) -> Self {
        Self { seed }
    }

    pub fn encrypt_strings(&self, content: &str, language: Language) -> Result<String> {
        if language != Language::Rust {
            return Ok(content.to_string());
        }

        let tree = parse_rust(content)?;
        let literals = find_safe_string_literals(content, &tree);
        if literals.is_empty() {
            return Ok(content.to_string());
        }

        let mut output = String::with_capacity(content.len() + literals.len() * 180);
        let mut cursor = 0;

        for (index, literal) in literals.into_iter().enumerate() {
            output.push_str(&content[cursor..literal.start]);
            output.push_str(&self.replacement(index, literal.value.as_bytes()));
            cursor = literal.end;
        }

        output.push_str(&content[cursor..]);
        parse_rust(&output)?;
        Ok(output)
    }

    fn replacement(&self, index: usize, bytes: &[u8]) -> String {
        let key = self.seed[index % self.seed.len()].wrapping_add(index as u8);
        let encrypted = bytes
            .iter()
            .enumerate()
            .map(|(offset, byte)| format!("{}u8", byte ^ key.wrapping_add((offset % 251) as u8)))
            .collect::<Vec<_>>()
            .join(", ");
        let symbol = format!("__NM_STRING_{index}");

        format!(
            r#"{{ static {symbol}: std::sync::OnceLock<String> = std::sync::OnceLock::new(); {symbol}.get_or_init(|| {{ let __nm_data = [{encrypted}]; let mut __nm_out = Vec::with_capacity(__nm_data.len()); for (i, b) in __nm_data.iter().enumerate() {{ __nm_out.push(*b ^ ({key}u8).wrapping_add((i % 251) as u8)); }} String::from_utf8(__nm_out).expect("nightmare string") }}).as_str() }}"#
        )
    }
}

#[derive(Debug)]
struct Literal {
    start: usize,
    end: usize,
    value: String,
}

fn find_safe_string_literals(content: &str, tree: &tree_sitter::Tree) -> Vec<Literal> {
    let mut literals = Vec::new();
    let mut cursor = tree.walk();
    collect_literals(content, tree.root_node(), &mut cursor, &mut literals);
    literals.sort_by_key(|literal| literal.start);
    literals
}

fn collect_literals(
    content: &str,
    node: tree_sitter::Node,
    cursor: &mut tree_sitter::TreeCursor,
    literals: &mut Vec<Literal>,
) {
    if node.kind() == "string_literal" && is_safe_literal_node(node) {
        let start = node.start_byte();
        let end = node.end_byte();
        let bytes = content.as_bytes();

        if !is_raw_or_byte_string(content, start) {
            if let Some((_, value)) = parse_simple_string(bytes, start) {
                literals.push(Literal { start, end, value });
            }
        }
    }

    if cursor.goto_first_child() {
        loop {
            collect_literals(content, cursor.node(), cursor, literals);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn is_raw_or_byte_string(content: &str, quote: usize) -> bool {
    let prefix = &content[..quote];
    prefix.ends_with('r') || prefix.ends_with("br") || prefix.ends_with('b')
}

fn is_safe_literal_node(node: tree_sitter::Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "macro_invocation" | "attribute_item" | "const_item" | "static_item"
            | "match_pattern" | "match_arm" | "let_condition" => return false,
            _ => {}
        }
        current = parent.parent();
    }

    true
}

fn parse_simple_string(bytes: &[u8], start: usize) -> Option<(usize, String)> {
    let mut i = start + 1;
    let mut value = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return String::from_utf8(value).ok().map(|value| (i + 1, value)),
            b'\\' => return None,
            byte => {
                value.push(byte);
                i += 1;
            }
        }
    }
    None
}

fn parse_rust(content: &str) -> Result<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_rust::language())
        .map_err(|e| NightmareError::Parse(e.to_string()))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| NightmareError::Parse("Rust parser returned no tree".to_string()))?;

    if tree.root_node().has_error() {
        return Err(NightmareError::Parse(
            "Rust syntax error detected before/after string encryption".to_string(),
        ));
    }

    Ok(tree)
}
