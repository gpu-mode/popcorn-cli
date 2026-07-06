use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::fs::File as StdFile;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;
use zip::ZipArchive;

use crate::models::{
    GpuItem, LeaderboardItem, RunnerQueueStatus, SubmissionDetails, SubmissionJobStatus,
    SubmissionRun, UserSubmission, UserSubmissionRun,
};

const SUBMISSION_POLL_INTERVAL_SECONDS: u64 = 5;
const SUBMISSION_POLL_TIMEOUT_SECONDS: u64 = 60 * 60;

/// Parse a run's `score` field, which the server may send either as a JSON
/// number or as a JSON string (e.g. `0.0033` or `"0.0033"`). A plain
/// `Value::as_f64()` returns `None` for the string form, which is why scores
/// rendered as `-` in the submissions list/show views. Accept both forms.
fn parse_score(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn parse_runner_queue(value: Option<&Value>) -> Option<RunnerQueueStatus> {
    let queue = value?;
    if queue.is_null() {
        return None;
    }

    Some(RunnerQueueStatus {
        runner: queue
            .get("runner")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        gpu: queue
            .get("gpu")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        queued_jobs: queue.get("queued_jobs").and_then(|v| v.as_i64()),
    })
}

fn runner_queue_summary(queue: Option<&RunnerQueueStatus>) -> Option<String> {
    let queue = queue?;
    let queued_jobs = queue.queued_jobs?;
    let runner = queue.runner.as_deref().unwrap_or("Runner");
    let gpu = queue.gpu.as_deref().unwrap_or("selected GPU");
    let suffix = match queued_jobs {
        1 => "job",
        _ => "jobs",
    };

    Some(format!(
        "{} {} queue: {} queued {}",
        runner, gpu, queued_jobs, suffix
    ))
}

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
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

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
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

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
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

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
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .get(format!("{}/admin/submissions/{}", base_url, submission_id))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Delete a submission by ID
pub async fn admin_delete_submission(client: &Client, submission_id: i64) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .delete(format!("{}/admin/submissions/{}", base_url, submission_id))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Create a dev leaderboard from a problem directory
pub async fn admin_create_leaderboard(client: &Client, directory: &str) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let payload = serde_json::json!({
        "directory": directory
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
pub async fn admin_delete_leaderboard(
    client: &Client,
    leaderboard_name: &str,
    force: bool,
) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let url = if force {
        format!(
            "{}/admin/leaderboards/{}?force=true",
            base_url, leaderboard_name
        )
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

/// Update problems from a GitHub repository
pub async fn admin_update_problems(
    client: &Client,
    problem_set: Option<&str>,
    repository: &str,
    branch: &str,
    force: bool,
    closed: bool,
) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let mut payload = serde_json::json!({
        "repository": repository,
        "branch": branch,
        "force": force
    });

    if let Some(ps) = problem_set {
        payload["problem_set"] = serde_json::Value::String(ps.to_string());
    }

    if closed {
        payload["visibility"] = serde_json::Value::String("closed".to_string());
    }

    let resp = client
        .post(format!("{}/admin/update-problems", base_url))
        .json(&payload)
        .timeout(Duration::from_secs(120)) // Longer timeout for repo download
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Generate invite codes for one or more leaderboards
pub async fn admin_generate_invites(
    client: &Client,
    leaderboards: &[String],
    count: u32,
) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let payload = serde_json::json!({
        "leaderboards": leaderboards,
        "count": count,
    });

    let resp = client
        .post(format!("{}/admin/invites", base_url))
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// List invite codes for a leaderboard
pub async fn admin_list_invites(client: &Client, leaderboard_name: &str) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .get(format!(
            "{}/admin/leaderboards/{}/invites",
            base_url, leaderboard_name
        ))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    handle_admin_response(resp).await
}

/// Revoke an invite code
pub async fn admin_revoke_invite(client: &Client, code: &str) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .delete(format!("{}/admin/invites/{}", base_url, code))
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
    resp.json()
        .await
        .map_err(|e| anyhow!("Failed to parse response: {}", e))
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

/// Get the authenticated user's submissions
pub async fn get_user_submissions(
    client: &Client,
    leaderboard: Option<&str>,
    limit: Option<i32>,
) -> Result<Vec<UserSubmission>> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let mut url = format!("{}/user/submissions", base_url);
    let mut params = Vec::new();
    if let Some(lb) = leaderboard {
        params.push(format!("leaderboard={}", lb));
    }
    if let Some(l) = limit {
        params.push(format!("limit={}", l));
    }
    if !params.is_empty() {
        url = format!("{}?{}", url, params.join("&"));
    }

    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(30))
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

    let submissions: Vec<Value> = resp.json().await?;

    let mut result = Vec::new();
    for sub in submissions {
        let runs = sub["runs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|r| UserSubmissionRun {
                        gpu_type: r["gpu_type"].as_str().unwrap_or("").to_string(),
                        score: parse_score(&r["score"]),
                    })
                    .collect()
            })
            .unwrap_or_default();

        result.push(UserSubmission {
            id: sub["id"].as_i64().unwrap_or(0),
            leaderboard_name: sub["leaderboard_name"].as_str().unwrap_or("").to_string(),
            file_name: sub["file_name"].as_str().unwrap_or("").to_string(),
            submission_time: sub["submission_time"].as_str().unwrap_or("").to_string(),
            done: sub["done"].as_bool().unwrap_or(false),
            runs,
        });
    }

    Ok(result)
}

/// Get a specific submission by ID (with code)
pub async fn get_user_submission(client: &Client, submission_id: i64) -> Result<SubmissionDetails> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .get(format!("{}/user/submissions/{}", base_url, submission_id))
        .timeout(Duration::from_secs(30))
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

    let sub: Value = resp.json().await?;

    let runs = sub["runs"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|r| SubmissionRun {
                    start_time: r["start_time"].as_str().map(str::to_string),
                    end_time: r["end_time"].as_str().map(str::to_string),
                    mode: r["mode"].as_str().unwrap_or("").to_string(),
                    secret: r["secret"].as_bool().unwrap_or(false),
                    runner: r["runner"].as_str().unwrap_or("").to_string(),
                    score: parse_score(&r["score"]),
                    passed: r["passed"].as_bool().unwrap_or(false),
                    result: r.get("result").filter(|value| !value.is_null()).cloned(),
                })
                .collect()
        })
        .unwrap_or_default();

    let job = sub.get("job").and_then(|job| {
        if job.is_null() {
            None
        } else {
            Some(SubmissionJobStatus {
                status: job["status"].as_str().map(str::to_string),
                error: job["error"].as_str().map(str::to_string),
            })
        }
    });

    Ok(SubmissionDetails {
        id: sub["id"].as_i64().unwrap_or(0),
        leaderboard_id: sub["leaderboard_id"].as_i64().unwrap_or(0),
        leaderboard_name: sub["leaderboard_name"].as_str().unwrap_or("").to_string(),
        file_name: sub["file_name"].as_str().unwrap_or("").to_string(),
        user_id: sub["user_id"].as_str().unwrap_or("").to_string(),
        submission_time: sub["submission_time"].as_str().unwrap_or("").to_string(),
        done: sub["done"].as_bool().unwrap_or(false),
        code: sub["code"].as_str().unwrap_or("").to_string(),
        runs,
        job,
        runner_queue: parse_runner_queue(sub.get("runner_queue")),
    })
}

/// Delete a user's submission by ID
pub async fn delete_user_submission(client: &Client, submission_id: i64) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let resp = client
        .delete(format!("{}/user/submissions/{}", base_url, submission_id))
        .timeout(Duration::from_secs(30))
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

    resp.json()
        .await
        .map_err(|e| anyhow!("Failed to parse response: {}", e))
}

/// Claim an invite code to join closed leaderboard(s)
pub async fn join_with_invite(client: &Client, code: &str) -> Result<Value> {
    let base_url =
        env::var("POPCORN_API_URL").map_err(|_| anyhow!("POPCORN_API_URL is not set"))?;

    let payload = serde_json::json!({ "code": code });

    let resp = client
        .post(format!("{}/user/join", base_url))
        .json(&payload)
        .timeout(Duration::from_secs(30))
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

    resp.json()
        .await
        .map_err(|e| anyhow!("Failed to parse response: {}", e))
}

pub async fn submit_solution<P: AsRef<Path>>(
    client: &Client,
    filepath: P,
    file_content: &[u8],
    leaderboard: &str,
    gpu: &str,
    submission_mode: &str,
    on_log: Option<Box<dyn Fn(String) + Send + Sync>>,
) -> Result<String> {
    if submission_mode.eq_ignore_ascii_case("profile") {
        return submit_solution_streaming(
            client,
            filepath,
            file_content,
            leaderboard,
            gpu,
            submission_mode,
            on_log,
        )
        .await;
    }

    submit_solution_background(
        client,
        filepath,
        file_content,
        leaderboard,
        gpu,
        submission_mode,
        on_log,
    )
    .await
}

pub async fn profile_brev_solution<P: AsRef<Path>>(
    client: &Client,
    filepath: P,
    file_content: &[u8],
    leaderboard: &str,
    benchmark_index: Option<usize>,
    on_log: Option<Box<dyn Fn(String) + Send + Sync>>,
) -> Result<String> {
    let base_url = env::var("POPCORN_BREV_PROFILER_URL")
        .or_else(|_| env::var("BREV_PROFILER_URL"))
        .map_err(|_| {
            anyhow!(
                "POPCORN_BREV_PROFILER_URL or BREV_PROFILER_URL is not set. Configure a hardened Brev profiler endpoint before using --profile-brev."
            )
        })?;
    let base_url = base_url.trim_end_matches('/');

    let filename = filepath
        .as_ref()
        .file_name()
        .ok_or_else(|| anyhow!("Invalid filepath"))?
        .to_string_lossy();

    let part = Part::bytes(file_content.to_vec()).file_name(filename.to_string());
    let mut form = Form::new()
        .part("file", part)
        .text("leaderboard", leaderboard.to_string());
    if let Some(index) = benchmark_index {
        form = form.text("benchmark_index", index.to_string());
    }

    let resp = client
        .post(format!("{}/profile", base_url))
        .multipart(form)
        .timeout(Duration::from_secs(60))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow!(
            "Profiler returned status {}: {}",
            status,
            response_error_text(resp).await?
        ));
    }

    let accepted: Value = resp.json().await?;
    let job_id = accepted
        .get("job_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Profiler did not return a job_id"))?
        .to_string();

    if let Some(ref cb) = on_log {
        cb(format!(
            "Profile job {} accepted. Waiting for results...",
            job_id
        ));
    }

    let mut elapsed = 0;
    loop {
        let resp = match client
            .get(format!("{}/jobs/{}", base_url, job_id))
            .timeout(Duration::from_secs(30))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                if elapsed >= SUBMISSION_POLL_TIMEOUT_SECONDS {
                    return Err(err.into());
                }
                if let Some(ref cb) = on_log {
                    cb(format!(
                        "Profile job {} status poll failed: {}. Retrying...",
                        job_id, err
                    ));
                }
                sleep(Duration::from_secs(SUBMISSION_POLL_INTERVAL_SECONDS)).await;
                elapsed += SUBMISSION_POLL_INTERVAL_SECONDS;
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow!(
                "Profiler status returned {}: {}",
                status,
                response_error_text(resp).await?
            ));
        }

        let job: Value = resp.json().await?;
        let job_status = job
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let queue_position = job.get("queue_position").and_then(|v| v.as_i64());

        if let Some(ref cb) = on_log {
            match queue_position {
                Some(pos) => cb(format!(
                    "Profile job {} status: {} (queue position {}, {}s)",
                    job_id, job_status, pos, elapsed
                )),
                None => cb(format!(
                    "Profile job {} status: {} ({}s)",
                    job_id, job_status, elapsed
                )),
            }
        }

        match job_status {
            "succeeded" => {
                let artifacts = download_profile_artifacts(client, base_url, &job).await?;
                let mut result = job;
                result["downloaded_artifacts"] = Value::Array(
                    artifacts
                        .iter()
                        .map(|artifact| artifact.to_json())
                        .collect(),
                );
                return serde_json::to_string_pretty(&result)
                    .map_err(|e| anyhow!("Failed to format profile result: {}", e));
            }
            "failed" | "timed_out" => {
                let error = job
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No error details were provided");
                return Err(anyhow!("Profile job {} {}: {}", job_id, job_status, error));
            }
            _ => {}
        }

        if elapsed >= SUBMISSION_POLL_TIMEOUT_SECONDS {
            return Err(anyhow!(
                "Timed out waiting for profile job {} after {} seconds",
                job_id,
                SUBMISSION_POLL_TIMEOUT_SECONDS
            ));
        }

        sleep(Duration::from_secs(SUBMISSION_POLL_INTERVAL_SECONDS)).await;
        elapsed += SUBMISSION_POLL_INTERVAL_SECONDS;
    }
}

#[derive(Debug)]
struct DownloadedProfileArtifact {
    zip_path: PathBuf,
    details: Vec<PathBuf>,
    reports: Vec<PathBuf>,
}

impl DownloadedProfileArtifact {
    fn to_json(&self) -> Value {
        let details: Vec<Value> = self
            .details
            .iter()
            .map(|path| {
                serde_json::json!({
                    "path": path.display().to_string(),
                    "file_url": file_url(path),
                })
            })
            .collect();
        let reports: Vec<Value> = self
            .reports
            .iter()
            .map(|path| {
                serde_json::json!({
                    "path": path.display().to_string(),
                    "file_url": file_url(path),
                    "open_command": format!(
                        "open -a \"NVIDIA Nsight Compute\" {}",
                        shell_quote(&path.display().to_string())
                    ),
                    "ncu_ui_command": format!("ncu-ui {}", shell_quote(&path.display().to_string())),
                })
            })
            .collect();

        serde_json::json!({
            "zip_path": self.zip_path.display().to_string(),
            "details": details,
            "reports": reports,
        })
    }
}

async fn download_profile_artifacts(
    client: &Client,
    base_url: &str,
    job: &Value,
) -> Result<Vec<DownloadedProfileArtifact>> {
    let artifacts = job
        .get("artifacts")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("Profiler job did not include artifacts"))?;

    let mut saved = Vec::new();
    for artifact in artifacts {
        let name = artifact
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Profiler artifact missing name"))?;
        let url = artifact
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Profiler artifact missing url"))?;
        let artifact_url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("{}{}", base_url, url)
        };
        let bytes = client
            .get(artifact_url)
            .timeout(Duration::from_secs(120))
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        let zip_path = PathBuf::from(name);
        std::fs::write(&zip_path, bytes.as_ref())
            .map_err(|e| anyhow!("Failed to write profile artifact {}: {}", name, e))?;
        let extracted = extract_profile_artifacts(&zip_path, bytes.as_ref())?;
        saved.push(DownloadedProfileArtifact {
            zip_path,
            details: extracted.details,
            reports: extracted.reports,
        });
    }
    Ok(saved)
}

#[derive(Debug)]
struct ExtractedProfileArtifacts {
    details: Vec<PathBuf>,
    reports: Vec<PathBuf>,
}

fn extract_profile_artifacts(zip_path: &Path, bytes: &[u8]) -> Result<ExtractedProfileArtifacts> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| {
        anyhow!(
            "Failed to read profile artifact {}: {}",
            zip_path.display(),
            e
        )
    })?;
    let extract_dir = zip_path.with_extension("");
    std::fs::create_dir_all(&extract_dir).map_err(|e| {
        anyhow!(
            "Failed to create profile report directory {}: {}",
            extract_dir.display(),
            e
        )
    })?;

    let mut details = Vec::new();
    let mut reports = Vec::new();
    for idx in 0..archive.len() {
        let mut entry = archive.by_index(idx).map_err(|e| {
            anyhow!(
                "Failed to read profile artifact entry in {}: {}",
                zip_path.display(),
                e
            )
        })?;
        if !entry.name().ends_with(".ncu-rep")
            && !entry.name().ends_with("ncu-details.txt")
            && !entry.name().ends_with("ncu-details.csv")
        {
            continue;
        }

        let file_name = Path::new(entry.name())
            .file_name()
            .ok_or_else(|| anyhow!("Profile artifact contains an invalid report path"))?;
        let mut output_path = extract_dir.join(file_name);
        if output_path.exists() {
            let stem = output_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("profile");
            let extension = output_path
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| format!(".{}", ext))
                .unwrap_or_default();
            output_path = extract_dir.join(format!("{}-{}{}", stem, idx, extension));
        }

        let mut output = StdFile::create(&output_path)
            .map_err(|e| anyhow!("Failed to create {}: {}", output_path.display(), e))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|e| anyhow!("Failed to extract {}: {}", output_path.display(), e))?;
        if output_path.extension().and_then(|s| s.to_str()) == Some("ncu-rep") {
            reports.push(output_path);
        } else {
            details.push(output_path);
        }
    }
    Ok(ExtractedProfileArtifacts { details, reports })
}

fn file_url(path: &Path) -> String {
    let absolute = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string();
    format!(
        "file://{}",
        urlencoding::encode(&absolute).replace("%2F", "/")
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn response_error_text(resp: reqwest::Response) -> Result<String> {
    let text = resp.text().await?;
    Ok(serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| {
            v.get("detail")
                .or_else(|| v.get("message"))
                .and_then(|d| d.as_str())
                .map(str::to_string)
        })
        .unwrap_or(text))
}

async fn submit_solution_background<P: AsRef<Path>>(
    client: &Client,
    filepath: P,
    file_content: &[u8],
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

    let part = Part::bytes(file_content.to_vec()).file_name(filename.to_string());
    let form = Form::new().part("file", part);
    let url = format!(
        "{}/submission/{}/{}/{}",
        base_url,
        leaderboard.to_lowercase(),
        gpu,
        submission_mode.to_lowercase()
    );

    let resp = client
        .post(&url)
        .multipart(form)
        .timeout(Duration::from_secs(60))
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

    let accepted: Value = resp.json().await?;
    let submission_id = accepted
        .get("details")
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow!("Server did not return a submission id"))?;
    let accepted_runner_queue = parse_runner_queue(accepted.get("runner_queue"));

    if let Some(ref cb) = on_log {
        match runner_queue_summary(accepted_runner_queue.as_ref()) {
            Some(queue_summary) => cb(format!(
                "Submission {} accepted. {}. Waiting for results...",
                submission_id, queue_summary
            )),
            None => cb(format!(
                "Submission {} accepted. Waiting for results...",
                submission_id
            )),
        }
    }

    let mut elapsed = 0;
    loop {
        let details = get_user_submission(client, submission_id).await?;
        let job_status = details
            .job
            .as_ref()
            .and_then(|job| job.status.as_deref())
            .unwrap_or(if details.done { "done" } else { "pending" });

        if let Some(ref cb) = on_log {
            match runner_queue_summary(details.runner_queue.as_ref()) {
                Some(queue_summary) => cb(format!(
                    "Submission {} status: {} - {} ({}s)",
                    submission_id, job_status, queue_summary, elapsed
                )),
                None => cb(format!(
                    "Submission {} status: {} ({}s)",
                    submission_id, job_status, elapsed
                )),
            }
        }

        match job_status {
            "failed" | "timed_out" | "hacked" => {
                let error = details
                    .job
                    .as_ref()
                    .and_then(|job| job.error.as_deref())
                    .unwrap_or("No error details were provided");
                return Err(anyhow!(
                    "Submission {} {}: {}",
                    submission_id,
                    job_status,
                    error
                ));
            }
            _ => {}
        }

        if details.done {
            if let Some(error) = leaderboard_completion_error(&details, submission_mode, gpu) {
                return Err(anyhow!(error));
            }
            return format_submission_result(&details, submission_mode);
        }

        if elapsed >= SUBMISSION_POLL_TIMEOUT_SECONDS {
            return Err(anyhow!(
                "Timed out waiting for submission {} after {} seconds",
                submission_id,
                SUBMISSION_POLL_TIMEOUT_SECONDS
            ));
        }

        sleep(Duration::from_secs(SUBMISSION_POLL_INTERVAL_SECONDS)).await;
        elapsed += SUBMISSION_POLL_INTERVAL_SECONDS;
    }
}

fn result_count(result: &Value, key: &str) -> Option<usize> {
    result.get(key).and_then(|value| match value {
        Value::Number(number) => number
            .as_u64()
            .and_then(|count| usize::try_from(count).ok()),
        Value::String(text) => text.parse::<usize>().ok(),
        _ => None,
    })
}

fn result_text(result: &Value, key: &str) -> Option<String> {
    result.get(key).and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    })
}

fn format_time(nanoseconds: f64, error: Option<f64>) -> String {
    let (scale, unit) = if nanoseconds > 2_000_000.0 {
        (1_000_000.0, "ms")
    } else if nanoseconds > 2_000.0 {
        (1_000.0, "µs")
    } else {
        (1.0, "ns")
    };
    let value = nanoseconds / scale;
    let error = error
        .filter(|error| *error != 0.0)
        .map(|error| error / scale);

    match (value, error) {
        (value, Some(error)) if value < 1.0 => format!("{} ± {} {}", value, error, unit),
        (value, None) if value < 1.0 => format!("{} {}", value, unit),
        (value, Some(error)) if value < 10.0 => {
            format!("{:.2} ± {:.3} {}", value, error, unit)
        }
        (value, None) if value < 10.0 => format!("{:.2} {}", value, unit),
        (value, Some(error)) if value < 100.0 => {
            format!("{:.1} ± {:.2} {}", value, error, unit)
        }
        (value, None) if value < 100.0 => format!("{:.1} {}", value, unit),
        (value, Some(error)) => format!("{:.0} ± {:.1} {}", value, error, unit),
        (value, None) => format!("{:.0} {}", value, unit),
    }
}

fn format_test_rows(result: &Value) -> Vec<String> {
    let Some(count) = result_count(result, "test-count") else {
        return Vec::new();
    };

    (0..count)
        .filter_map(|index| {
            let status = result_text(result, &format!("test.{}.status", index))?;
            let spec = result_text(result, &format!("test.{}.spec", index))
                .unwrap_or_else(|| "<unknown test>".to_string());
            let mut row = match status.as_str() {
                "pass" => format!("✅ {}", spec),
                "fail" => format!("❌ {}", spec),
                _ => format!("? {} ({})", spec, status),
            };

            let detail_key = if status == "pass" { "message" } else { "error" };
            if let Some(detail) = result_text(result, &format!("test.{}.{}", index, detail_key)) {
                if !detail.is_empty() {
                    row.push_str("\n> ");
                    row.push_str(&detail.replace("\\n", "\n"));
                }
            }
            Some(row)
        })
        .collect()
}

fn format_benchmark_rows(result: &Value) -> Vec<String> {
    let Some(count) = result_count(result, "benchmark-count") else {
        return Vec::new();
    };

    (0..count)
        .map(|index| {
            let base = format!("benchmark.{}", index);
            let status = result_text(result, &format!("{}.status", base)).unwrap_or_default();
            let spec = result_text(result, &format!("{}.spec", base))
                .unwrap_or_else(|| "<unknown benchmark>".to_string());

            if status == "fail" {
                let error = result_text(result, &format!("{}.error", base))
                    .unwrap_or_else(|| "No error information available".to_string());
                return format!("❌ {} failed testing:\n{}", spec, error);
            }

            let mut row = spec;
            if let Some(mean) = result.get(format!("{}.mean", base)).and_then(parse_score) {
                let error = result.get(format!("{}.err", base)).and_then(parse_score);
                row.push_str(&format!("\n ⏱ {}", format_time(mean, error)));
            }
            let best = result.get(format!("{}.best", base)).and_then(parse_score);
            let worst = result.get(format!("{}.worst", base)).and_then(parse_score);
            if let (Some(best), Some(worst)) = (best, worst) {
                row.push_str(&format!(
                    "\n ⚡ {} 🐌 {}",
                    format_time(best, None),
                    format_time(worst, None)
                ));
            }
            row
        })
        .collect()
}

fn format_submission_rows(details: &SubmissionDetails, submission_mode: &str) -> Option<String> {
    let format_rows: fn(&Value) -> Vec<String> = if submission_mode.eq_ignore_ascii_case("test") {
        format_test_rows
    } else if submission_mode.eq_ignore_ascii_case("benchmark") {
        format_benchmark_rows
    } else {
        return None;
    };

    let sections: Vec<String> = details
        .runs
        .iter()
        .filter(|run| !run.secret && run.mode.eq_ignore_ascii_case(submission_mode))
        .filter_map(|run| {
            let rows = format_rows(run.result.as_ref()?);
            if rows.is_empty() {
                None
            } else {
                Some(rows.join("\n\n"))
            }
        })
        .collect();

    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn format_submission_result(details: &SubmissionDetails, submission_mode: &str) -> Result<String> {
    let summary = format_submission_details(details)?;
    match format_submission_rows(details, submission_mode) {
        Some(rows) => Ok(format!("{}\n\n{}", rows, summary)),
        None => Ok(summary),
    }
}

/// Build a human-readable summary of the geomean leaderboard score(s) for a
/// finished submission, or `None` if it has no scored `leaderboard` run.
///
/// The score the server reports for a `leaderboard`-mode run is the geometric
/// mean (in seconds) of the per-shape benchmark means — i.e. the number the
/// leaderboard ranks on. It is otherwise only visible buried in the runs JSON,
/// so surface it on its own line(s).
fn leaderboard_score_summary(details: &SubmissionDetails) -> Option<String> {
    let lines: Vec<String> = details
        .runs
        .iter()
        .filter(|run| run.mode == "leaderboard")
        .filter_map(|run| {
            run.score.map(|score| {
                let scope = if run.secret { " (secret)" } else { " (public)" };
                format!("Geomean score{} on {}: {} s", scope, run.runner, score)
            })
        })
        .collect();

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn same_gpu(run_gpu: &str, requested_gpu: &str) -> bool {
    run_gpu.eq_ignore_ascii_case(requested_gpu)
}

fn has_qualifying_leaderboard_result(details: &SubmissionDetails, gpu: &str) -> bool {
    let has_public_score = details.runs.iter().any(|run| {
        run.mode == "leaderboard"
            && !run.secret
            && same_gpu(&run.runner, gpu)
            && run.passed
            && run.score.is_some()
    });

    let has_secret_leaderboard_pass = details.runs.iter().any(|run| {
        run.mode == "leaderboard" && run.secret && same_gpu(&run.runner, gpu) && run.passed
    });

    let has_secret_failure = details
        .runs
        .iter()
        .any(|run| run.secret && same_gpu(&run.runner, gpu) && !run.passed);

    has_public_score && has_secret_leaderboard_pass && !has_secret_failure
}

fn leaderboard_completion_error(
    details: &SubmissionDetails,
    submission_mode: &str,
    gpu: &str,
) -> Option<String> {
    if !submission_mode.eq_ignore_ascii_case("leaderboard") {
        return None;
    }

    if has_qualifying_leaderboard_result(details, gpu) {
        return None;
    }

    if let Some(error) = details
        .job
        .as_ref()
        .and_then(|job| job.error.as_deref())
        .filter(|error| !error.is_empty())
    {
        return Some(format!("Submission {}: {}", details.id, error));
    }

    let secret_failed = details
        .runs
        .iter()
        .any(|run| run.secret && same_gpu(&run.runner, gpu) && !run.passed);

    if secret_failed {
        return Some(format!(
            "Submission {} failed secret validation on {}; it will not appear on the leaderboard",
            details.id, gpu
        ));
    }

    let has_public_leaderboard_score = details.runs.iter().any(|run| {
        run.mode == "leaderboard"
            && !run.secret
            && same_gpu(&run.runner, gpu)
            && run.passed
            && run.score.is_some()
    });

    if has_public_leaderboard_score {
        Some(format!(
            "Submission {} did not pass secret leaderboard validation on {}; it will not appear on the leaderboard",
            details.id, gpu
        ))
    } else {
        Some(format!(
            "Submission {} completed without a leaderboard score on {}; it will not appear on the leaderboard",
            details.id, gpu
        ))
    }
}

fn format_submission_details(details: &SubmissionDetails) -> Result<String> {
    let runs: Vec<Value> = details
        .runs
        .iter()
        .map(|run| {
            serde_json::json!({
                "mode": run.mode,
                "secret": run.secret,
                "runner": run.runner,
                "score": run.score,
                "passed": run.passed,
                "start_time": run.start_time,
                "end_time": run.end_time,
            })
        })
        .collect();

    let job = details.job.as_ref().map(|job| {
        serde_json::json!({
            "status": job.status,
            "error": job.error,
        })
    });

    let json = serde_json::to_string_pretty(&serde_json::json!({
        "submission_id": details.id,
        "leaderboard": details.leaderboard_name,
        "file_name": details.file_name,
        "done": details.done,
        "job": job,
        "runs": runs,
    }))
    .map_err(|e| anyhow!("Failed to format submission result: {}", e))?;

    // Lead with the geomean score so it is not lost in the runs JSON.
    match leaderboard_score_summary(details) {
        Some(summary) => Ok(format!("{}\n\n{}", summary, json)),
        None => Ok(json),
    }
}

async fn submit_solution_streaming<P: AsRef<Path>>(
    client: &Client,
    filepath: P,
    file_content: &[u8],
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

    let part = Part::bytes(file_content.to_vec()).file_name(filename.to_string());

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
                                                        handle_profile_result(cb, run_data, i, key);
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
fn handle_profile_result(
    cb: &(dyn Fn(String) + Send + Sync),
    run_data: &Value,
    result_idx: usize,
    run_key: &str,
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
                        match write_profile_trace_file(&trace_data, Utc::now(), result_idx, run_key)
                        {
                            Ok(filename) => cb(format!("\nSaved profile trace to: {}", filename)),
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

fn sanitize_profile_run_key(run_key: &str) -> String {
    let sanitized: String = run_key
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "profile".to_string()
    } else {
        sanitized
    }
}

fn build_profile_trace_filename(
    timestamp: DateTime<Utc>,
    result_idx: usize,
    run_key: &str,
) -> String {
    let run_key = sanitize_profile_run_key(run_key);
    format!(
        "profile_{}_result{}_{}.zip",
        timestamp.format("%Y%m%d_%H%M%S"),
        result_idx,
        run_key
    )
}

fn write_profile_trace_file(
    trace_data: &[u8],
    timestamp: DateTime<Utc>,
    result_idx: usize,
    run_key: &str,
) -> std::io::Result<String> {
    let filename = build_profile_trace_filename(timestamp, result_idx, run_key);
    std::fs::write(&filename, trace_data)?;
    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::tempdir;
    use tokio::io::AsyncReadExt;
    use tokio::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::const_new(());

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
        let _env_guard = ENV_LOCK.lock().await;
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
        let _env_guard = ENV_LOCK.lock().await;
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
        let _env_guard = ENV_LOCK.lock().await;
        let original = std::env::var("POPCORN_API_URL").ok();
        std::env::remove_var("POPCORN_API_URL");

        let client = create_client(None).unwrap();
        let result = submit_solution(
            &client,
            "test.py",
            b"print('hello')",
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

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected_len = None;

        loop {
            let bytes_read = stream.read(&mut buffer).await.unwrap();
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..bytes_read]);

            if expected_len.is_none() {
                if let Some(header_end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(str::trim)
                                .and_then(|value| value.parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    expected_len = Some(header_end + 4 + content_length);
                }
            }

            if expected_len.is_some_and(|len| request.len() >= len) {
                break;
            }
        }

        String::from_utf8(request).unwrap()
    }

    #[tokio::test]
    async fn test_standard_submission_modes_use_background_results_except_profile() {
        let _env_guard = ENV_LOCK.lock().await;
        let original = std::env::var("POPCORN_API_URL").ok();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        std::env::set_var("POPCORN_API_URL", &base_url);

        let server = tokio::spawn(async move {
            let mut request_lines = Vec::new();
            let mut pending_mode = None;
            for _ in 0..7 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut stream).await;
                let request_line = request.lines().next().unwrap().to_string();
                request_lines.push(request_line.clone());

                let (status, content_type, body) = if request_line.starts_with("POST /submission/")
                {
                    let path = request_line.split_whitespace().nth(1).unwrap();
                    pending_mode = path.rsplit('/').next().map(str::to_string);
                    (
                        "202 Accepted",
                        "application/json",
                        serde_json::json!({
                            "details": {"id": 42},
                            "runner_queue": null,
                            "status": "accepted",
                        })
                        .to_string(),
                    )
                } else if request_line.starts_with("GET /user/submissions/42") {
                    let mode = pending_mode.take().unwrap();
                    let runs = match mode.as_str() {
                        "test" => serde_json::json!([{
                            "start_time": null,
                            "end_time": null,
                            "mode": "test",
                            "secret": false,
                            "runner": "H100",
                            "score": null,
                            "passed": true,
                            "result": {
                                "test-count": 2,
                                "test.0.status": "pass",
                                "test.0.spec": "test-row-one",
                                "test.1.status": "pass",
                                "test.1.spec": "test-row-two",
                            },
                        }]),
                        "benchmark" => serde_json::json!([{
                            "start_time": null,
                            "end_time": null,
                            "mode": "benchmark",
                            "secret": false,
                            "runner": "H100",
                            "score": null,
                            "passed": true,
                            "result": {
                                "benchmark-count": "2",
                                "benchmark.0.status": "pass",
                                "benchmark.0.spec": "shape-one",
                                "benchmark.0.mean": "1500",
                                "benchmark.1.status": "pass",
                                "benchmark.1.spec": "shape-two",
                                "benchmark.1.mean": "2500",
                            },
                        }]),
                        "leaderboard" => serde_json::json!([
                            {
                                "start_time": null,
                                "end_time": null,
                                "mode": "leaderboard",
                                "secret": false,
                                "runner": "H100",
                                "score": 0.0066,
                                "passed": true,
                            },
                            {
                                "start_time": null,
                                "end_time": null,
                                "mode": "leaderboard",
                                "secret": true,
                                "runner": "H100",
                                "score": null,
                                "passed": true,
                            },
                        ]),
                        _ => unreachable!(),
                    };
                    (
                        "200 OK",
                        "application/json",
                        serde_json::json!({
                            "id": 42,
                            "leaderboard_id": 7,
                            "leaderboard_name": "test-leaderboard",
                            "file_name": "solution.py",
                            "user_id": "user",
                            "submission_time": "2026-07-06T12:00:00Z",
                            "done": true,
                            "code": "print('hello')",
                            "runs": runs,
                            "job": {"status": "succeeded", "error": null},
                            "runner_queue": null,
                        })
                        .to_string(),
                    )
                } else {
                    (
                        "200 OK",
                        "text/event-stream",
                        concat!(
                            "event: result\n",
                            "data: {\"status\":\"success\",\"results\":[],",
                            "\"reports\":[\"profile complete\"]}\n\n"
                        )
                        .to_string(),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    content_type,
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            request_lines
        });

        let client = create_client(None).unwrap();
        let submit = |mode| {
            submit_solution(
                &client,
                "solution.py",
                b"print('hello')",
                "test-leaderboard",
                "H100",
                mode,
                None,
            )
        };

        let test_result = submit("test").await.unwrap();
        assert!(
            test_result.find("test-row-one").unwrap() < test_result.find("test-row-two").unwrap()
        );

        let benchmark_result = submit("benchmark").await.unwrap();
        assert!(
            benchmark_result.find("shape-one").unwrap()
                < benchmark_result.find("shape-two").unwrap()
        );

        let leaderboard_result = submit("leaderboard").await.unwrap();
        assert!(leaderboard_result.contains("Geomean score (public) on H100: 0.0066 s"));

        let profile_result = submit("profile").await.unwrap();
        assert_eq!(profile_result, r#"["profile complete"]"#);

        let request_lines = server.await.unwrap();
        assert_eq!(
            request_lines,
            [
                "POST /submission/test-leaderboard/H100/test HTTP/1.1",
                "GET /user/submissions/42 HTTP/1.1",
                "POST /submission/test-leaderboard/H100/benchmark HTTP/1.1",
                "GET /user/submissions/42 HTTP/1.1",
                "POST /submission/test-leaderboard/H100/leaderboard HTTP/1.1",
                "GET /user/submissions/42 HTTP/1.1",
                "POST /test-leaderboard/H100/profile HTTP/1.1",
            ]
        );

        match original {
            Some(val) => std::env::set_var("POPCORN_API_URL", val),
            None => std::env::remove_var("POPCORN_API_URL"),
        }
    }

    #[test]
    fn test_build_profile_trace_filename_uses_result_index_and_run_key() {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 3, 27, 9, 38, 46)
            .single()
            .unwrap();

        let filename = build_profile_trace_filename(timestamp, 0, "profile3");

        assert_eq!(filename, "profile_20260327_093846_result0_profile3.zip");
    }

    #[test]
    fn test_build_profile_trace_filename_sanitizes_run_key() {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 3, 27, 9, 38, 46)
            .single()
            .unwrap();

        let filename = build_profile_trace_filename(timestamp, 1, "profile:1/a b");

        assert_eq!(
            filename,
            "profile_20260327_093846_result1_profile_1_a_b.zip"
        );
    }

    #[test]
    fn test_build_profile_trace_filename_uses_default_run_key_when_empty() {
        let timestamp = Utc
            .with_ymd_and_hms(2026, 3, 27, 9, 38, 46)
            .single()
            .unwrap();

        let filename = build_profile_trace_filename(timestamp, 2, "");

        assert_eq!(filename, "profile_20260327_093846_result2_profile.zip");
    }

    #[test]
    fn test_write_profile_trace_file_writes_expected_contents() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        let timestamp = Utc
            .with_ymd_and_hms(2026, 3, 27, 9, 38, 46)
            .single()
            .unwrap();
        let trace_data = b"trace-bytes";

        std::env::set_current_dir(temp_dir.path()).unwrap();

        let filename = write_profile_trace_file(trace_data, timestamp, 3, "profile/3").unwrap();
        let written_path = temp_dir.path().join(&filename);

        assert_eq!(filename, "profile_20260327_093846_result3_profile_3.zip");
        assert_eq!(std::fs::read(&written_path).unwrap(), trace_data);

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_parse_score_accepts_number_and_string() {
        use serde_json::json;

        // The server sends score as a JSON string; older code only handled
        // numbers, so string scores rendered as `-`. Both must parse now.
        assert_eq!(parse_score(&json!("0.0033")), Some(0.0033));
        assert_eq!(parse_score(&json!(0.0033)), Some(0.0033));

        // Absent / null / non-numeric scores stay None.
        assert_eq!(parse_score(&json!(null)), None);
        assert_eq!(parse_score(&json!("")), None);
        assert_eq!(parse_score(&json!("not-a-number")), None);
        assert_eq!(parse_score(&Value::Null), None);
    }

    fn run(mode: &str, secret: bool, score: Option<f64>) -> SubmissionRun {
        SubmissionRun {
            start_time: None,
            end_time: None,
            mode: mode.to_string(),
            secret,
            runner: "B200".to_string(),
            score,
            passed: true,
            result: None,
        }
    }

    fn failed_run(mode: &str, secret: bool, score: Option<f64>) -> SubmissionRun {
        let mut run = run(mode, secret, score);
        run.passed = false;
        run
    }

    fn details(runs: Vec<SubmissionRun>) -> SubmissionDetails {
        SubmissionDetails {
            id: 1,
            leaderboard_id: 1,
            leaderboard_name: "qr_v2".to_string(),
            file_name: "submission.py".to_string(),
            user_id: "u".to_string(),
            submission_time: String::new(),
            done: true,
            code: String::new(),
            runs,
            job: None,
            runner_queue: None,
        }
    }

    #[test]
    fn test_format_test_rows_preserves_index_order_and_details() {
        let result = serde_json::json!({
            "test-count": "2",
            "test.0.status": "pass",
            "test.0.spec": "first test",
            "test.0.message": "worked\\nas expected",
            "test.1.status": "fail",
            "test.1.spec": "second test",
            "test.1.error": "mismatch",
        });

        let rows = format_test_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains("✅ first test"));
        assert!(rows[0].contains("worked\nas expected"));
        assert!(rows[1].contains("❌ second test"));
        assert!(rows[1].contains("mismatch"));
    }

    #[test]
    fn test_format_benchmark_rows_preserves_index_order_and_timings() {
        let result = serde_json::json!({
            "benchmark-count": 2,
            "benchmark.0.status": "pass",
            "benchmark.0.spec": "first shape",
            "benchmark.0.mean": "2500",
            "benchmark.0.err": 100,
            "benchmark.0.best": "2200",
            "benchmark.0.worst": 2800,
            "benchmark.1.status": "fail",
            "benchmark.1.spec": "second shape",
            "benchmark.1.error": "timeout",
        });

        let rows = format_benchmark_rows(&result);

        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains("first shape"));
        assert!(rows[0].contains("2.50 ± 0.100 µs"));
        assert!(rows[0].contains("2.20 µs"));
        assert!(rows[0].contains("2.80 µs"));
        assert!(rows[1].contains("❌ second shape failed testing"));
        assert!(rows[1].contains("timeout"));
    }

    #[test]
    fn test_format_submission_rows_ignores_secret_results() {
        let mut public = run("benchmark", false, None);
        public.result = Some(serde_json::json!({
            "benchmark-count": 1,
            "benchmark.0.status": "pass",
            "benchmark.0.spec": "public shape",
        }));
        let mut secret = run("benchmark", true, None);
        secret.result = Some(serde_json::json!({
            "benchmark-count": 1,
            "benchmark.0.status": "pass",
            "benchmark.0.spec": "SECRET_SENTINEL",
        }));

        let output = format_submission_rows(&details(vec![public, secret]), "benchmark").unwrap();

        assert!(output.contains("public shape"));
        assert!(!output.contains("SECRET_SENTINEL"));
    }

    #[test]
    fn test_format_submission_result_falls_back_without_detailed_result() {
        let output =
            format_submission_result(&details(vec![run("benchmark", false, None)]), "benchmark")
                .unwrap();

        assert!(output.starts_with('{'));
        assert!(output.contains("\"mode\": \"benchmark\""));
    }

    #[test]
    fn test_runner_queue_summary_reports_queued_jobs() {
        let queue = RunnerQueueStatus {
            runner: Some("Modal".to_string()),
            gpu: Some("B200".to_string()),
            queued_jobs: Some(5),
        };

        assert_eq!(
            runner_queue_summary(Some(&queue)),
            Some("Modal B200 queue: 5 queued jobs".to_string())
        );
    }

    #[test]
    fn test_leaderboard_score_summary_reports_geomean_scores() {
        let details = details(vec![
            run("test", false, None),
            run("benchmark", false, None),
            run("leaderboard", false, Some(0.0066)),
            run("leaderboard", true, Some(0.0018)),
        ]);
        let summary = leaderboard_score_summary(&details).expect("expected a score summary");
        assert_eq!(
            summary,
            "Geomean score (public) on B200: 0.0066 s\n\
             Geomean score (secret) on B200: 0.0018 s"
        );

        let formatted = format_submission_details(&details).unwrap();
        assert!(formatted.starts_with("Geomean score (public) on B200: 0.0066 s"));
    }

    #[test]
    fn test_leaderboard_score_summary_none_without_scored_leaderboard_run() {
        let details = details(vec![
            run("test", false, None),
            run("benchmark", false, Some(0.5)),
            run("leaderboard", false, None),
        ]);
        assert!(leaderboard_score_summary(&details).is_none());
        assert!(format_submission_details(&details)
            .unwrap()
            .starts_with('{'));
    }

    #[test]
    fn test_leaderboard_completion_error_none_for_qualified_submission() {
        let details = details(vec![
            run("leaderboard", false, Some(0.0066)),
            run("leaderboard", true, Some(0.0018)),
        ]);

        assert!(leaderboard_completion_error(&details, "leaderboard", "B200").is_none());
    }

    #[test]
    fn test_leaderboard_completion_error_for_secret_failure() {
        let details = details(vec![
            run("leaderboard", false, Some(0.0066)),
            failed_run("benchmark", true, None),
        ]);

        let error = leaderboard_completion_error(&details, "leaderboard", "B200")
            .expect("expected secret validation failure");

        assert!(error.contains("failed secret validation"));
        assert!(error.contains("will not appear on the leaderboard"));
    }

    #[test]
    fn test_leaderboard_completion_error_for_missing_secret_leaderboard() {
        let details = details(vec![run("leaderboard", false, Some(0.0066))]);

        let error = leaderboard_completion_error(&details, "leaderboard", "B200")
            .expect("expected missing secret validation failure");

        assert!(error.contains("did not pass secret leaderboard validation"));
        assert!(error.contains("will not appear on the leaderboard"));
    }

    #[test]
    fn test_leaderboard_completion_error_for_missing_public_score() {
        let details = details(vec![run("leaderboard", false, None)]);

        let error = leaderboard_completion_error(&details, "leaderboard", "B200")
            .expect("expected missing score failure");

        assert!(error.contains("completed without a leaderboard score"));
        assert!(error.contains("will not appear on the leaderboard"));
    }

    #[test]
    fn test_leaderboard_completion_error_ignores_non_leaderboard_modes() {
        let details = details(vec![run("test", false, None)]);

        assert!(leaderboard_completion_error(&details, "test", "B200").is_none());
    }
}
