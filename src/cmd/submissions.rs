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
        "ID", "Leaderboard", "File", "Time", "GPU(s)", "Status", "Score"
    );
    println!("{}", "-".repeat(105));

    // Print each submission
    for sub in submissions {
        let status = if sub.done { "done" } else { "pending" };

        // Collect all GPU types and best score from runs
        let gpus: Vec<&str> = sub.runs.iter().map(|r| r.gpu_type.as_str()).collect();
        let gpu_display = if gpus.is_empty() {
            "-".to_string()
        } else {
            gpus.join(",")
        };

        // Get best score (lowest).
        let best_score = sub
            .runs
            .iter()
            .filter_map(|r| r.score)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let score_display = format_score(best_score);

        let time = truncate(&sub.submission_time, 19);

        println!(
            "{:<8} {:<20} {:<20} {:<20} {:<12} {:<10} {:>10}",
            sub.id,
            truncate(&sub.leaderboard_name, 19),
            truncate(&sub.file_name, 19),
            time,
            truncate(&gpu_display, 11),
            status,
            score_display
        );
    }

    Ok(())
}

/// Show a specific submission with full details
pub async fn show_submission(cli_id: String, submission_id: i64, no_code: bool) -> Result<()> {
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
            let score_str = format_score(run.score);
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

    if !no_code {
        println!("\nCode:");
        println!("{}", "-".repeat(60));
        println!("{}", sub.code);
    }

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
    let result = service::delete_user_submission(&client, submission_id).await?;
    if result.get("status").and_then(|s| s.as_str()) == Some("ok") {
        println!("Submission {} deleted successfully.", submission_id);
    } else {
        println!("Submission deleted.");
    }

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

/// Render a score for display: full f64 precision (the shortest string that
/// round-trips) rather than a rounded `{:.4}`, or "-" when there is no score.
fn format_score(score: Option<f64>) -> String {
    score
        .map(|s| s.to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_score_full_precision() {
        // Full precision, not rounded to 4 decimals: two near-tied scores stay
        // distinguishable (the bug this replaces rendered both as "0.0017").
        assert_eq!(
            format_score(Some(0.001731805142084383)),
            "0.001731805142084383"
        );
        assert_eq!(
            format_score(Some(0.0017448536290123567)),
            "0.0017448536290123567"
        );
    }

    #[test]
    fn test_format_score_no_trailing_zeros() {
        // Shortest round-tripping form: no padding to 4 decimals.
        assert_eq!(format_score(Some(1.5)), "1.5");
        assert_eq!(format_score(Some(0.0)), "0");
    }

    #[test]
    fn test_format_score_none_is_dash() {
        assert_eq!(format_score(None), "-");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 19), "short");
        assert_eq!(truncate("submission.py", 19), "submission.py");
        assert_eq!(truncate("0123456789", 8), "01234...");
    }
}
