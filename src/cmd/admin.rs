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
    /// Generate invite codes for leaderboard(s)
    GenerateInvites {
        /// Leaderboard names to grant access to
        #[arg(long, required = true, num_args = 1..)]
        leaderboards: Vec<String>,

        /// Number of invite codes to generate (1-10000)
        #[arg(long, default_value = "1")]
        count: u32,
    },
    /// List invite codes for a leaderboard
    ListInvites {
        /// Leaderboard name
        leaderboard: String,
    },
    /// Revoke an invite code
    RevokeInvite {
        /// The invite code to revoke
        code: String,
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

        /// Set leaderboard visibility to closed (requires invite to access)
        #[arg(long)]
        closed: bool,
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
        AdminAction::GenerateInvites {
            leaderboards,
            count,
        } => {
            let result = service::admin_generate_invites(&client, &leaderboards, count).await?;
            let codes = result["codes"].as_array().map(|arr| arr.len()).unwrap_or(0);
            let lbs = result["leaderboards"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            println!("Generated {} invite code(s) for: {}", codes, lbs);
            if let Some(arr) = result["codes"].as_array() {
                for code in arr {
                    println!("  {}", code.as_str().unwrap_or("???"));
                }
            }
        }
        AdminAction::ListInvites { leaderboard } => {
            let result = service::admin_list_invites(&client, &leaderboard).await?;
            let invites = result["invites"].as_array();
            match invites {
                Some(arr) if arr.is_empty() => {
                    println!("No invites for '{}'", leaderboard);
                }
                Some(arr) => {
                    let claimed = arr
                        .iter()
                        .filter(|i| i["user_id"].as_str().is_some())
                        .count();
                    println!(
                        "Invites for '{}': {} total, {} claimed, {} unclaimed\n",
                        leaderboard,
                        arr.len(),
                        claimed,
                        arr.len() - claimed,
                    );
                    let header = format!(
                        "{:<26} {:<16} {:<20} {}",
                        "CODE", "STATUS", "CLAIMED BY", "CREATED"
                    );
                    println!("{header}");
                    println!("{}", "-".repeat(82));
                    for invite in arr {
                        let code = invite["code"].as_str().unwrap_or("???");
                        let user = invite["user_name"]
                            .as_str()
                            .or_else(|| invite["user_id"].as_str());
                        let status = if user.is_some() {
                            "claimed"
                        } else {
                            "unclaimed"
                        };
                        let user_display = user.unwrap_or("-");
                        let created = invite["created_at"].as_str().unwrap_or("-");
                        println!(
                            "{:<26} {:<16} {:<20} {}",
                            code, status, user_display, created,
                        );
                    }
                }
                None => {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }
        AdminAction::RevokeInvite { code } => {
            let result = service::admin_revoke_invite(&client, &code).await?;
            let was_claimed = result["was_claimed"].as_bool().unwrap_or(false);
            println!(
                "Revoked invite code '{}' (was {})",
                code,
                if was_claimed { "claimed" } else { "unclaimed" }
            );
        }
        AdminAction::UpdateProblems {
            problem_set,
            repository,
            branch,
            force,
            closed,
        } => {
            println!(
                "Updating problems from {}/tree/{}{}{}...",
                repository,
                branch,
                problem_set
                    .as_ref()
                    .map(|ps| format!(" (problem set: {})", ps))
                    .unwrap_or_default(),
                if closed { " (closed)" } else { "" },
            );
            let result = service::admin_update_problems(
                &client,
                problem_set.as_deref(),
                &repository,
                &branch,
                force,
                closed,
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
