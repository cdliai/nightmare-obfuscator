use anyhow::Result;
use colored::Colorize;
use nightmare_core::NightmareManifest;
use std::path::PathBuf;

pub async fn inspect(input: PathBuf) -> Result<()> {
    let manifest_path = input.join(".nightmare/manifest.json");
    if !manifest_path.exists() {
        anyhow::bail!("no metadata vault found at {}", manifest_path.display());
    }

    let manifest: NightmareManifest = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let obfuscated = manifest.files.iter().filter(|f| f.obfuscated).count();

    println!("{}", "Nightmare Metadata Vault".bold().underline());
    println!(
        "  {} Project: {}",
        "->".dimmed(),
        manifest.project.name.cyan()
    );
    println!("  {} Owner: {}", "->".dimmed(), manifest.owner.name.cyan());
    if let Some(contact) = &manifest.owner.contact {
        println!("  {} Contact: {}", "->".dimmed(), contact.cyan());
    }
    println!("  {} Session: {}", "->".dimmed(), manifest.session_id.0);
    println!(
        "  {} Created: {}",
        "->".dimmed(),
        manifest.created_at.to_rfc2822()
    );
    println!(
        "  {} Files: {} total, {} obfuscated",
        "->".dimmed(),
        manifest.files.len().to_string().cyan(),
        obfuscated.to_string().cyan()
    );
    println!(
        "  {} Signature key fingerprint: {}",
        "->".dimmed(),
        manifest.signature_public_key[..16].dimmed()
    );

    Ok(())
}
