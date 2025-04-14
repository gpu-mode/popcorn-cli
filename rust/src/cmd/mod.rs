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
        filepath: Option<String>,
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
        Some(Commands::Submit { filepath }) => {
            let config = load_config()?;
            let cli_id = config.cli_id.ok_or_else(|| {
                anyhow!(
                    "cli_id not found in config file ({}). Please run `popcorn register` first.",
                    get_config_path()
                        .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
                )
            })?;
            let file_to_submit = filepath.or(cli.filepath);
            submit::run_submit_tui(file_to_submit, cli_id).await
        }
        None => {
            let config = load_config()?;
            let cli_id = config.cli_id.ok_or_else(|| {
                anyhow!(
                    "cli_id not found in config file ({}). Please run `popcorn register` first.",
                    get_config_path()
                        .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
                )
            })?;
            submit::run_submit_tui(cli.filepath, cli_id).await
        }
    }
}
