use anyhow::Result;
use colored::Colorize;
use nightmare_core::{
    RunConfig, RunResult, RunStage, RunStatus, StageStatus, RUN_CONTRACT_SCHEMA_VERSION,
};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::commands::{obfuscate, verify};

pub struct RunArgs {
    pub config: PathBuf,
    pub json: bool,
}

pub async fn run(args: RunArgs) -> Result<()> {
    let config = load_config(&args.config)?;
    let result = execute_config(config, Some(args.config.clone())).await;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_human_result(&result);
    }

    if result.status == RunStatus::Failed {
        anyhow::bail!("nightmare run failed");
    }

    Ok(())
}

pub fn load_config(path: &Path) -> Result<RunConfig> {
    let mut config = RunConfig::from_toml_file(path)?;
    if let Some(base) = path.parent() {
        config.resolve_relative_paths(base);
    }
    config.validate()?;
    Ok(config)
}

pub fn planned_contract_stages(config: &RunConfig, config_path: &Path) -> Vec<RunStage> {
    let mut stages = vec![RunStage::skipped(
        "obfuscate",
        format!("nightmare run {}", config_path.display()),
        "thin adapter will call the core run contract",
    )];

    if config.checks.verify_metadata {
        stages.push(RunStage::skipped(
            "verify",
            format!("nightmare verify {}", config.output.display()),
            "metadata verification stage from run contract",
        ));
    } else {
        stages.push(RunStage::skipped(
            "verify",
            format!("nightmare verify {}", config.output.display()),
            "verify_metadata is false",
        ));
    }

    if let Some(command) = &config.checks.build {
        stages.push(RunStage::skipped(
            "build",
            command.clone(),
            "build/smoke check from run contract",
        ));
    } else {
        stages.push(RunStage::skipped(
            "build",
            "build check",
            "build check disabled",
        ));
    }

    stages
}

pub async fn execute_config(config: RunConfig, config_path: Option<PathBuf>) -> RunResult {
    let mut stages = Vec::new();
    let manifest_path = config.output.join(".nightmare/manifest.json");

    let obfuscate_command = format!("nightmare run {}", display_config(config_path.as_deref()));
    match obfuscate::execute_contract(&config, false).await {
        Ok(summary) => {
            let mut stage = RunStage::passed("obfuscate", obfuscate_command);
            stage.manifest_path = Some(summary.manifest_path.clone());
            stages.push(stage);
        }
        Err(err) => {
            stages.push(RunStage::failed("obfuscate", obfuscate_command, err));
            return result(config, config_path, stages, RunStatus::Failed, None);
        }
    }

    if config.checks.verify_metadata {
        let verify_command = format!("nightmare verify {}", config.output.display());
        match verify::verify_project(&config.output) {
            Ok(_) => {
                let mut stage = RunStage::passed("verify", verify_command);
                stage.manifest_path = Some(manifest_path.clone());
                stages.push(stage);
            }
            Err(err) => {
                stages.push(RunStage::failed("verify", verify_command, err));
                if config.checks.build.is_some() {
                    stages.push(RunStage::skipped(
                        "build",
                        config.checks.build.clone().unwrap_or_default(),
                        "metadata verification failed",
                    ));
                }
                return result(
                    config,
                    config_path,
                    stages,
                    RunStatus::Failed,
                    Some(manifest_path),
                );
            }
        }
    } else {
        stages.push(RunStage::skipped(
            "verify",
            format!("nightmare verify {}", config.output.display()),
            "verify_metadata is false",
        ));
    }

    if let Some(command) = &config.checks.build {
        stages.push(run_build_stage(&config.output, command));
    } else {
        stages.push(RunStage::skipped(
            "build",
            "build check",
            "build check disabled",
        ));
    }

    let status = if stages
        .iter()
        .any(|stage| stage.status == StageStatus::Failed)
    {
        RunStatus::Failed
    } else {
        RunStatus::Passed
    };
    result(config, config_path, stages, status, Some(manifest_path))
}

fn result(
    config: RunConfig,
    config_path: Option<PathBuf>,
    stages: Vec<RunStage>,
    status: RunStatus,
    manifest_path: Option<PathBuf>,
) -> RunResult {
    RunResult {
        schema_version: RUN_CONTRACT_SCHEMA_VERSION,
        status,
        config_path,
        source: config.source,
        output: config.output,
        manifest_path,
        stages,
        gate: None,
    }
}

fn run_build_stage(output: &Path, command: &str) -> RunStage {
    let mut process = shell_command(command);
    let output_result = process.current_dir(output).output();

    match output_result {
        Ok(output_result) if output_result.status.success() => RunStage {
            name: "build".to_string(),
            status: StageStatus::Passed,
            command: Some(command.to_string()),
            exit_code: output_result.status.code(),
            stderr_summary: None,
            manifest_path: None,
        },
        Ok(output_result) => RunStage {
            name: "build".to_string(),
            status: StageStatus::Failed,
            command: Some(command.to_string()),
            exit_code: output_result.status.code(),
            stderr_summary: Some(summarize_command_output(&output_result)),
            manifest_path: None,
        },
        Err(err) => RunStage::failed("build", command, err),
    }
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("sh");
    process.arg("-c").arg(command);
    process
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.arg("/C").arg(command);
    process
}

fn summarize_command_output(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    let mut summary = text.chars().take(600).collect::<String>();
    if text.chars().count() > 600 {
        summary.push_str("...");
    }
    summary
}

fn display_config(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "<contract>".to_string())
}

pub fn print_human_result(result: &RunResult) {
    println!("{}", "Nightmare Run".bold().underline());
    println!("  {} Status: {:?}", "->".dimmed(), result.status);
    println!("  {} Output: {}", "->".dimmed(), result.output.display());
    for stage in &result.stages {
        println!(
            "  {} {}: {:?}",
            stage_marker(stage.status),
            stage.name,
            stage.status
        );
    }
}

fn stage_marker(status: StageStatus) -> colored::ColoredString {
    match status {
        StageStatus::Passed => "✓".green(),
        StageStatus::Failed => "x".red(),
        StageStatus::Skipped => "-".yellow(),
    }
}
