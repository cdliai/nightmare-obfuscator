//! Nightmare Obfuscator CLI
//!
//! Defensive IP-protection tooling for controlled collaboration.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tracing::info;

mod commands;

#[derive(Parser)]
#[command(name = "nightmare")]
#[command(about = "Share working obfuscated Rust projects with provenance metadata.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    Obfuscate {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        select: Vec<PathBuf>,
        #[arg(short, long, default_value = "7")]
        intensity: u8,
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
    },

    Vault {
        input: PathBuf,
    },

    Verify {
        input: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = if cli.verbose { "debug" } else { "info" };

    tracing_subscriber::fmt().with_env_filter(filter).init();
    info!(
        "{} {}",
        "Nightmare".red().bold(),
        "Obfuscator v0.1.0".dimmed()
    );

    match cli.command {
        Commands::Obfuscate {
            input,
            output,
            select,
            intensity,
            config,
            ignore,
            no_dead_code,
            no_string_encrypt,
            no_flatten,
        } => {
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
            })
            .await?;
        }

        Commands::Vault { input } => {
            commands::vault::inspect(input).await?;
        }

        Commands::Verify { input } => {
            commands::verify::run(input).await?;
        }
    }

    Ok(())
}
