use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use nightmare_core::{
    FileEntry, Language, ManifestOwner, ManifestProject, NightmareManifest, ObfuscationConfig,
    SessionId, SourceFile,
};
use nightmare_obfuscator::ObfuscationEngine;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

pub struct ObfuscateArgs {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub select: Vec<PathBuf>,
    pub intensity: u8,
    pub config: Option<PathBuf>,
    pub ignore: Vec<String>,
    pub no_dead_code: bool,
    pub no_string_encrypt: bool,
    pub no_flatten: bool,
}

pub async fn run(args: ObfuscateArgs) -> Result<()> {
    dotenvy::dotenv().ok();

    if args.config.is_some() {
        println!(
            "{} --config is reserved for a future config-file schema and was ignored",
            "warning:".yellow()
        );
    }

    let input = args
        .input
        .canonicalize()
        .with_context(|| format!("input path does not exist: {}", args.input.display()))?;
    let output = args.output.unwrap_or_else(|| default_output_path(&input));

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

    let config = ObfuscationConfig {
        intensity: args.intensity.clamp(1, 10),
        dead_code: !args.no_dead_code,
        encrypt_strings: false,
        flatten_control_flow: !args.no_flatten,
        opaque_predicates: false,
        ..ObfuscationConfig::default()
    };

    if !args.no_string_encrypt {
        println!(
            "{} string encryption is disabled in v1 to preserve builds",
            "warning:".yellow()
        );
    }

    std::fs::create_dir_all(&output)?;

    let defaults = default_ignores();
    let ignore_patterns = defaults
        .iter()
        .chain(args.ignore.iter())
        .cloned()
        .collect::<Vec<_>>();
    let selected_paths = args
        .select
        .iter()
        .map(|p| normalize_selected_path(&input, p))
        .collect::<Result<Vec<_>>>()?;

    let entries = collect_entries(&input, &ignore_patterns)?;
    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40.cyan/blue}] {pos}/{len} {msg}")?
            .progress_chars("#>-"),
    );

    let mut engine = ObfuscationEngine::new(config.clone());
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
        pb.set_message(rel.display().to_string());

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
        files.push(FileEntry {
            path: rel,
            language,
            checksum_before: checksum_bytes(&before),
            checksum_after: checksum_bytes(&after),
            obfuscated: should_obfuscate,
        });

        pb.inc(1);
    }

    pb.finish_with_message("copy complete");
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let session_id = SessionId::new();
    let project_name = input
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();
    let signature_public_key = checksum_bytes(session_id.0.to_string().as_bytes());
    let manifest = NightmareManifest {
        session_id,
        created_at: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        owner: ManifestOwner {
            name: std::env::var("NIGHTMARE_OWNER").unwrap_or_else(|_| "unclaimed".to_string()),
            contact: std::env::var("NIGHTMARE_OWNER_CONTACT").ok(),
        },
        project: ManifestProject {
            name: std::env::var("NIGHTMARE_PROJECT").unwrap_or(project_name),
            source_root: input.clone(),
        },
        selected_paths: selected_paths.clone(),
        ignored_patterns: ignore_patterns,
        files,
        obfuscation_hash: checksum_bytes(&serde_json::to_vec(&config)?),
        signature_public_key,
    };

    let vault_dir = output.join(".nightmare");
    std::fs::create_dir_all(&vault_dir)?;
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    let signature = sign_manifest(&manifest.signature_public_key, &manifest_json);
    std::fs::write(vault_dir.join("manifest.json"), manifest_json)?;
    std::fs::write(vault_dir.join("signature"), signature)?;

    let obfuscated_count = manifest.files.iter().filter(|f| f.obfuscated).count();
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

    Ok(())
}

pub fn sign_manifest(public_key: &str, manifest_json: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nightmare-manifest-v1");
    hasher.update(public_key.as_bytes());
    hasher.update(manifest_json);
    format!("{:x}", hasher.finalize())
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
    let rel_str = rel.to_string_lossy();
    ignore_patterns.iter().any(|pattern| {
        rel.components()
            .any(|c| c.as_os_str().to_string_lossy() == pattern.as_str())
            || rel_str.contains(pattern)
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
    if language != Language::Rust {
        return false;
    }

    selected_paths.is_empty()
        || selected_paths
            .iter()
            .any(|selected| rel == selected || rel.starts_with(selected))
}

fn language_for_path(path: &Path) -> Language {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    Language::from_extension(ext)
}

fn checksum_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
