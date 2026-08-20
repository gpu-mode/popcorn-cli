use std::io::ErrorKind;
use std::path::Path;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::service;

const LOCAL_RUNNER: &str = include_str!("../templates/local_modal_runner.py");
const RESULT_MARKER: &str = "POPCORN_LOCAL_RESULT=";

fn gpu_names(gpu: &str) -> Result<(&'static str, &'static str)> {
    match gpu.to_ascii_lowercase().as_str() {
        "t4" => Ok(("T4", "T4")),
        "l4" => Ok(("L4", "L4")),
        "l4x4" | "l4:4" => Ok(("L4x4", "L4:4")),
        "a100" | "a100-80gb" => Ok(("A100", "A100-80GB")),
        "h100" | "h100!" => Ok(("H100", "H100!")),
        "b200" => Ok(("B200", "B200")),
        _ => Err(anyhow!(
            "GPU '{}' is not supported by local Modal mode. Supported GPUs: T4, L4, L4x4, A100, H100, B200",
            gpu
        )),
    }
}

fn score_value(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn local_score(payload: &Value) -> Option<f64> {
    let run_result = payload.pointer("/result/runs/leaderboard/run/result")?;
    let count = score_value(run_result.get("benchmark-count"))? as usize;
    if count == 0 {
        return None;
    }

    let scores: Option<Vec<f64>> = (0..count)
        .map(|index| score_value(run_result.get(format!("benchmark.{}.mean", index))))
        .collect();
    let scores = scores?;
    let score_ns = match payload
        .get("ranking_by")
        .and_then(Value::as_str)
        .unwrap_or("last")
    {
        "last" if scores.len() == 1 => scores[0],
        "mean" => scores.iter().sum::<f64>() / scores.len() as f64,
        "geom" => (scores.iter().map(|score| score.ln()).sum::<f64>() / scores.len() as f64).exp(),
        _ => return None,
    };
    Some(score_ns / 1e9)
}

fn run_failure(run: &Value) -> Option<String> {
    let compilation = run.get("compilation").filter(|value| !value.is_null());
    if compilation
        .and_then(|value| value.get("success"))
        .and_then(Value::as_bool)
        == Some(false)
    {
        return Some(format!(
            "Compilation failed:\n{}",
            compilation
                .and_then(|value| value.get("stderr"))
                .and_then(Value::as_str)
                .unwrap_or("No compiler error was reported")
        ));
    }

    let result = run.get("run")?;
    if result.get("success").and_then(Value::as_bool) == Some(false) {
        return Some(format!(
            "Execution failed:\n{}",
            result
                .get("stderr")
                .and_then(Value::as_str)
                .unwrap_or("No execution error was reported")
        ));
    }
    None
}

fn format_local_result(payload: &Value) -> Result<String> {
    let result = payload
        .get("result")
        .ok_or_else(|| anyhow!("Modal runner returned no result"))?;
    if result.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(anyhow!(
            "Local Modal evaluation failed:\n{}",
            result
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("No error details were returned")
        ));
    }

    let mut sections = vec![format!(
        "Local Modal result for {} on {} (not submitted to gpumode.com)",
        payload
            .get("leaderboard")
            .and_then(Value::as_str)
            .unwrap_or("unknown leaderboard"),
        payload
            .get("gpu")
            .and_then(Value::as_str)
            .unwrap_or("unknown GPU")
    )];

    let runs = result
        .get("runs")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Modal runner returned no evaluation runs"))?;
    for (key, title, formatter) in [
        (
            "test",
            "Tests",
            service::format_test_rows as fn(&Value) -> Vec<String>,
        ),
        (
            "benchmark",
            "Benchmarks",
            service::format_benchmark_rows as fn(&Value) -> Vec<String>,
        ),
        (
            "leaderboard",
            "Ranked benchmarks",
            service::format_benchmark_rows as fn(&Value) -> Vec<String>,
        ),
    ] {
        let Some(run) = runs.get(key) else {
            continue;
        };
        if let Some(failure) = run_failure(run) {
            sections.push(format!("{}\n{}", title, failure));
            continue;
        }
        let rows = run
            .pointer("/run/result")
            .map(formatter)
            .unwrap_or_default();
        if !rows.is_empty() {
            sections.push(format!("{}\n{}", title, rows.join("\n\n")));
        }
    }

    if let Some(score) = local_score(payload) {
        let criterion = payload
            .get("ranking_by")
            .and_then(Value::as_str)
            .unwrap_or("ranked");
        sections.push(format!("Local {} score: {} s", criterion, score));
    }

    sections.push(format!(
        "Sources: reference-kernels {} · kernelbot {}",
        payload
            .get("reference_kernels_ref")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        payload
            .get("kernelbot_ref")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    ));
    Ok(sections.join("\n\n"))
}

pub async fn run_modal_submission(
    submission_path: &Path,
    leaderboard: &str,
    gpu: &str,
    mode: &str,
) -> Result<String> {
    if !matches!(
        mode.to_ascii_lowercase().as_str(),
        "test" | "benchmark" | "leaderboard"
    ) {
        return Err(anyhow!(
            "Local Modal mode supports test, benchmark, and leaderboard; got '{}'",
            mode
        ));
    }
    let (kernelbot_gpu, modal_gpu) = gpu_names(gpu)?;
    let helper = tempfile::Builder::new()
        .prefix("popcorn-local-modal-")
        .suffix(".py")
        .tempfile()
        .context("Failed to create the local Modal helper")?;
    std::fs::write(helper.path(), LOCAL_RUNNER)
        .context("Failed to write the local Modal helper")?;

    let submission_path = submission_path
        .canonicalize()
        .with_context(|| format!("Failed to resolve {}", submission_path.display()))?;
    let mut child = Command::new("modal")
        .arg("run")
        .arg(helper.path())
        .env("POPCORN_LOCAL_SUBMISSION", &submission_path)
        .env("POPCORN_LOCAL_LEADERBOARD", leaderboard)
        .env("POPCORN_LOCAL_GPU", kernelbot_gpu)
        .env("POPCORN_LOCAL_MODAL_GPU", modal_gpu)
        .env("POPCORN_LOCAL_MODE", mode.to_ascii_lowercase())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                anyhow!(
                    "The Modal CLI was not found. Install it with `python3 -m pip install modal`, then configure your token with `modal token set` or MODAL_TOKEN_ID/MODAL_TOKEN_SECRET."
                )
            } else {
                anyhow!("Failed to start Modal: {}", error)
            }
        })?;

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut payload = None;
        while let Some(line) = lines.next_line().await? {
            if let Some(index) = line.find(RESULT_MARKER) {
                payload = Some(line[index + RESULT_MARKER.len()..].to_string());
            } else {
                eprintln!("{}", line);
            }
        }
        Ok::<_, std::io::Error>(payload)
    });
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Some(line) = lines.next_line().await? {
            eprintln!("{}", line);
        }
        Ok::<_, std::io::Error>(())
    });

    let status = child
        .wait()
        .await
        .context("Failed while waiting for Modal")?;
    let payload = stdout_task.await.context("Modal stdout task failed")??;
    stderr_task.await.context("Modal stderr task failed")??;
    if !status.success() {
        return Err(anyhow!("Modal exited with status {}", status));
    }
    let payload = payload.ok_or_else(|| anyhow!("Modal returned no Popcorn result"))?;
    let payload: Value = serde_json::from_str(&payload).context("Modal returned invalid JSON")?;
    format_local_result(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_popcorn_gpu_names_to_modal() {
        assert_eq!(gpu_names("B200").unwrap(), ("B200", "B200"));
        assert_eq!(gpu_names("H100!").unwrap(), ("H100", "H100!"));
        assert_eq!(gpu_names("A100-80GB").unwrap(), ("A100", "A100-80GB"));
        assert_eq!(gpu_names("L4:4").unwrap(), ("L4x4", "L4:4"));
        assert!(gpu_names("MI300").is_err());
    }

    #[test]
    fn computes_geometric_mean_score_in_seconds() {
        let payload = serde_json::json!({
            "ranking_by": "geom",
            "result": {"runs": {"leaderboard": {"run": {"result": {
                "benchmark-count": 2,
                "benchmark.0.mean": 1_000_000,
                "benchmark.1.mean": 4_000_000
            }}}}}
        });
        let score = local_score(&payload).unwrap();
        assert!((score - 0.002).abs() < 1e-12);
    }

    #[test]
    fn formats_all_public_leaderboard_stages() {
        let payload = serde_json::json!({
            "leaderboard": "vectoradd_py",
            "gpu": "B200",
            "mode": "leaderboard",
            "ranking_by": "geom",
            "reference_kernels_ref": "abc",
            "kernelbot_ref": "def",
            "result": {
                "success": true,
                "error": "",
                "system": {},
                "runs": {
                    "test": {"compilation": null, "run": {"success": true, "passed": true, "result": {
                        "test-count": 1, "test.0.status": "pass", "test.0.spec": "size=128"
                    }}},
                    "benchmark": {"compilation": null, "run": {"success": true, "passed": true, "result": {
                        "benchmark-count": 1, "benchmark.0.spec": "size=1024", "benchmark.0.mean": 1000
                    }}},
                    "leaderboard": {"compilation": null, "run": {"success": true, "passed": true, "result": {
                        "benchmark-count": 1, "benchmark.0.spec": "size=1024", "benchmark.0.mean": 1000
                    }}}
                }
            }
        });
        let output = format_local_result(&payload).unwrap();
        assert!(output.contains("Tests\n✅ size=128"));
        assert!(output.contains("Benchmarks\nsize=1024"));
        assert!(output.contains("Ranked benchmarks\nsize=1024"));
        assert!(output.contains("not submitted to gpumode.com"));
    }
}
