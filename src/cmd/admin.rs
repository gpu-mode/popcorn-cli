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
    /// Create a new leaderboard
    CreateLeaderboard {
        /// Name of the leaderboard
        #[arg(long)]
        name: String,
        /// Deadline in YYYY-MM-DD or YYYY-MM-DD HH:MM format
        #[arg(long)]
        deadline: String,
        /// Problem directory (relative to PROBLEM_DEV_DIR)
        #[arg(long)]
        directory: String,
        /// GPU type for this leaderboard
        #[arg(long)]
        gpu: String,
    },
    /// Delete a leaderboard
    DeleteLeaderboard {
        /// Name of the leaderboard to delete
        name: String,
        /// Force deletion even if there are submissions
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
        AdminAction::CreateLeaderboard {
            name,
            deadline,
            directory,
            gpu,
        } => {
            let result =
                service::admin_create_leaderboard(&client, &name, &deadline, &directory, &gpu)
                    .await?;
            println!("Created leaderboard '{}'", name);
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        AdminAction::DeleteLeaderboard { name, force } => {
            let result = service::admin_delete_leaderboard(&client, &name, force).await?;
            println!("Deleted leaderboard '{}'", name);
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
