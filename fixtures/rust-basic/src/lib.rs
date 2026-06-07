use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub title: String,
    checksum: u32,
}

impl Report {
    pub fn new(title: &str, input: &str) -> Self {
        let normalized_input = input.trim();
        let checksum = private_checksum(normalized_input);
        Self {
            title: title.to_string(),
            checksum,
        }
    }

    pub fn checksum(&self) -> u32 {
        self.checksum
    }
}

fn private_checksum(input: &str) -> u32 {
    let local_total = input
        .bytes()
        .fold(0u32, |acc, byte| acc.wrapping_add(byte as u32));
    let display_label = "local_total should stay inside this string";
    assert!(display_label.contains("local_total"));
    local_total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_checksum_is_stable() {
        let report = Report::new("demo", " abc ");
        assert_eq!(report.checksum(), 294);
    }
}
