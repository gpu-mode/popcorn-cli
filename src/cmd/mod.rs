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

    /// Profile with Nsight Compute in your Modal account (default GPU: B200).
    #[arg(long, conflicts_with = "profile_brev")]
    pub profile: bool,

    /// Profile on the hosted GPU Mode Brev B200 and save Nsight Compute artifacts locally.
    /// Requires POPCORN_BREV_PROFILER_URL, or BREV_PROFILER_URL as a fallback.
    #[arg(long)]
    pub profile_brev: bool,

    /// Run the public evaluation in your own Modal account instead of submitting to GPU Mode.
    /// Uses Modal's configured profile or MODAL_TOKEN_ID/MODAL_TOKEN_SECRET.
    #[arg(long, conflicts_with = "profile_brev")]
    pub local: bool,

    #[command(flatten)]
    pub profile_options: crate::local::ProfileOptions,

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

        /// Do not print the submission's code
        #[arg(long)]
        no_code: bool,
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

        /// Profile with Nsight Compute in your Modal account (default GPU: B200).
        #[arg(long, conflicts_with = "profile_brev")]
        profile: bool,

        /// Profile on the hosted GPU Mode Brev B200 and save Nsight Compute artifacts locally.
        /// Requires POPCORN_BREV_PROFILER_URL, or BREV_PROFILER_URL as a fallback.
        #[arg(long)]
        profile_brev: bool,

        /// Run the public evaluation in your own Modal account instead of submitting to GPU Mode.
        /// Uses Modal's configured profile or MODAL_TOKEN_ID/MODAL_TOKEN_SECRET.
        #[arg(long, conflicts_with = "profile_brev")]
        local: bool,

        #[command(flatten)]
        profile_options: crate::local::ProfileOptions,

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
            profile_brev,
            profile,
            local,
            profile_options,
            output,
            no_tui,
        }) => {
            // Use filepath from Submit command first, fallback to top-level filepath
            let final_filepath = filepath.or(cli.filepath);
            let profile_brev = profile_brev || cli.profile_brev;
            let profile = profile || cli.profile;
            let local = local || cli.local;
            if profile_brev && (profile || local) {
                return Err(anyhow!(
                    "--profile-brev cannot be combined with --profile or --local"
                ));
            }
            let profile_options = profile_options.merge(cli.profile_options);
            let final_gpu = if profile_brev {
                Some("B200_Brev".to_string())
            } else {
                gpu.clone().or(cli.gpu.clone())
            };
            let final_mode = if profile_brev || profile {
                Some("profile".to_string())
            } else {
                mode.clone().or(cli.mode.clone())
            };

            profile_options.validate(final_mode.as_deref())?;
            if local || use_modal_profile(profile, profile_brev, final_mode.as_deref()) {
                submit::run_submit_local(
                    final_filepath,
                    final_gpu,
                    leaderboard.or(cli.leaderboard),
                    final_mode,
                    profile_options,
                    output,
                )
                .await
            } else {
                let config = load_config()?;
                let cli_id = config.cli_id.ok_or_else(|| {
                    anyhow!(
                        "cli_id not found in config file ({}). Please run 'popcorn-cli register' first.",
                        get_config_path().map_or_else(
                            |_| "unknown path".to_string(),
                            |p| p.display().to_string()
                        )
                    )
                })?;

                if no_tui || cli.no_tui || is_profile_mode(final_mode.as_deref()) {
                    submit::run_submit_plain(
                        final_filepath, // Resolved filepath
                        final_gpu,      // From Submit command
                        leaderboard,    // From Submit command
                        final_mode,     // From Submit command
                        cli_id,
                        profile_options,
                        output, // From Submit command
                    )
                    .await
                } else {
                    submit::run_submit_tui(
                        final_filepath, // Resolved filepath
                        final_gpu,      // From Submit command
                        leaderboard,    // From Submit command
                        final_mode,     // From Submit command
                        cli_id,
                        output, // From Submit command
                    )
                    .await
                }
            }
        }
        Some(Commands::Join { code }) => {
            let config = load_config()?;
            let cli_id = config.cli_id.ok_or_else(|| {
                anyhow!(
                    "cli_id not found in config file ({}). Please run `popcorn register` first.",
                    get_config_path()
                        .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
                )
            })?;
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
            let config = load_config()?;
            let cli_id = config.cli_id.ok_or_else(|| {
                anyhow!(
                    "cli_id not found in config file ({}). Please run `popcorn register` first.",
                    get_config_path()
                        .map_or_else(|_| "unknown path".to_string(), |p| p.display().to_string())
                )
            })?;

            match action {
                SubmissionsAction::List { leaderboard, limit } => {
                    submissions::list_submissions(cli_id, leaderboard, Some(limit)).await
                }
                SubmissionsAction::Show { id, no_code } => {
                    submissions::show_submission(cli_id, id, no_code).await
                }
                SubmissionsAction::Delete { id, force } => {
                    submissions::delete_submission(cli_id, id, force).await
                }
            }
        }
        None => {
            // Check if any of the submission-related flags were used at the top level
            if !cli.profile_brev
                && !cli.profile
                && !is_profile_mode(cli.mode.as_deref())
                && !cli.local
                && (cli.gpu.is_some() || cli.leaderboard.is_some() || cli.mode.is_some())
            {
                return Err(anyhow!(
                    "Please use the 'submit' subcommand when specifying submission options:\n\
                    popcorn-cli submit [--gpu GPU] [--leaderboard LEADERBOARD] [--mode MODE] FILEPATH"
                ));
            }

            // Handle the case where only a filepath is provided (for backward compatibility)
            if let Some(top_level_filepath) = cli.filepath {
                let mode = if cli.profile || cli.profile_brev {
                    Some("profile".to_string())
                } else {
                    cli.mode
                };
                cli.profile_options.validate(mode.as_deref())?;
                if cli.local || use_modal_profile(cli.profile, cli.profile_brev, mode.as_deref()) {
                    submit::run_submit_local(
                        Some(top_level_filepath),
                        cli.gpu,
                        cli.leaderboard,
                        mode,
                        cli.profile_options,
                        cli.output,
                    )
                    .await
                } else {
                    let config = load_config()?;
                    let cli_id = config.cli_id.ok_or_else(|| {
                        anyhow!(
                            "cli_id not found in config file ({}). Please run `popcorn register` first.",
                            get_config_path().map_or_else(
                                |_| "unknown path".to_string(),
                                |p| p.display().to_string()
                            )
                        )
                    })?;

                    if cli.profile_brev || is_profile_mode(mode.as_deref()) {
                        submit::run_submit_plain(
                            Some(top_level_filepath),
                            if cli.profile_brev {
                                Some("B200_Brev".to_string())
                            } else {
                                cli.gpu
                            },
                            cli.leaderboard,
                            Some("profile".to_string()),
                            cli_id,
                            cli.profile_options,
                            cli.output,
                        )
                        .await
                    } else {
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
                    }
                }
            } else {
                Err(anyhow!(
                    "No command or submission file specified. Use --help for usage."
                ))
            }
        }
    }
}

fn use_modal_profile(profile: bool, brev: bool, mode: Option<&str>) -> bool {
    !brev && (profile || is_profile_mode(mode))
}

fn is_profile_mode(mode: Option<&str>) -> bool {
    mode.is_some_and(|mode| mode.eq_ignore_ascii_case("profile"))
}

#[cfg(test)]
mod profile_tests {
    use super::*;

    #[test]
    fn profile_is_modal_and_brev_requires_its_explicit_flag() {
        assert!(use_modal_profile(true, false, None));
        assert!(use_modal_profile(false, false, Some("profile")));
        assert!(use_modal_profile(false, false, Some("PROFILE")));
        assert!(!use_modal_profile(false, true, Some("profile")));
        assert!(!use_modal_profile(false, false, Some("benchmark")));
    }

    #[test]
    fn profile_flags_accept_filters_at_both_entry_points() {
        for prefix in [vec!["popcorn"], vec!["popcorn", "submit"]] {
            for flag in ["--profile", "--profile-brev"] {
                let mut args = prefix.clone();
                args.extend([
                    "submission.py",
                    flag,
                    "--benchmark-index",
                    "3",
                    "--ncu-kernel-name",
                    "regex:custom",
                    "--ncu-launch-count",
                    "2",
                ]);
                assert!(Cli::try_parse_from(args).is_ok());
            }
        }
        assert!(Cli::try_parse_from([
            "popcorn",
            "submit",
            "submission.py",
            "--profile",
            "--profile-brev"
        ])
        .is_err());
        assert!(Cli::try_parse_from([
            "popcorn",
            "submit",
            "submission.py",
            "--profile",
            "--ncu-kernel-name-base",
            "invalid"
        ])
        .is_err());
    }
}
