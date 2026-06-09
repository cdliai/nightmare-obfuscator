const PUBLIC_CONST: &str = "CONST_LITERAL_STAYS";
pub const PUBLIC_PUB_CONST: &str = "PUB_CONST_LITERAL_STAYS";
static PUBLIC_STATIC: &str = "STATIC_LITERAL_STAYS";

pub fn runtime_secret() -> &'static str {
    "RUNTIME_SECRET_DISAPPEARS"
}

pub fn macro_message() -> String {
    format!("MACRO_LITERAL_STAYS: {}", runtime_secret())
}

pub fn multiline_macro_message() -> String {
    format!(
        "MULTILINE_MACRO_LITERAL_STAYS: {}",
        runtime_secret()
    )
}

pub fn classify(value: &str) -> &'static str {
    match value {
        "PATTERN_LITERAL_STAYS" => PUBLIC_CONST,
        _ => PUBLIC_STATIC,
    }
}

pub fn if_let_pattern(value: &str) -> bool {
    if let "IF_LET_PATTERN_STAYS" = value {
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_secret_survives() {
        let expected = ["RUNTIME", "SECRET", "DISAPPEARS"].join("_");
        assert_eq!(runtime_secret(), expected);
        assert_eq!(macro_message(), format!("MACRO_LITERAL_STAYS: {}", expected));
        assert_eq!(
            multiline_macro_message(),
            format!("MULTILINE_MACRO_LITERAL_STAYS: {}", expected)
        );
        assert_eq!(classify("PATTERN_LITERAL_STAYS"), "CONST_LITERAL_STAYS");
        assert_eq!(PUBLIC_PUB_CONST, "PUB_CONST_LITERAL_STAYS");
        assert!(if_let_pattern("IF_LET_PATTERN_STAYS"));
    }
}
