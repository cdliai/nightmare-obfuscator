use anyhow::Result;
use nightmare_core::NightmareManifest;
use nightmare_crypto::signing::{public_key_from_seed_file, ManifestSigner};
use std::path::PathBuf;

pub async fn public_key(signing_key: PathBuf) -> Result<()> {
    println!("{}", public_key_from_seed_file(&signing_key)?);
    Ok(())
}

pub async fn sign_manifest(input: PathBuf, signing_key: PathBuf) -> Result<()> {
    let manifest_path = input.join(".nightmare/manifest.json");
    let signature_path = input.join(".nightmare/signature");
    let manifest_json = std::fs::read(&manifest_path)?;
    let manifest: NightmareManifest = serde_json::from_slice(&manifest_json)?;
    let signer = ManifestSigner::from_seed_file(&signing_key)?;

    if manifest.signature_public_key != signer.public_key_base64() {
        anyhow::bail!("signing key does not match manifest public key");
    }

    std::fs::write(signature_path, signer.sign_manifest(&manifest_json))?;
    println!("manifest signed");
    Ok(())
}
