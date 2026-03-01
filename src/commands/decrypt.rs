use anyhow::Result;
use colored::Colorize;
use nightmare_core::{NightmareError, NightmareManifest};
use nightmare_crypto::{EncryptedBlob, MasterKey, OwnerEncryption};
use std::path::PathBuf;

pub struct DecryptArgs {
    pub input: PathBuf,
    pub output: PathBuf,
    pub key: Option<String>,
}

pub async fn run(args: DecryptArgs) -> Result<()> {
    println!("{}", "Initializing decryption...".bold());

    let master_key = args
        .key
        .or_else(|| std::env::var("NIGHTMARE_KEY").ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Master key
                required (use --key or NIGHTMARE_KEY env)"
            )
        })?;

    let manifest_path = args.input.join("nightmare.lock");
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "Not a nightmare
                obfuscated directory (no nightmare.lock)"
        ));
    }

    let manifest: NightmareManifest =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;

    println!(
        "  {} Session: {}",
        "→".dimmed(),
        manifest.session_id.0.to_string().cyan()
    );
    println!(
        "  {} Created: {}",
        "→".dimmed(),
        manifest.created_at.to_rfc2822().dimmed()
    );

    let key_path = args.input.join(".nightmare-key");
    if !key_path.exists() {
        let map_path = args.input.join(".symbols.json");
        if map_path.exists() {
            println!("{}", "Using unencrypted symbols (not secure)".dimmed());
            let _symbol_map: std::collections::HashMap<String, String> =
                serde_json::from_str(&std::fs::read_to_string(&map_path)?)?;
            // TODO: Implement symbol reversal
        } else {
            return Err(anyhow::anyhow!("No recovery data found"));
        }
    } else {
        let encrypted: EncryptedBlob = serde_json::from_slice(&std::fs::read(&key_path)?)?;

        let key = MasterKey::from_env_or_string(&master_key)?;
        let encryption = OwnerEncryption::new(key);

        let symbol_data = encryption.decrypt(&encrypted, Some(b"symbols"))?;
        let _symbol_map: std::collections::HashMap<String, String> =
            serde_json::from_slice(&symbol_data)?;

        println!("{}", "Recovery data decrypted successfully".dimmed());
    }

    // Create output directory
    std::fs::create_dir_all(&args.output)?;

    // TODO: Implement full deobfuscation
    println!("  Symbol mapping available for manual recovery");
    println!("{}", "Full deobfuscation not yet implemented".red());

    Ok(())
}
