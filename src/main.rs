//! Nightmare Obfuscator CLI
//!
//! Defensive IP-protection tooling for controlled collaboration.

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use colored::Colorize;
use std::io::IsTerminal;
use std::path::PathBuf;
use tracing::info;

mod commands;

#[derive(Parser)]
#[command(name = "nightmare")]
#[command(about = "Share working obfuscated Rust projects with provenance metadata.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(long, default_value = "nightmare.toml")]
        config: PathBuf,
        #[arg(long)]
        source: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        owner_contact: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        profile: Option<String>,
        #[arg(short, long)]
        intensity: Option<u8>,
        #[arg(long)]
        select: Vec<PathBuf>,
        #[arg(short = 'x', long)]
        ignore: Vec<String>,
        #[arg(long)]
        no_build_check: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        instant: bool,
        #[arg(long)]
        run: bool,
        #[arg(long)]
        signing_key: Option<PathBuf>,
    },

    Run {
        config: PathBuf,
        #[arg(long)]
        json: bool,
    },

    Obfuscate {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        select: Vec<PathBuf>,
        #[arg(short, long)]
        intensity: Option<u8>,
        #[arg(short, long)]
        config: Option<PathBuf>,
        #[arg(short = 'x', long)]
        ignore: Vec<String>,
        #[arg(long)]
        no_dead_code: bool,
        #[arg(long)]
        no_string_encrypt: bool,
        #[arg(long)]
        no_flatten: bool,
        #[arg(long)]
        signing_key: Option<PathBuf>,
    },

    Vault {
        input: PathBuf,
    },

    Verify {
        input: PathBuf,
        #[arg(long)]
        trusted_public_key: Option<String>,
    },

    Gate {
        #[command(subcommand)]
        command: GateCommands,
    },

    Signing {
        #[command(subcommand)]
        command: SigningCommands,
    },
}

#[derive(Subcommand)]
enum GateCommands {
    Github {
        #[arg(long)]
        repo: String,
        #[arg(long = "ref")]
        git_ref: String,
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SigningCommands {
    PublicKey {
        #[arg(long)]
        signing_key: PathBuf,
    },
    SignManifest {
        input: PathBuf,
        #[arg(long)]
        signing_key: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    info!(
        "{} {}",
        "Nightmare".red().bold(),
        "Obfuscator v0.1.0".dimmed()
    );

    match cli.command {
        Some(Commands::Init {
            config,
            source,
            output,
            owner,
            owner_contact,
            project,
            profile,
            intensity,
            select,
            ignore,
            no_build_check,
            yes,
            json,
            instant,
            run,
            signing_key,
        }) => {
            commands::init::run(commands::init::InitArgs {
                config,
                source,
                output,
                owner,
                owner_contact,
                project,
                profile,
                intensity,
                select,
                ignore,
                no_build_check,
                yes,
                json,
                instant,
                run_after_write: run,
                signing_key,
            })
            .await?;
        }

        Some(Commands::Run { config, json }) => {
            commands::run::run(commands::run::RunArgs { config, json }).await?;
        }

        Some(Commands::Obfuscate {
            input,
            output,
            select,
            intensity,
            config,
            ignore,
            no_dead_code,
            no_string_encrypt,
            no_flatten,
            signing_key,
        }) => {
            commands::obfuscate::run(commands::obfuscate::ObfuscateArgs {
                input,
                output,
                select,
                intensity,
                config,
                ignore,
                no_dead_code,
                no_string_encrypt,
                no_flatten,
                signing_key,
            })
            .await?;
        }

        Some(Commands::Vault { input }) => {
            commands::vault::inspect(input).await?;
        }

        Some(Commands::Verify {
            input,
            trusted_public_key,
        }) => {
            commands::verify::run(input, trusted_public_key).await?;
        }

        Some(Commands::Gate { command }) => match command {
            GateCommands::Github {
                repo,
                git_ref,
                config,
                json,
            } => {
                commands::gate::run(commands::gate::GateCommand::GitHub(
                    commands::gate::GitHubGateArgs {
                        repo,
                        git_ref,
                        config,
                        json,
                    },
                ))
                .await?;
            }
        },

        Some(Commands::Signing { command }) => match command {
            SigningCommands::PublicKey { signing_key } => {
                commands::signing::public_key(signing_key).await?;
            }
            SigningCommands::SignManifest { input, signing_key } => {
                commands::signing::sign_manifest(input, signing_key).await?;
            }
        },

        None if std::io::stdout().is_terminal() => {
            commands::init::run(commands::init::InitArgs {
                config: PathBuf::from("nightmare.toml"),
                source: None,
                output: None,
                owner: None,
                owner_contact: None,
                project: None,
                profile: None,
                intensity: None,
                select: Vec::new(),
                ignore: Vec::new(),
                no_build_check: false,
                yes: false,
                json: false,
                instant: false,
                run_after_write: false,
                signing_key: None,
            })
            .await?;
        }

        None => {
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}
