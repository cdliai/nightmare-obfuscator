use anyhow::Result;
use colored::Colorize;
use nightmare_core::NightmareManifest;
use nightmare_crypto::signing::verify_manifest_signature;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub struct VerifySummary {
    pub verified_files: usize,
    pub obfuscated_files: usize,
}

pub async fn run(input: PathBuf, trusted_public_key: Option<String>) -> Result<()> {
    let summary = verify_project_with_trust(&input, trusted_public_key.as_deref())?;
    let manifest_json = std::fs::read(input.join(".nightmare/manifest.json"))?;
    let manifest: NightmareManifest = serde_json::from_slice(&manifest_json)?;

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

    println!("\n{}", "Verification".underline());
    println!("  {} Signature: valid", "✓".green());
    println!(
        "  {} Files verified: {}",
        "✓".green(),
        summary.verified_files.to_string().green()
    );
    println!(
        "  {} Obfuscated Rust files: {}",
        "->".dimmed(),
        summary.obfuscated_files.to_string().green()
    );

    Ok(())
}

pub fn verify_project(input: &std::path::Path) -> Result<VerifySummary> {
    verify_project_with_trust(input, None)
}

pub fn verify_project_with_trust(
    input: &std::path::Path,
    trusted_public_key: Option<&str>,
) -> Result<VerifySummary> {
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
    let actual_signature = std::fs::read_to_string(&signature_path)?.trim().to_string();

    if let Some(trusted_public_key) = trusted_public_key {
        if trusted_public_key.trim() != manifest.signature_public_key.trim() {
            anyhow::bail!("trusted public key mismatch");
        }
    }
    verify_manifest_signature(
        &manifest.signature_public_key,
        &actual_signature,
        &manifest_json,
    )?;

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
        anyhow::bail!(
            "verification failed for {} file(s): {}",
            failed.len(),
            failed.join("; ")
        );
    }

    let obfuscated = manifest.files.iter().filter(|f| f.obfuscated).count();

    Ok(VerifySummary {
        verified_files: verified,
        obfuscated_files: obfuscated,
    })
}

fn checksum_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
