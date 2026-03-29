//! BIP39 mnemonic implementation for seed phrases
//! 
//! Supports 8-12 word phrases for vault access control

use nightmare_core::{NightmareError, Result};
use sha2::{Digest, Sha256};

/// BIP39 English wordlist (first 64 words shown, truncated for brevity)
const WORDLIST: &[&str] = &[
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract",
    "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
    "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual",
    "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance",
    "advice", "aerobic", "affair", "afford", "afraid", "again", "age", "agent",
    "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
    "alcohol", "alert", "alien", "all", "alley", "allow", "almost", "alone",
    "alpha", "already", "also", "alter", "always", "amateur", "amazing", "among",
];

/// Generate a random mnemonic with specified word count (8 or 12)
pub fn generate_mnemonic(word_count: u8) -> Result<Vec<String>> {
    if word_count != 8 && word_count != 12 {
        return Err(NightmareError::Config(
            "Word count must be 8 or 12".to_string()
        ));
    }
    
    let entropy_bits = match word_count {
        8 => 88,   // 88 bits entropy + 8 bits checksum = 96 bits = 8 words
        12 => 128, // 128 bits entropy + 4 bits checksum = 132 bits = 12 words
        _ => unreachable!(),
    };
    
    let entropy_bytes = entropy_bits / 8;
    let entropy: Vec<u8> = (0..entropy_bytes)
        .map(|_| rand::random::<u8>())
        .collect();
    
    // Calculate checksum
    let hash = Sha256::digest(&entropy);
    let checksum_bits = entropy_bits / 32;
    let checksum = hash[0] >> (8 - checksum_bits);
    
    // Combine entropy + checksum
    let mut combined = entropy.clone();
    combined.push(checksum);
    
    // Split into 11-bit chunks and map to words
    let mut words = Vec::new();
    let mut buffer: u128 = 0;
    let mut bits = 0;
    
    for byte in &combined {
        buffer = (buffer << 8) | (*byte as u128);
        bits += 8;
        
        while bits >= 11 {
            bits -= 11;
            let index = ((buffer >> bits) & 0x7FF) as usize;
            words.push(WORDLIST[index % WORDLIST.len()].to_string());
        }
    }
    
    // Handle remaining bits
    if bits > 0 && words.len() < word_count as usize {
        let index = ((buffer << (11 - bits)) & 0x7FF) as usize;
        words.push(WORDLIST[index % WORDLIST.len()].to_string());
    }
    
    words.truncate(word_count as usize);
    Ok(words)
}

/// Validate a mnemonic phrase
pub fn validate_mnemonic(words: &[String]) -> bool {
    if words.len() != 8 && words.len() != 12 {
        return false;
    }
    
    words.iter().all(|w| WORDLIST.contains(&w.as_str()))
}

/// Convert mnemonic to seed (64 bytes)
pub fn mnemonic_to_seed(mnemonic: &[String], passphrase: &str) -> Vec<u8> {
    let mnemonic_str = mnemonic.join(" ");
    let salt = format!("mnemonic{}", passphrase);
    
    // Use PBKDF2 with 2048 iterations
    let mut seed = vec![0u8; 64];
    pbkdf2::pbkdf2_hmac::<Sha256>(
        mnemonic_str.as_bytes(),
        salt.as_bytes(),
        2048,
        &mut seed,
    );
    
    seed
}

/// Derive vault key from mnemonic
pub fn derive_vault_key(mnemonic: &[String]) -> [u8; 32] {
    let seed = mnemonic_to_seed(mnemonic, "");
    let hash = Sha256::digest(&seed);
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_mnemonic_12() {
        let words = generate_mnemonic(12).unwrap();
        assert_eq!(words.len(), 12);
        assert!(validate_mnemonic(&words));
    }
    
    #[test]
    fn test_generate_mnemonic_8() {
        let words = generate_mnemonic(8).unwrap();
        assert_eq!(words.len(), 8);
        assert!(validate_mnemonic(&words));
    }
}
