use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use dirs;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs::File;
use std::path::PathBuf;

mod auth;
mod submit;

#[derive(Serialize, Deserialize, Debug, Default)]
struct Config {
    cli_id: Option<String>,
}

fn get_config_path() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|mut path| {
            path.push(".popcorn.yaml");
            path
        })
        .ok_or_else(|| anyhow!("Could not find home directory"))
}

fn load_config() -> Result<Config> {
    let path = get_config_path()?;
    if !path.exists() {
        return Err(anyhow!(
            "Config file not found at {}. Please run `popcorn register` first.",
            path.display()
        ));
    }
    let file = File::open(path)?;
    serde_yaml::from_reader(file).map_err(|e| anyhow!("Failed to parse config file: {}", e))
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional: Path to the solution file
    filepath: Option<String>,

    /// Optional: Directly specify the GPU to use (e.g., "mi300")
    #[arg(long)]
    pub gpu: Option<String>,

    /// Optional: Directly specify the leaderboard (e.g., "fp8")
    #[arg(long)]
    pub leaderboard: Option<String>,

    /// Optional: Specify submission mode (test, benchmark, leaderboard, profile)
    #[arg(long)]
    pub mode: Option<String>,

    // Optional: Specify output file
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Subcommand, Debug)]
enum AuthProvider {
    Discord,
    Github,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Reregister {
        #[command(subcommand)]
        provider: AuthProvider,
    },
    Register {
        #[command(subcommand)]
        provider: AuthProvider,
    },
    Submit {
        /// Optional: Path to the solution file (can also be provided as a top-level argument)
        filepath: Option<String>,

        /// Optional: Directly specify the GPU to use (e.g., "MI300")
        #[arg(long)]
        gpu: Option<String>,

        /// Optional: Directly specify the leaderboard (e.g., "amd-fp8-mm")
        #[arg(long)]
        leaderboard: Option<String>,

        /// Optional: Specify submission mode (test, benchmark, leaderboard, profile)
        #[arg(long)]
        mode: Option<String>,

        // Optional: Specify output file
        #[arg(short, long)]
        output: Option<String>,
    },
}

pub async fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Reregister { provider }) => {
            let provider_str = match provider {
                AuthProvider::Discord => "discord",
                AuthProvider::Github => "github",
            };
            auth::run_auth(true, provider_str).await
        }
        Some(Commands::Register { provider }) => {
            let provider_str = match provider {
                AuthProvider::Discord => "discord",
                AuthProvider::Github => "github",
            };
            auth::run_auth(false, provider_str).await
        }
        Some(Commands::Submit {
            filepath,
            gpu,
            leaderboard,
            mode,
            output,
        }) => {
            let config = load_config()?;
            let cli_id = config.cli_id.ok_or_else(|| {
                anyhow!(
                    "cli_id not found in config file ({}). Please run 'popcorn-cli register' first.",
                    get_config_path()
                        .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
                )
            })?;

            // Use filepath from Submit command first, fallback to top-level filepath
            let final_filepath = filepath.or(cli.filepath);
            submit::run_submit_tui(
                final_filepath, // Resolved filepath
                gpu,            // From Submit command
                leaderboard,    // From Submit command
                mode,           // From Submit command
                cli_id,
                output, // From Submit command
            )
            .await
        }
        None => {
            // Check if any of the submission-related flags were used at the top level
            if cli.gpu.is_some() || cli.leaderboard.is_some() || cli.mode.is_some() {
                return Err(anyhow!(
                    "Please use the 'submit' subcommand when specifying submission options:\n\
                    popcorn-cli submit [--gpu GPU] [--leaderboard LEADERBOARD] [--mode MODE] FILEPATH"
                ));
            }

            // Handle the case where only a filepath is provided (for backward compatibility)
            if let Some(top_level_filepath) = cli.filepath {
                let config = load_config()?;
                let cli_id = config.cli_id.ok_or_else(|| {
                    anyhow!(
                        "cli_id not found in config file ({}). Please run `popcorn register` first.",
                        get_config_path()
                            .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
                    )
                })?;

                // Run TUI with only filepath, no other options
                submit::run_submit_tui(
                    Some(top_level_filepath),
                    None, // No GPU option
                    None, // No leaderboard option
                    None, // No mode option
                    cli_id,
                    None, // No output option
                )
                .await
            } else {
                Err(anyhow!("No command or submission file specified. Use --help for usage."))
            }
        }
    }
}
