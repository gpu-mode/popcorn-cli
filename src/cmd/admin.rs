use anyhow::{anyhow, Result};
use clap::Subcommand;
use std::env;

use crate::service;

#[derive(Subcommand, Debug)]
pub enum AdminAction {
    /// Start accepting jobs on the server
    Start,
    /// Stop accepting jobs on the server
    Stop,
    /// Get server statistics
    Stats {
        /// Only show stats for the last 24 hours
        #[arg(long)]
        last_day: bool,
    },
    /// Get a submission by ID
    GetSubmission {
        /// The submission ID to retrieve
        id: i64,
    },
    /// Delete a submission by ID
    DeleteSubmission {
        /// The submission ID to delete
        id: i64,
    },
    /// Create a dev leaderboard from a problem directory (requires gpus in task.yml)
    CreateLeaderboard {
        /// Problem directory name (e.g., "identity_py")
        directory: String,
    },
    /// Delete a leaderboard
    DeleteLeaderboard {
        /// Name of the leaderboard to delete
        name: String,
        /// Force deletion even if there are submissions
        #[arg(long)]
        force: bool,
    },
    /// Update problems from a GitHub repository (mirrors Discord /admin update-problems)
    UpdateProblems {
        /// Problem set name (e.g., "nvidia", "pmpp_v2"). If not specified, updates all.
        #[arg(long)]
        problem_set: Option<String>,

        /// Repository in format "owner/repo" (default: gpu-mode/reference-kernels)
        #[arg(long, default_value = "gpu-mode/reference-kernels")]
        repository: String,

        /// Branch to pull from (default: main)
        #[arg(long, default_value = "main")]
        branch: String,

        /// Force update even if task definition changed significantly
        #[arg(long)]
        force: bool,
    },
}

fn get_admin_token() -> Result<String> {
    env::var("POPCORN_ADMIN_TOKEN").map_err(|_| {
        anyhow!(
            "POPCORN_ADMIN_TOKEN environment variable is not set.\n\
            Set it to your admin token to use admin commands:\n\
            export POPCORN_ADMIN_TOKEN=your_token_here"
        )
    })
}

pub async fn handle_admin(action: AdminAction) -> Result<()> {
    let admin_token = get_admin_token()?;
    let client = service::create_admin_client(&admin_token)?;

    match action {
        AdminAction::Start => {
            let result = service::admin_start(&client).await?;
            println!("Server started accepting jobs");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::Stop => {
            let result = service::admin_stop(&client).await?;
            println!("Server stopped accepting jobs");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::Stats { last_day } => {
            let result = service::admin_stats(&client, last_day).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::GetSubmission { id } => {
            let result = service::admin_get_submission(&client, id).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::DeleteSubmission { id } => {
            let result = service::admin_delete_submission(&client, id).await?;
            println!("Deleted submission {}", id);
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::CreateLeaderboard { directory } => {
            let result = service::admin_create_leaderboard(&client, &directory).await?;
            let name = result["leaderboard"].as_str().unwrap_or(&directory);
            println!("Created leaderboard '{}'", name);
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::DeleteLeaderboard { name, force } => {
            let result = service::admin_delete_leaderboard(&client, &name, force).await?;
            println!("Deleted leaderboard '{}'", name);
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::UpdateProblems {
            problem_set,
            repository,
            branch,
            force,
        } => {
            println!(
                "Updating problems from {}/tree/{}{}...",
                repository,
                branch,
                problem_set
                    .as_ref()
                    .map(|ps| format!(" (problem set: {})", ps))
                    .unwrap_or_default()
            );
            let result = service::admin_update_problems(
                &client,
                problem_set.as_deref(),
                &repository,
                &branch,
                force,
            )
            .await?;

            // Pretty print the results
            if let Some(created) = result.get("created").and_then(|v| v.as_array()) {
                if !created.is_empty() {
                    println!("\nCreated {} leaderboard(s):", created.len());
                    for name in created {
                        println!("  + {}", name.as_str().unwrap_or("unknown"));
                    }
                }
            }
            if let Some(updated) = result.get("updated").and_then(|v| v.as_array()) {
                if !updated.is_empty() {
                    println!("\nUpdated {} leaderboard(s):", updated.len());
                    for name in updated {
                        println!("  ~ {}", name.as_str().unwrap_or("unknown"));
                    }
                }
            }
            if let Some(skipped) = result.get("skipped").and_then(|v| v.as_array()) {
                if !skipped.is_empty() {
                    println!("\nSkipped {} leaderboard(s):", skipped.len());
                    for item in skipped {
                        let name = item
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let reason = item
                            .get("reason")
                            .and_then(|r| r.as_str())
                            .unwrap_or("no changes");
                        println!("  - {} ({})", name, reason);
                    }
                }
            }
            if let Some(errors) = result.get("errors").and_then(|v| v.as_array()) {
                if !errors.is_empty() {
                    println!("\nErrors ({}):", errors.len());
                    for item in errors {
                        let name = item
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        let error = item
                            .get("error")
                            .and_then(|e| e.as_str())
                            .unwrap_or("unknown");
                        println!("  ! {}: {}", name, error);
                    }
                }
            }
        }
    }

    Ok(())
}
