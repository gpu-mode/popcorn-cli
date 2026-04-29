use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

mod admin;
mod auth;
mod setup;
mod submissions;
mod submit;

use crate::service;

pub use admin::AdminAction;

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

fn submit_cli_id_from_env() -> Option<String> {
    std::env::var("POPCORN_SUBMITTER_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn resolve_cli_id() -> Result<String> {
    if let Some(cli_id) = submit_cli_id_from_env() {
        return Ok(cli_id);
    }

    let config = load_config()?;
    config.cli_id.ok_or_else(|| {
        anyhow!(
            "cli_id not found in config file ({}). Please run 'popcorn-cli register' first.",
            get_config_path()
                .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
        )
    })
}

#[derive(Parser, Debug)]
#[command(author, version = env!("CLI_VERSION"), about, long_about = None)]
/// Popcorn CLI for GPU Mode competitions. Run `popcorn setup` first in each project so agents use the correct workflow and templates.
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

    /// Skip the TUI and print results directly to stdout
    #[arg(long)]
    pub no_tui: bool,
}

#[derive(Subcommand, Debug)]
enum AuthProvider {
    Discord,
    Github,
}

#[derive(Subcommand, Debug)]
enum SubmissionsAction {
    /// List your submissions for a leaderboard
    List {
        /// Leaderboard name (required)
        #[arg(long)]
        leaderboard: String,

        /// Maximum number of submissions to show
        #[arg(long, default_value = "50")]
        limit: i32,
    },
    /// Show a specific submission with full details and code
    Show {
        /// Submission ID
        id: i64,
    },
    /// Delete a submission
    Delete {
        /// Submission ID
        id: i64,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run this first: bootstrap the project with Popcorn agent skills and a submission template
    Setup,
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

        /// Skip the TUI and print results directly to stdout
        #[arg(long)]
        no_tui: bool,
    },
    /// Join a closed leaderboard using an invite code
    Join {
        /// The invite code
        code: String,
    },
    /// Admin commands (requires POPCORN_ADMIN_TOKEN env var)
    Admin {
        #[command(subcommand)]
        action: AdminAction,
    },
    /// Manage your submissions
    Submissions {
        #[command(subcommand)]
        action: SubmissionsAction,
    },
}

pub async fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Setup) => setup::run_setup().await,
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
            no_tui,
        }) => {
            let cli_id = resolve_cli_id()?;

            // Use filepath from Submit command first, fallback to top-level filepath
            let final_filepath = filepath.or(cli.filepath);

            if no_tui {
                submit::run_submit_plain(
                    final_filepath, // Resolved filepath
                    gpu,            // From Submit command
                    leaderboard,    // From Submit command
                    mode,           // From Submit command
                    cli_id,
                    output, // From Submit command
                )
                .await
            } else {
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
        }
        Some(Commands::Join { code }) => {
            let cli_id = resolve_cli_id()?;
            let client = service::create_client(Some(cli_id))?;
            let result = service::join_with_invite(&client, &code).await?;
            let leaderboards = result["leaderboards"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!("Joined leaderboard(s): {}", leaderboards);
            Ok(())
        }
        Some(Commands::Admin { action }) => admin::handle_admin(action).await,
        Some(Commands::Submissions { action }) => {
            let cli_id = resolve_cli_id()?;

            match action {
                SubmissionsAction::List { leaderboard, limit } => {
                    submissions::list_submissions(cli_id, leaderboard, Some(limit)).await
                }
                SubmissionsAction::Show { id } => submissions::show_submission(cli_id, id).await,
                SubmissionsAction::Delete { id, force } => {
                    submissions::delete_submission(cli_id, id, force).await
                }
            }
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
                let cli_id = resolve_cli_id()?;

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
                Err(anyhow!(
                    "No command or submission file specified. Use --help for usage."
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_cli_id;
    use std::env;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        old_home: Option<String>,
        old_submitter: Option<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self {
                old_home: env::var("HOME").ok(),
                old_submitter: env::var("POPCORN_SUBMITTER_ID").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.old_home {
                Some(v) => env::set_var("HOME", v),
                None => env::remove_var("HOME"),
            }
            match &self.old_submitter {
                Some(v) => env::set_var("POPCORN_SUBMITTER_ID", v),
                None => env::remove_var("POPCORN_SUBMITTER_ID"),
            }
        }
    }

    #[test]
    fn test_resolve_cli_id_prefers_env_over_config() {
        let _lock = ENV_LOCK.lock().expect("Failed to lock env mutex");
        let _guard = EnvGuard::new();

        let temp_home = tempdir().expect("Failed to create temp home dir");
        let config_path = temp_home.path().join(".popcorn.yaml");
        fs::write(config_path, "cli_id: config-cli-id\n").expect("Failed to write config");

        env::set_var("HOME", temp_home.path());
        env::set_var("POPCORN_SUBMITTER_ID", "env-cli-id");

        let cli_id = resolve_cli_id().expect("Expected cli_id resolution to succeed");
        assert_eq!(cli_id, "env-cli-id");
    }

    #[test]
    fn test_resolve_cli_id_falls_back_to_config() {
        let _lock = ENV_LOCK.lock().expect("Failed to lock env mutex");
        let _guard = EnvGuard::new();

        let temp_home = tempdir().expect("Failed to create temp home dir");
        let config_path = temp_home.path().join(".popcorn.yaml");
        fs::write(config_path, "cli_id: config-cli-id\n").expect("Failed to write config");

        env::set_var("HOME", temp_home.path());
        env::remove_var("POPCORN_SUBMITTER_ID");

        let cli_id = resolve_cli_id().expect("Expected cli_id resolution to succeed");
        assert_eq!(cli_id, "config-cli-id");
    }

    #[test]
    fn test_resolve_cli_id_ignores_empty_env() {
        let _lock = ENV_LOCK.lock().expect("Failed to lock env mutex");
        let _guard = EnvGuard::new();

        let temp_home = tempdir().expect("Failed to create temp home dir");
        let config_path = temp_home.path().join(".popcorn.yaml");
        fs::write(config_path, "cli_id: config-cli-id\n").expect("Failed to write config");

        env::set_var("HOME", temp_home.path());
        env::set_var("POPCORN_SUBMITTER_ID", "   ");

        let cli_id = resolve_cli_id().expect("Expected cli_id resolution to succeed");
        assert_eq!(cli_id, "config-cli-id");
    }

    #[test]
    fn test_resolve_cli_id_errors_when_no_cli_id() {
        let _lock = ENV_LOCK.lock().expect("Failed to lock env mutex");
        let _guard = EnvGuard::new();

        let temp_home = tempdir().expect("Failed to create temp home dir");
        let config_path = temp_home.path().join(".popcorn.yaml");
        fs::write(config_path, "{}\n").expect("Failed to write config");

        env::set_var("HOME", temp_home.path());
        env::remove_var("POPCORN_SUBMITTER_ID");

        let err = resolve_cli_id().unwrap_err();
        assert!(err.to_string().contains("cli_id not found"));
    }
}
