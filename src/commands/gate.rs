use anyhow::Result;
use nightmare_core::{GateContext, RunResult, RunStage, RunStatus, RUN_CONTRACT_SCHEMA_VERSION};
use std::path::PathBuf;

use crate::commands::run::{load_config, planned_contract_stages};

pub enum GateCommand {
    GitHub(GitHubGateArgs),
}

pub struct GitHubGateArgs {
    pub repo: String,
    pub git_ref: String,
    pub config: PathBuf,
    pub json: bool,
}

pub async fn run(command: GateCommand) -> Result<()> {
    match command {
        GateCommand::GitHub(args) => run_github(args).await,
    }
}

async fn run_github(args: GitHubGateArgs) -> Result<()> {
    let config = load_config(&args.config)?;
    let workspace = std::env::temp_dir().join(format!(
        "nightmare-gate-{}",
        args.git_ref.chars().take(12).collect::<String>()
    ));
    let gate = GateContext {
        provider: "github".to_string(),
        repo: args.repo.clone(),
        git_ref: args.git_ref.clone(),
        disposable_workspace: workspace.clone(),
        mcp_adapter: "thin-adapter-over-nightmare-gate".to_string(),
    };

    if let Err(err) = validate_immutable_ref(&args.git_ref) {
        let result = RunResult {
            schema_version: RUN_CONTRACT_SCHEMA_VERSION,
            status: RunStatus::Failed,
            config_path: Some(args.config.clone()),
            source: config.source,
            output: config.output,
            manifest_path: None,
            stages: vec![RunStage::failed(
                "fetch",
                format!("git checkout {}", args.git_ref),
                err,
            )],
            gate: Some(gate),
        };
        if args.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        anyhow::bail!("nightmare gate failed");
    }

    let mut stages = vec![RunStage::skipped(
        "fetch",
        format!(
            "git clone --no-checkout {} {} && git checkout {}",
            args.repo,
            workspace.display(),
            args.git_ref
        ),
        "experimental gate is plan-only until explicit untrusted build execution is enabled",
    )];
    stages.extend(planned_contract_stages(&config, &args.config));

    let result = RunResult {
        schema_version: RUN_CONTRACT_SCHEMA_VERSION,
        status: RunStatus::Planned,
        config_path: Some(args.config.clone()),
        source: config.source,
        output: config.output,
        manifest_path: None,
        stages,
        gate: Some(gate),
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "Nightmare gate planned for {}",
            result.gate.as_ref().unwrap().repo
        );
        println!("Use --json for agent-readable stage details.");
    }

    Ok(())
}

fn validate_immutable_ref(git_ref: &str) -> Result<()> {
    let is_sha = git_ref.len() == 40 && git_ref.chars().all(|ch| ch.is_ascii_hexdigit());
    if !is_sha {
        anyhow::bail!("--ref must be an immutable 40-character commit SHA");
    }
    Ok(())
}
