use anyhow::Result;
use std::io::{self, Write};

use crate::service;

/// List user's submissions for a leaderboard
pub async fn list_submissions(
    cli_id: String,
    leaderboard: String,
    limit: Option<i32>,
) -> Result<()> {
    let client = service::create_client(Some(cli_id))?;
    let submissions = service::get_user_submissions(&client, Some(&leaderboard), limit).await?;

    if submissions.is_empty() {
        println!("No submissions found.");
        return Ok(());
    }

    // Print header
    println!(
        "{:<8} {:<20} {:<20} {:<20} {:<12} {:<10} {:>10}",
        "ID", "Leaderboard", "File", "Time", "GPU", "Status", "Score"
    );
    println!("{}", "-".repeat(105));

    // Print each submission
    for sub in submissions {
        let status = if sub.done { "done" } else { "pending" };
        let gpu = sub.gpu_type.as_deref().unwrap_or("-");
        let score = sub
            .score
            .map(|s| format!("{:.4}", s))
            .unwrap_or_else(|| "-".to_string());
        let time = truncate(&sub.submission_time, 19);

        println!(
            "{:<8} {:<20} {:<20} {:<20} {:<12} {:<10} {:>10}",
            sub.id,
            truncate(&sub.leaderboard_name, 19),
            truncate(&sub.file_name, 19),
            time,
            gpu,
            status,
            score
        );
    }

    Ok(())
}

/// Show a specific submission with full details
pub async fn show_submission(cli_id: String, submission_id: i64) -> Result<()> {
    let client = service::create_client(Some(cli_id))?;
    let sub = service::get_user_submission(&client, submission_id).await?;

    println!("Submission #{}", sub.id);
    println!("{}", "=".repeat(60));
    println!(
        "Leaderboard:    {} (id: {})",
        sub.leaderboard_name, sub.leaderboard_id
    );
    println!("File:           {}", sub.file_name);
    println!("User ID:        {}", sub.user_id);
    println!("Submitted:      {}", sub.submission_time);
    println!(
        "Status:         {}",
        if sub.done { "done" } else { "pending" }
    );

    if !sub.runs.is_empty() {
        println!("\nRuns:");
        for run in &sub.runs {
            let score_str = run
                .score
                .map(|s| format!("{:.4}", s))
                .unwrap_or_else(|| "-".to_string());
            let status = if run.passed { "passed" } else { "failed" };
            let secret_marker = if run.secret { " [secret]" } else { "" };
            let time_info = match (&run.start_time, &run.end_time) {
                (Some(start), Some(end)) => format!(" ({} - {})", start, end),
                (Some(start), None) => format!(" (started: {})", start),
                _ => String::new(),
            };
            println!(
                "  - {} on {}: {} (score: {}){}{}",
                run.mode, run.runner, status, score_str, secret_marker, time_info
            );
        }
    }

    println!("\nCode:");
    println!("{}", "-".repeat(60));
    println!("{}", sub.code);

    Ok(())
}

/// Delete a submission with confirmation
pub async fn delete_submission(cli_id: String, submission_id: i64, force: bool) -> Result<()> {
    let client = service::create_client(Some(cli_id))?;

    // Fetch submission first to show preview
    let sub = service::get_user_submission(&client, submission_id).await?;

    println!("Submission #{}", sub.id);
    println!("Leaderboard: {}", sub.leaderboard_name);
    println!("File:        {}", sub.file_name);
    println!("Submitted:   {}", sub.submission_time);

    // Show first 20 lines of code
    println!("\nCode preview:");
    println!("{}", "-".repeat(60));
    let lines: Vec<&str> = sub.code.lines().take(20).collect();
    for line in &lines {
        println!("{}", line);
    }
    if sub.code.lines().count() > 20 {
        println!("... ({} more lines)", sub.code.lines().count() - 20);
    }
    println!("{}", "-".repeat(60));

    // Ask for confirmation unless --force
    if !force {
        print!("\nDelete this submission? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Delete the submission
    service::delete_user_submission(&client, submission_id).await?;
    println!("Submission {} deleted successfully.", submission_id);

    Ok(())
}

/// Truncate a string to max length, adding "..." if truncated
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
