use anyhow::{Result, anyhow};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::path::Path;
use std::time::Duration;

use crate::models::{LeaderboardItem, GpuItem};

pub async fn fetch_leaderboards() -> Result<Vec<LeaderboardItem>> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;
    
    let client = Client::new();
    let resp = client
        .get(format!("{}/leaderboards", base_url))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!("Failed to fetch leaderboards: {}", status));
    }
    
    let leaderboards: Vec<Value> = resp.json().await?;
    
    let mut leaderboard_items = Vec::new();
    for lb in leaderboards {
        let task = lb["task"].as_object().ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        let name = lb["name"].as_str().ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        let description = task["description"].as_str().ok_or_else(|| anyhow!("Invalid JSON structure"))?;
        
        leaderboard_items.push(LeaderboardItem::new(
            name.to_string(),
            description.to_string(),
        ));
    }
    
    Ok(leaderboard_items)
}

pub async fn fetch_available_gpus(leaderboard: &str) -> Result<Vec<GpuItem>> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;
    
    let client = Client::new();
    let resp = client
        .get(format!("{}/gpus/{}", base_url, leaderboard))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    
    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await?;
        return Err(anyhow!("Server returned status {}: {}", status, error_text));
    }
    
    let gpus: Vec<String> = resp.json().await?;
    
    let gpu_items = gpus.into_iter()
        .map(|gpu| GpuItem::new(gpu))
        .collect();
    
    Ok(gpu_items)
}

pub async fn submit_solution<P: AsRef<Path>>(
    leaderboard: &str,
    gpu: &str,
    submission_mode: &str,
    filename: P,
    file_content: &[u8],
) -> Result<String> {
    let base_url = env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;
    
    let filename = filename.as_ref()
        .file_name()
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string_lossy();
    
    let part = Part::bytes(file_content.to_vec())
        .file_name(filename.to_string());
    
    let form = Form::new().part("file", part);
    
    let url = format!("{}/{}/{}/{}",
        base_url,
        leaderboard.to_lowercase(),
        gpu.to_lowercase(),
        submission_mode.to_lowercase()
    );
    
    let client = Client::new();
    let resp = client
        .post(&url)
        .multipart(form)
        .timeout(Duration::from_secs(60))
        .send()
        .await?;
    
    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await?;
        return Err(anyhow!("Server returned status {}: {}", status, error_text));
    }
    
    let result: Value = resp.json().await?;
    
    let pretty_result = match result.get("result") {
        Some(result_obj) => serde_json::to_string_pretty(result_obj)?,
        None => return Err(anyhow!("Invalid response structure")),
    };
    
    Ok(pretty_result)
}
