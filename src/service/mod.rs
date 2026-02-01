use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use crate::models::{GpuItem, LeaderboardItem};

// Helper function to create a reusable reqwest client
pub fn create_client(cli_id: Option<String>) -> Result<Client> {
    let mut default_headers = HeaderMap::new();

    if let Some(id) = cli_id {
        match HeaderValue::from_str(&id) {
            Ok(val) => {
                default_headers.insert("X-Popcorn-Cli-Id", val);
            }
            Err(_) => {
                return Err(anyhow!("Invalid cli_id format for HTTP header"));
            }
        }
    }

    Client::builder()
        .timeout(Duration::from_secs(180))
        .default_headers(default_headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

pub async fn fetch_leaderboards(client: &Client) -> Result<Vec<LeaderboardItem>> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .get(format!("{}/leaderboards", base_url))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await?;
        return Err(anyhow!("Server returned status {}: {}", status, error_text));
    }

    let leaderboards: Vec<Value> = resp.json().await?;

    let mut leaderboard_items = Vec::new();
    for lb in leaderboards {
        let _task = lb["task"]
            .as_object()
            .ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        let name = lb["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        let description = lb["description"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid JSON structure"))?;

        leaderboard_items.push(LeaderboardItem::new(
            name.to_string(),
            description.to_string(),
        ));
    }

    Ok(leaderboard_items)
}

pub async fn fetch_gpus(client: &Client, leaderboard: &str) -> Result<Vec<GpuItem>> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .get(format!("{}/gpus/{}", base_url, leaderboard))
        .timeout(Duration::from_secs(120))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await?;
        return Err(anyhow!("Server returned status {}: {}", status, error_text));
    }

    let gpus: Vec<String> = resp.json().await?;

    let gpu_items = gpus.into_iter().map(GpuItem::new).collect();

    Ok(gpu_items)
}

pub async fn submit_solution<P: AsRef<Path>>(
    client: &Client,
    filepath: P,
    file_content: &str,
    leaderboard: &str,
    gpu: &str,
    submission_mode: &str,
    on_log: Option<Box<dyn Fn(String) + Send + Sync>>,
) -> Result<String> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let filename = filepath
        .as_ref()
        .file_name()
        .ok_or_else(|| anyhow!("Invalid filepath"))?
        .to_string_lossy();

    let part = Part::bytes(file_content.as_bytes().to_vec()).file_name(filename.to_string());

    let form = Form::new().part("file", part);

    let url = format!(
        "{}/{}/{}/{}",
        base_url,
        leaderboard.to_lowercase(),
        gpu,
        submission_mode.to_lowercase()
    );

    let resp = client
        .post(&url)
        .multipart(form)
        .timeout(Duration::from_secs(3600))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await?;
        let detail = serde_json::from_str::<Value>(&error_text)
            .ok()
            .and_then(|v| v.get("detail").and_then(|d| d.as_str()).map(str::to_string));

        return Err(anyhow!(
            "Server returned status {}: {}",
            status,
            detail.unwrap_or(error_text)
        ));
    }

    if resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.starts_with("text/event-stream"))
    {
        let mut resp = resp;
        let mut buffer = String::new();
        let mut stderr = tokio::io::stderr();

        while let Some(chunk) = resp.chunk().await? {
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find("\n\n") {
                let message_str = buffer.drain(..pos + 2).collect::<String>();
                let mut event_type = None;
                let mut data_json = None;

                for line in message_str.lines() {
                    if let Some(stripped) = line.strip_prefix("event:") {
                        event_type = Some(stripped.trim());
                    } else if let Some(stripped) = line.strip_prefix("data:") {
                        data_json = Some(stripped.trim());
                    }
                }

                if let (Some(event), Some(data)) = (event_type, data_json) {
                    match event {
                        "status" => {
                            if let Some(ref cb) = on_log {
                                // Try to parse as JSON and extract "message" or just return raw data
                                if let Ok(val) = serde_json::from_str::<Value>(data) {
                                    if let Some(msg) = val.get("message").and_then(|m| m.as_str()) {
                                        cb(msg.to_string());
                                    } else {
                                        cb(data.to_string());
                                    }
                                } else {
                                    cb(data.to_string());
                                }
                            }
                        }
                        "result" => {
                            let result_val: Value = serde_json::from_str(data)?;

                            if let Some(ref cb) = on_log {
                                // Handle "results" array
                                if let Some(results_array) =
                                    result_val.get("results").and_then(|v| v.as_array())
                                {
                                    let mode_key = submission_mode.to_lowercase();

                                    // Special handling for profile mode
                                    if mode_key == "profile" {
                                        for (i, result_item) in results_array.iter().enumerate() {
                                            if let Some(runs) =
                                                result_item.get("runs").and_then(|r| r.as_object())
                                            {
                                                for (key, run_data) in runs.iter() {
                                                    if key.starts_with("profile") {
                                                        handle_profile_result(cb, run_data, i);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        // Existing handling for non-profile modes
                                        for (i, result_item) in results_array.iter().enumerate() {
                                            if let Some(run_obj) = result_item
                                                .get("runs")
                                                .and_then(|r| r.get(&mode_key))
                                                .and_then(|t| t.get("run"))
                                            {
                                                if let Some(stdout) =
                                                    run_obj.get("stdout").and_then(|s| s.as_str())
                                                {
                                                    if !stdout.is_empty() {
                                                        cb(format!(
                                                            "STDOUT (Run {}):\n{}",
                                                            i + 1,
                                                            stdout
                                                        ));
                                                    }
                                                }
                                                // Also check stderr
                                                if let Some(stderr) =
                                                    run_obj.get("stderr").and_then(|s| s.as_str())
                                                {
                                                    if !stderr.is_empty() {
                                                        cb(format!(
                                                            "STDERR (Run {}):\n{}",
                                                            i + 1,
                                                            stderr
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Fallback for single object or different structure
                                    if let Some(stdout) =
                                        result_val.get("stdout").and_then(|s| s.as_str())
                                    {
                                        if !stdout.is_empty() {
                                            cb(format!("STDOUT:\n{}", stdout));
                                        }
                                    }
                                }
                            }

                            if let Some(reports) = result_val.get("reports") {
                                return Ok(reports.to_string());
                            } else {
                                // If no reports, return the whole result as a string
                                return Ok(serde_json::to_string_pretty(&result_val)?);
                            }
                        }
                        "error" => {
                            let error_val: Value = serde_json::from_str(data)?;
                            let detail = error_val
                                .get("detail")
                                .and_then(|d| d.as_str())
                                .unwrap_or("Unknown server error");
                            let status_code = error_val.get("status_code").and_then(|s| s.as_i64());
                            let raw_error = error_val.get("raw_error").and_then(|e| e.as_str());

                            let mut error_msg = format!("Server processing error: {}", detail);
                            if let Some(sc) = status_code {
                                error_msg.push_str(&format!(" (Status Code: {})", sc));
                            }
                            if let Some(re) = raw_error {
                                error_msg.push_str(&format!(" | Raw Error: {}", re));
                            }

                            return Err(anyhow!(error_msg));
                        }
                        _ => {
                            let msg = format!("Ignoring unknown SSE event: {}\n", event);
                            if let Some(ref cb) = on_log {
                                cb(msg.clone());
                            }
                            stderr.write_all(msg.as_bytes()).await?;
                            stderr.flush().await?;
                        }
                    }
                }
            }
        }
        Err(anyhow!(
            "Stream ended unexpectedly without a final result or error event."
        ))
    } else {
        let result: Value = resp.json().await?;
        let pretty_result = match result.get("results") {
            Some(result_obj) => serde_json::to_string_pretty(result_obj)?,
            None => return Err(anyhow!("Invalid non-streaming response structure")),
        };
        Ok(pretty_result)
    }
}

/// Handle profile mode results by decoding and displaying profile data,
/// and saving trace files to the current directory.
fn handle_profile_result(cb: &(dyn Fn(String) + Send + Sync), run_data: &Value, run_idx: usize) {
    // 1. Get profiler type and display it
    if let Some(profile) = run_data.get("profile") {
        let profiler = profile
            .get("profiler")
            .and_then(|p| p.as_str())
            .unwrap_or("Unknown");
        cb(format!("\n=== Profiler: {} ===", profiler));

        // 2. Decode and display profile report from run.result
        if let Some(run) = run_data.get("run") {
            // Display stdout/stderr if present
            if let Some(stdout) = run.get("stdout").and_then(|s| s.as_str()) {
                if !stdout.is_empty() {
                    cb(format!("STDOUT:\n{}", stdout));
                }
            }
            if let Some(stderr) = run.get("stderr").and_then(|s| s.as_str()) {
                if !stderr.is_empty() {
                    cb(format!("STDERR:\n{}", stderr));
                }
            }

            // Extract and decode profile report from result
            if let Some(result) = run.get("result").and_then(|r| r.as_object()) {
                let bench_count = result
                    .get("benchmark-count")
                    .and_then(|c| c.as_i64())
                    .unwrap_or(0);

                for i in 0..bench_count {
                    // Get benchmark spec
                    let spec_key = format!("benchmark.{}.spec", i);
                    let spec = result
                        .get(&spec_key)
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown");
                    cb(format!("\nBenchmark: {}", spec));

                    // Decode and display the profile report
                    let report_key = format!("benchmark.{}.report", i);
                    if let Some(encoded_report) = result.get(&report_key).and_then(|r| r.as_str()) {
                        match base64::engine::general_purpose::STANDARD.decode(encoded_report) {
                            Ok(decoded) => {
                                if let Ok(report_text) = String::from_utf8(decoded) {
                                    cb(format!("\n{}", report_text));
                                }
                            }
                            Err(e) => cb(format!("Failed to decode profile report: {}", e)),
                        }
                    }
                }
            }
        }

        // 3. Save trace file with unique timestamp
        if let Some(trace_b64) = profile.get("trace").and_then(|t| t.as_str()) {
            if !trace_b64.is_empty() {
                match base64::engine::general_purpose::STANDARD.decode(trace_b64) {
                    Ok(trace_data) => {
                        // Generate unique filename with timestamp and run index
                        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                        let filename = format!("profile_{}_run{}.zip", timestamp, run_idx);
                        match std::fs::write(&filename, &trace_data) {
                            Ok(_) => cb(format!("\nSaved profile trace to: {}", filename)),
                            Err(e) => cb(format!("Failed to save trace file: {}", e)),
                        }
                    }
                    Err(e) => cb(format!("Failed to decode trace data: {}", e)),
                }
            }
        }

        // 4. Show download URL if available
        if let Some(url) = profile.get("download_url").and_then(|u| u.as_str()) {
            if !url.is_empty() {
                cb(format!("Download full profile: {}", url));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client_without_cli_id() {
        let client = create_client(None);

        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_valid_cli_id() {
        let client = create_client(Some("valid-cli-id-123".to_string()));

        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_empty_cli_id() {
        let client = create_client(Some("".to_string()));

        assert!(client.is_ok());
    }

    #[test]
    fn test_create_client_with_invalid_header_chars() {
        // Headers cannot contain newlines or certain control characters
        let client = create_client(Some("invalid\nheader".to_string()));

        assert!(client.is_err());
        let err_msg = client.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid cli_id format"));
    }

    #[tokio::test]
    async fn test_fetch_leaderboards_missing_env_var() {
        // Temporarily unset the env var if set
        let original = std::env::var("POPCORN_API_URL").ok();
        std::env::remove_var("POPCORN_API_URL");

        let client = create_client(None).unwrap();
        let result = fetch_leaderboards(&client).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("POPCORN_API_URL"));

        // Restore original value if it existed
        if let Some(val) = original {
            std::env::set_var("POPCORN_API_URL", val);
        }
    }

    #[tokio::test]
    async fn test_fetch_gpus_missing_env_var() {
        let original = std::env::var("POPCORN_API_URL").ok();
        std::env::remove_var("POPCORN_API_URL");

        let client = create_client(None).unwrap();
        let result = fetch_gpus(&client, "test-leaderboard").await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("POPCORN_API_URL"));

        if let Some(val) = original {
            std::env::set_var("POPCORN_API_URL", val);
        }
    }

    #[tokio::test]
    async fn test_submit_solution_missing_env_var() {
        let original = std::env::var("POPCORN_API_URL").ok();
        std::env::remove_var("POPCORN_API_URL");

        let client = create_client(None).unwrap();
        let result = submit_solution(
            &client,
            "test.py",
            "print('hello')",
            "test-leaderboard",
            "H100",
            "test",
            None,
        )
        .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("POPCORN_API_URL"));

        if let Some(val) = original {
            std::env::set_var("POPCORN_API_URL", val);
        }
    }
}
