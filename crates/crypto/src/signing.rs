use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use nightmare_core::{NightmareError, Result};
use std::path::Path;

const DOMAIN: &[u8] = b"nightmare-manifest-v1\0";

pub struct ManifestSigner {
    signing_key: SigningKey,
}

impl ManifestSigner {
    pub fn ephemeral(seed: [u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn from_seed_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let bytes = STANDARD
            .decode(text.trim())
            .map_err(|err| NightmareError::Crypto(format!("invalid signing key base64: {err}")))?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| NightmareError::Crypto("signing key must be 32 bytes".to_string()))?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&seed),
        })
    }

    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.signing_key.verifying_key().as_bytes())
    }

    pub fn sign_manifest(&self, manifest_json: &[u8]) -> String {
        let signature = self.signing_key.sign(&domain_message(manifest_json));
        STANDARD.encode(signature.to_bytes())
    }
}

pub fn public_key_from_seed_file(path: &Path) -> Result<String> {
    Ok(ManifestSigner::from_seed_file(path)?.public_key_base64())
}

pub fn verify_manifest_signature(
    public_key_base64: &str,
    signature_base64: &str,
    manifest_json: &[u8],
) -> Result<()> {
    let public_key_bytes = STANDARD
        .decode(public_key_base64.trim())
        .map_err(|err| NightmareError::Crypto(format!("invalid public key base64: {err}")))?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| NightmareError::Crypto("public key must be 32 bytes".to_string()))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|err| NightmareError::Crypto(format!("invalid public key: {err}")))?;

    let signature_bytes = STANDARD
        .decode(signature_base64.trim())
        .map_err(|err| NightmareError::Crypto(format!("invalid signature base64: {err}")))?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| NightmareError::Crypto("signature must be 64 bytes".to_string()))?;
    let signature = Signature::from_bytes(&signature_bytes);

    verifying_key
        .verify_strict(&domain_message(manifest_json), &signature)
        .map_err(|err| NightmareError::Crypto(format!("manifest signature mismatch: {err}")))
}

fn domain_message(manifest_json: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(DOMAIN.len() + manifest_json.len());
    message.extend_from_slice(DOMAIN);
    message.extend_from_slice(manifest_json);
    message
}
