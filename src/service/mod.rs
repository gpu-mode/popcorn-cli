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

/// Create an HTTP client with admin token authentication
pub fn create_admin_client(admin_token: &str) -> Result<Client> {
    let mut default_headers = HeaderMap::new();

    let auth_value = format!("Bearer {}", admin_token);
    match HeaderValue::from_str(&auth_value) {
        Ok(val) => {
            default_headers.insert("Authorization", val);
        }
        Err(_) => {
            return Err(anyhow!("Invalid admin token format for HTTP header"));
        }
    }

    Client::builder()
        .timeout(Duration::from_secs(60))
        .default_headers(default_headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))
}

/// Start accepting jobs on the server
pub async fn admin_start(client: &Client) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .post(format!("{}/admin/start", base_url))
        .header("Content-Length", "0")
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Stop accepting jobs on the server
pub async fn admin_stop(client: &Client) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .post(format!("{}/admin/stop", base_url))
        .header("Content-Length", "0")
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Get server stats
pub async fn admin_stats(client: &Client, last_day_only: bool) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let url = if last_day_only {
        format!("{}/admin/stats?last_day_only=true", base_url)
    } else {
        format!("{}/admin/stats", base_url)
    };

    let resp = client
        .get(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Get a submission by ID
pub async fn admin_get_submission(client: &Client, submission_id: i64) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .get(format!("{}/admin/submissions/{}", base_url, submission_id))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Delete a submission by ID
pub async fn admin_delete_submission(client: &Client, submission_id: i64) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .delete(format!("{}/admin/submissions/{}", base_url, submission_id))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Create a leaderboard
pub async fn admin_create_leaderboard(
    client: &Client,
    name: &str,
    deadline: &str,
    directory: &str,
    gpu: &str,
) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let payload = serde_json::json!({
        "name": name,
        "deadline": deadline,
        "directory": directory,
        "gpu": gpu
    });

    let resp = client
        .post(format!("{}/admin/leaderboards", base_url))
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Delete a leaderboard
pub async fn admin_delete_leaderboard(client: &Client, leaderboard_name: &str, force: bool) -> Result<Value> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let url = if force {
        format!("{}/admin/leaderboards/{}?force=true", base_url, leaderboard_name)
    } else {
        format!("{}/admin/leaderboards/{}", base_url, leaderboard_name)
    };

    let resp = client
        .delete(url)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Helper to handle admin API responses
async fn handle_admin_response(resp: reqwest::Response) -> Result<Value> {
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
    resp.json().await.map_err(|e| anyhow!("Failed to parse response: {}", e))
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
        let task = lb["task"]
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

    let gpu_items = gpus.into_iter().map(|gpu| GpuItem::new(gpu)).collect();

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
        .map_or(false, |s| s.starts_with("text/event-stream"))
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
                    if line.starts_with("event:") {
                        event_type = Some(line["event:".len()..].trim());
                    } else if line.starts_with("data:") {
                        data_json = Some(line["data:".len()..].trim());
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
                                if let Some(results_array) = result_val.get("results").and_then(|v| v.as_array()) {
                                    let mode_key = submission_mode.to_lowercase();

                                    // Special handling for profile mode
                                    if mode_key == "profile" {
                                        for (i, result_item) in results_array.iter().enumerate() {
                                            if let Some(runs) = result_item.get("runs").and_then(|r| r.as_object()) {
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
                                            if let Some(run_obj) = result_item.get("runs")
                                                .and_then(|r| r.get(&mode_key))
                                                .and_then(|t| t.get("run"))
                                            {
                                                if let Some(stdout) = run_obj.get("stdout").and_then(|s| s.as_str()) {
                                                    if !stdout.is_empty() {
                                                        cb(format!("STDOUT (Run {}):\n{}", i + 1, stdout));
                                                    }
                                                }
                                                // Also check stderr
                                                if let Some(stderr) = run_obj.get("stderr").and_then(|s| s.as_str()) {
                                                    if !stderr.is_empty() {
                                                        cb(format!("STDERR (Run {}):\n{}", i + 1, stderr));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Fallback for single object or different structure
                                    if let Some(stdout) = result_val.get("stdout").and_then(|s| s.as_str()) {
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
fn handle_profile_result(
    cb: &Box<dyn Fn(String) + Send + Sync>,
    run_data: &Value,
    run_idx: usize,
) {
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
