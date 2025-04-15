use anyhow::{anyhow, Result};
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
        let task = lb["task"]
            .as_object()
            .ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        let name = lb["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        let description = task["description"]
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
        .timeout(Duration::from_secs(300))
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
                        "status" => (),
                        "result" => {
                            let result_val: Value = serde_json::from_str(data)?;
                            let pretty_result = match result_val.get("results") {
                                Some(result_obj) => serde_json::to_string_pretty(result_obj)?,
                                None => {
                                    return Err(anyhow!(
                                        "Invalid 'result' event structure: missing 'results' field"
                                    ))
                                }
                            };
                            return Ok(pretty_result);
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
                            stderr
                                .write_all(
                                    format!("Ignoring unknown SSE event: {}\n", event).as_bytes(),
                                )
                                .await?;
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
