use anyhow::Result;
use colored::Colorize;
use nightmare_core::{RunConfig, RunProfile, RUN_CONTRACT_SCHEMA_VERSION};
use serde::Serialize;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use crate::commands::run as run_command;

pub struct InitArgs {
    pub config: PathBuf,
    pub source: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub owner: Option<String>,
    pub owner_contact: Option<String>,
    pub project: Option<String>,
    pub profile: Option<String>,
    pub intensity: Option<u8>,
    pub select: Vec<PathBuf>,
    pub ignore: Vec<String>,
    pub no_build_check: bool,
    pub yes: bool,
    pub json: bool,
    pub instant: bool,
    pub run_after_write: bool,
    pub signing_key: Option<PathBuf>,
}

#[derive(Serialize)]
struct InitResult {
    schema_version: u16,
    config_path: PathBuf,
    source: PathBuf,
    output: PathBuf,
    mode: &'static str,
}

pub async fn run(args: InitArgs) -> Result<()> {
    let interactive = !args.yes && io::stdin().is_terminal() && io::stdout().is_terminal();
    let config_path = args.config.clone();
    let json = args.json;
    let instant = args.instant;
    let run_after_write = args.run_after_write;
    if json && run_after_write {
        anyhow::bail!("--run cannot be combined with --json; use `nightmare run --json`");
    }
    let mut config = build_template(args)?;

    if interactive {
        prompt_for_values(&mut config)?;
        normalize_paths_for_write(&mut config)?;
    }

    config.validate()?;
    if let Some(parent) = config_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, config.to_toml_string()?)?;

    emit_init_result(&config, &config_path, interactive, json, instant)?;
    if run_after_write {
        let persisted_config = run_command::load_config(&config_path)?;
        let result = run_command::execute_config(persisted_config, Some(config_path)).await;
        run_command::print_human_result(&result);
        if result.status == nightmare_core::RunStatus::Failed {
            anyhow::bail!("nightmare run failed");
        }
    }
    Ok(())
}

fn build_template(args: InitArgs) -> Result<RunConfig> {
    let loaded_existing = args.config.exists();
    let source_supplied = args.source.is_some();
    let output_supplied = args.output.is_some();
    let mut config = if args.config.exists() {
        RunConfig::from_toml_file(&args.config)?
    } else {
        let source = args.source.clone().unwrap_or_else(|| PathBuf::from("."));
        let output = args
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from("./nightmare-obfs"));
        let owner = args
            .owner
            .clone()
            .or_else(|| std::env::var("NIGHTMARE_OWNER").ok())
            .unwrap_or_else(|| "unclaimed".to_string());
        let project = args
            .project
            .clone()
            .or_else(|| std::env::var("NIGHTMARE_PROJECT").ok())
            .unwrap_or_else(|| "project".to_string());

        let mut template = RunConfig::template(source, output, owner, project);
        template.owner.contact = args
            .owner_contact
            .clone()
            .or_else(|| std::env::var("NIGHTMARE_OWNER_CONTACT").ok());
        template
    };

    if let Some(source) = args.source {
        config.source = source;
    }
    if let Some(output) = args.output {
        config.output = output;
    }
    if let Some(owner) = args.owner {
        config.owner.name = owner;
    }
    if let Some(owner_contact) = args.owner_contact {
        config.owner.contact = Some(owner_contact);
    }
    if let Some(project) = args.project {
        config.project.name = project;
    }
    if let Some(profile) = args.profile {
        config.profile = parse_profile(&profile)?;
    }
    if let Some(intensity) = args.intensity {
        config.intensity = intensity.clamp(1, 10);
    }
    if !args.select.is_empty() {
        config.selected_paths = args.select;
    }
    if !args.ignore.is_empty() {
        config.ignored_patterns.extend(args.ignore);
    }
    if args.no_build_check {
        config.checks.build = None;
    }
    if let Some(signing_key) = args.signing_key {
        config.signing.private_key_path = Some(signing_key);
    } else if config.signing.private_key_path.is_none() {
        config.signing.private_key_path = std::env::var("NIGHTMARE_SIGNING_KEY_PATH")
            .ok()
            .map(PathBuf::from);
    }

    if !loaded_existing || source_supplied {
        absolutize_relative_path(&mut config.source)?;
    }
    if !loaded_existing || output_supplied {
        absolutize_relative_path(&mut config.output)?;
    }

    Ok(config)
}

fn normalize_paths_for_write(config: &mut RunConfig) -> Result<()> {
    absolutize_relative_path(&mut config.source)?;
    absolutize_relative_path(&mut config.output)?;
    Ok(())
}

fn absolutize_relative_path(path: &mut PathBuf) -> Result<()> {
    if path.is_relative() {
        *path = std::env::current_dir()?.join(&path);
    }
    Ok(())
}

fn prompt_for_values(config: &mut RunConfig) -> Result<()> {
    println!("{}", retro_banner(false));
    println!("{}", "Guided config setup".bold());
    config.source = prompt_path("Source project", &config.source)?;
    config.output = prompt_path("Output directory", &config.output)?;
    config.owner.name = prompt_string("Owner", &config.owner.name)?;
    config.project.name = prompt_string("Project", &config.project.name)?;
    Ok(())
}

fn prompt_path(label: &str, default: &Path) -> Result<PathBuf> {
    Ok(PathBuf::from(prompt_string(
        label,
        &default.display().to_string(),
    )?))
}

fn prompt_string(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim();
    if value.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(value.to_string())
    }
}

fn parse_profile(value: &str) -> Result<RunProfile> {
    match value {
        "light" => Ok(RunProfile::Light),
        "balanced" => Ok(RunProfile::Balanced),
        "aggressive" => Ok(RunProfile::Aggressive),
        other => anyhow::bail!("unknown profile: {other}; use light, balanced, or aggressive"),
    }
}

fn emit_init_result(
    config: &RunConfig,
    config_path: &Path,
    interactive: bool,
    json: bool,
    instant: bool,
) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&InitResult {
                schema_version: RUN_CONTRACT_SCHEMA_VERSION,
                config_path: config_path.to_path_buf(),
                source: config.source.clone(),
                output: config.output.clone(),
                mode: if interactive { "guided" } else { "template" },
            })?
        );
        return Ok(());
    }

    println!("{}", retro_banner(instant));
    println!(
        "{} {}",
        "Config written:".green().bold(),
        config_path.display()
    );
    println!("  {} Source: {}", "->".dimmed(), config.source.display());
    println!("  {} Output: {}", "->".dimmed(), config.output.display());
    println!(
        "  {} Run: nightmare run {}",
        "->".dimmed(),
        config_path.display()
    );
    Ok(())
}

pub fn retro_banner(_instant: bool) -> &'static str {
    if _instant {
        return "NIGHTMARE :: retro obfuscation console";
    }

    r#" _   _ ___ ____ _   _ _____ __  __    _    ____  _____
| \ | |_ _/ ___| | | |_   _|  \/  |  / \  |  _ \| ____|
|  \| || | |  _| |_| | | | |\/| | / _ \ | |_) |  _|
| |\  || | |_| |  _  | | | | |  | |/ ___ \|  _ <| |___
|_| \_|___\____|_| |_| |_| |_|  |_/_/   \_\_| \_\_____|
NIGHTMARE :: retro obfuscation console"#
}
