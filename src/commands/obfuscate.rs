use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use nightmare_core::{
    FileEntry, Language, ManifestOwner, ManifestProject, NightmareManifest, ObfuscationConfig,
    RunConfig, SessionId, SourceFile,
};
use nightmare_crypto::signing::ManifestSigner;
use nightmare_obfuscator::ObfuscationEngine;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub struct ObfuscateArgs {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub select: Vec<PathBuf>,
    pub intensity: Option<u8>,
    pub config: Option<PathBuf>,
    pub ignore: Vec<String>,
    pub no_dead_code: bool,
    pub no_string_encrypt: bool,
    pub no_flatten: bool,
    pub signing_key: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ObfuscationSummary {
    pub manifest_path: PathBuf,
}

pub async fn run(args: ObfuscateArgs) -> Result<()> {
    let config = resolve_run_config(&args)?;
    execute_contract(&config, true).await?;
    Ok(())
}

pub fn resolve_run_config(args: &ObfuscateArgs) -> Result<RunConfig> {
    dotenvy::dotenv().ok();

    let mut config = if let Some(config_path) = &args.config {
        let mut loaded = RunConfig::from_toml_file(config_path)?;
        if let Some(base) = config_path.parent() {
            loaded.resolve_relative_paths(base);
        }
        loaded.source = args.input.clone();
        loaded
    } else {
        let output = args
            .output
            .clone()
            .unwrap_or_else(|| default_output_path(&args.input));
        let project_name = args
            .input
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();
        RunConfig::template(
            args.input.clone(),
            output,
            std::env::var("NIGHTMARE_OWNER").unwrap_or_else(|_| "unclaimed".to_string()),
            std::env::var("NIGHTMARE_PROJECT").unwrap_or(project_name),
        )
    };

    if let Some(output) = &args.output {
        config.output = output.clone();
    }
    if !args.select.is_empty() {
        config.selected_paths = args.select.clone();
    }
    config.ignored_patterns.extend(args.ignore.clone());
    if let Some(intensity) = args.intensity {
        config.intensity = intensity.clamp(1, 10);
    }
    if args.no_dead_code {
        config.features.dead_code = false;
    }
    if args.no_flatten {
        config.features.flatten_control_flow = false;
    }
    if args.no_string_encrypt {
        config.features.encrypt_strings = false;
    }
    if let Some(signing_key) = &args.signing_key {
        config.signing.private_key_path = Some(signing_key.clone());
    } else if config.signing.private_key_path.is_none() {
        config.signing.private_key_path = std::env::var("NIGHTMARE_SIGNING_KEY_PATH")
            .ok()
            .map(PathBuf::from);
    }

    config.validate()?;
    Ok(config)
}

pub async fn execute_contract(config: &RunConfig, emit_human: bool) -> Result<ObfuscationSummary> {
    config.validate()?;

    let input = config
        .source
        .canonicalize()
        .with_context(|| format!("input path does not exist: {}", config.source.display()))?;
    let output = config.output.clone();

    if output.exists()
        && output
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        anyhow::bail!(
            "output directory already exists and is not empty: {}",
            output.display()
        );
    }

    let obfuscation_config = ObfuscationConfig {
        intensity: config.intensity.clamp(1, 10),
        dead_code: config.features.dead_code,
        encrypt_strings: config.features.encrypt_strings,
        flatten_control_flow: config.features.flatten_control_flow,
        opaque_predicates: false,
        rename_identifiers: config.features.rename_identifiers,
        ..ObfuscationConfig::default()
    };

    std::fs::create_dir_all(&output)?;

    let defaults = default_ignores();
    let ignore_patterns = defaults
        .iter()
        .chain(config.ignored_patterns.iter())
        .cloned()
        .collect::<Vec<_>>();
    let selected_paths = config
        .selected_paths
        .iter()
        .map(|p| normalize_selected_path(&input, p))
        .collect::<Result<Vec<_>>>()?;
    reject_non_rust_selected_files(&input, &selected_paths)?;

    let entries = collect_entries(&input, &ignore_patterns)?;
    let pb = if emit_human {
        let pb = ProgressBar::new(entries.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")?
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let mut engine = ObfuscationEngine::new(obfuscation_config.clone());
    let mut files = Vec::new();
    let input_is_file = input.is_file();

    for entry in entries {
        let rel = if input_is_file {
            input.file_name().map(PathBuf::from).unwrap_or_default()
        } else {
            entry.path().strip_prefix(&input)?.to_path_buf()
        };

        if rel.starts_with(".nightmare") {
            continue;
        }

        let output_path = output.join(&rel);
        if let Some(pb) = &pb {
            pb.set_message(rel.display().to_string());
        }

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let before = std::fs::read(entry.path())?;
        let language = language_for_path(entry.path());
        let should_obfuscate = should_obfuscate_path(&rel, language, &selected_paths);

        let after = if should_obfuscate {
            let content = String::from_utf8(before.clone()).with_context(|| {
                format!("selected Rust source is not valid UTF-8: {}", rel.display())
            })?;
            let source = SourceFile {
                path: entry.path().to_path_buf(),
                content,
                language,
                checksum: checksum_bytes(&before),
            };
            engine.obfuscate(&source)?.into_bytes()
        } else {
            before.clone()
        };

        std::fs::write(&output_path, &after)?;
        std::fs::set_permissions(&output_path, entry.metadata()?.permissions())?;
        files.push(FileEntry {
            path: rel,
            language,
            checksum_before: checksum_bytes(&before),
            checksum_after: checksum_bytes(&after),
            obfuscated: should_obfuscate,
        });

        if let Some(pb) = &pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = &pb {
        pb.finish_with_message("copy complete");
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let session_id = SessionId::new();
    let signer = signer_for_config(config)?;
    let signature_public_key = signer.public_key_base64();
    let manifest = NightmareManifest {
        session_id,
        created_at: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        owner: ManifestOwner {
            name: config.owner.name.clone(),
            contact: config.owner.contact.clone(),
        },
        project: ManifestProject {
            name: config.project.name.clone(),
            source_root: input.clone(),
        },
        selected_paths: selected_paths.clone(),
        ignored_patterns: ignore_patterns,
        files,
        obfuscation_hash: checksum_bytes(&serde_json::to_vec(&obfuscation_config)?),
        signature_public_key,
    };

    let vault_dir = output.join(".nightmare");
    std::fs::create_dir_all(&vault_dir)?;
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    let signature = signer.sign_manifest(&manifest_json);
    std::fs::write(vault_dir.join("manifest.json"), manifest_json)?;
    std::fs::write(vault_dir.join("signature"), signature)?;

    let obfuscated_count = manifest.files.iter().filter(|f| f.obfuscated).count();
    if emit_human {
        println!("\n{}", "Obfuscation Summary".bold().underline());
        println!(
            "  {} Output: {}",
            "->".dimmed(),
            output.display().to_string().cyan()
        );
        println!(
            "  {} Files copied: {}",
            "->".dimmed(),
            manifest.files.len().to_string().green()
        );
        println!(
            "  {} Rust files obfuscated: {}",
            "->".dimmed(),
            obfuscated_count.to_string().green()
        );
        println!(
            "  {} Manifest: {}",
            "->".dimmed(),
            output
                .join(".nightmare/manifest.json")
                .display()
                .to_string()
                .cyan()
        );
    }

    Ok(ObfuscationSummary {
        manifest_path: output.join(".nightmare/manifest.json"),
    })
}

fn signer_for_config(config: &RunConfig) -> Result<ManifestSigner> {
    if let Some(path) = &config.signing.private_key_path {
        return Ok(ManifestSigner::from_seed_file(path)?);
    }

    Ok(ManifestSigner::ephemeral(nightmare_crypto::generate_salt()))
}

fn default_output_path(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let name = input
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("output");
    parent.join(format!("{name}-obfs"))
}

fn default_ignores() -> Vec<String> {
    [
        ".git",
        "target",
        "node_modules",
        "vendor",
        ".venv",
        "__pycache__",
        ".nightmare",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn collect_entries(input: &Path, ignore_patterns: &[String]) -> Result<Vec<DirEntry>> {
    if input.is_file() {
        return Ok(vec![WalkDir::new(input)
            .into_iter()
            .next()
            .context("input file was not walkable")??]);
    }

    let mut entries = Vec::new();
    for entry in WalkDir::new(input)
        .into_iter()
        .filter_entry(|e| !is_ignored(e, input, ignore_patterns))
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    Ok(entries)
}

fn is_ignored(entry: &DirEntry, root: &Path, ignore_patterns: &[String]) -> bool {
    if entry.depth() == 0 {
        return false;
    }

    let rel = entry
        .path()
        .strip_prefix(root)
        .unwrap_or_else(|_| entry.path());
    ignore_patterns.iter().any(|pattern| {
        rel.components()
            .any(|c| c.as_os_str().to_string_lossy() == pattern.as_str())
            || rel == Path::new(pattern)
            || rel.starts_with(pattern)
    })
}

fn normalize_selected_path(input: &Path, selected: &Path) -> Result<PathBuf> {
    if selected.is_absolute() {
        return Ok(selected
            .strip_prefix(input)
            .with_context(|| {
                format!(
                    "selected path {} is not inside input {}",
                    selected.display(),
                    input.display()
                )
            })?
            .to_path_buf());
    }

    Ok(selected
        .components()
        .filter(|c| *c != std::path::Component::CurDir)
        .collect())
}

fn should_obfuscate_path(rel: &Path, language: Language, selected_paths: &[PathBuf]) -> bool {
    if !language.is_v1_obfuscation_supported() {
        return false;
    }

    selected_paths.is_empty()
        || selected_paths
            .iter()
            .any(|selected| rel == selected || rel.starts_with(selected))
}

fn language_for_path(path: &Path) -> Language {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let detected = Language::from_extension(ext);
    if detected.is_v1_obfuscation_supported() {
        detected
    } else {
        Language::Unknown
    }
}

fn checksum_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn reject_non_rust_selected_files(input: &Path, selected_paths: &[PathBuf]) -> Result<()> {
    for selected in selected_paths {
        let selected_path = input.join(selected);
        if !selected_path.is_file() {
            continue;
        }
        let detected = Language::from_extension(
            selected_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or(""),
        );
        if !detected.is_v1_obfuscation_supported() {
            anyhow::bail!(
                "Rust-only V1 cannot obfuscate selected non-Rust file: {}",
                selected.display()
            );
        }
    }
    Ok(())
}
