use anyhow::Result;
use colored::Colorize;
use nightmare_core::NightmareManifest;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::commands::obfuscate::sign_manifest;

pub async fn run(input: PathBuf) -> Result<()> {
    let manifest_path = input.join(".nightmare/manifest.json");
    let signature_path = input.join(".nightmare/signature");

    if !manifest_path.exists() {
        anyhow::bail!("no .nightmare/manifest.json found in {}", input.display());
    }

    if !signature_path.exists() {
        anyhow::bail!("no .nightmare/signature found in {}", input.display());
    }

    let manifest_json = std::fs::read(&manifest_path)?;
    let manifest: NightmareManifest = serde_json::from_slice(&manifest_json)?;
    let expected_signature = sign_manifest(&manifest.signature_public_key, &manifest_json);
    let actual_signature = std::fs::read_to_string(&signature_path)?.trim().to_string();

    if actual_signature != expected_signature {
        anyhow::bail!("manifest signature mismatch");
    }

    println!("{}", "Manifest".underline());
    println!("  {} Session ID: {}", "->".dimmed(), manifest.session_id.0);
    println!(
        "  {} Created: {}",
        "->".dimmed(),
        manifest.created_at.to_rfc2822()
    );
    println!("  {} Version: {}", "->".dimmed(), manifest.version.cyan());
    println!(
        "  {} Project: {}",
        "->".dimmed(),
        manifest.project.name.cyan()
    );
    println!("  {} Owner: {}", "->".dimmed(), manifest.owner.name.cyan());

    let mut verified = 0usize;
    let mut failed = Vec::new();

    for file in &manifest.files {
        let path = input.join(&file.path);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) => {
                failed.push(format!("missing {} ({err})", file.path.display()));
                continue;
            }
        };

        let current_hash = checksum_bytes(&bytes);
        if current_hash == file.checksum_after {
            verified += 1;
        } else {
            failed.push(format!("checksum mismatch {}", file.path.display()));
        }
    }

    if !failed.is_empty() {
        for item in &failed {
            println!("  {} {}", "x".red(), item);
        }
        anyhow::bail!("verification failed for {} file(s)", failed.len());
    }

    let obfuscated = manifest.files.iter().filter(|f| f.obfuscated).count();
    println!("\n{}", "Verification".underline());
    println!("  {} Signature: valid", "✓".green());
    println!(
        "  {} Files verified: {}",
        "✓".green(),
        verified.to_string().green()
    );
    println!(
        "  {} Obfuscated Rust files: {}",
        "->".dimmed(),
        obfuscated.to_string().green()
    );

    Ok(())
}

fn checksum_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
