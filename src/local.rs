use std::io::ErrorKind;
use std::path::Path;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::service;

const LOCAL_RUNNER: &str = include_str!("../templates/local_modal_runner.py");
const RESULT_MARKER: &str = "POPCORN_LOCAL_RESULT=";

#[derive(clap::Args, Debug, Default, Clone, Serialize)]
pub struct ProfileOptions {
    /// Profile benchmarks[N]; omit to profile all benchmark shapes.
    #[arg(long)]
    pub benchmark_index: Option<usize>,
    /// Capture only matching NCU kernel names (supports regex: expressions).
    #[arg(long)]
    pub ncu_kernel_name: Option<String>,
    /// How NCU interprets kernel names.
    #[arg(long, value_parser = ["function", "demangled", "mangled"])]
    pub ncu_kernel_name_base: Option<String>,
    /// Maximum matching kernel launches to capture per benchmark (default: 10).
    #[arg(long)]
    pub ncu_launch_count: Option<usize>,
}

impl ProfileOptions {
    pub fn merge(self, fallback: Self) -> Self {
        Self {
            benchmark_index: self.benchmark_index.or(fallback.benchmark_index),
            ncu_kernel_name: self.ncu_kernel_name.or(fallback.ncu_kernel_name),
            ncu_kernel_name_base: self.ncu_kernel_name_base.or(fallback.ncu_kernel_name_base),
            ncu_launch_count: self.ncu_launch_count.or(fallback.ncu_launch_count),
        }
    }

    pub fn validate(&self, mode: Option<&str>) -> Result<()> {
        if self.ncu_launch_count == Some(0) {
            return Err(anyhow!("--ncu-launch-count must be greater than zero"));
        }
        if (self.benchmark_index.is_some()
            || self.ncu_kernel_name.is_some()
            || self.ncu_kernel_name_base.is_some()
            || self.ncu_launch_count.is_some())
            && !mode.is_some_and(|mode| mode.eq_ignore_ascii_case("profile"))
        {
            return Err(anyhow!(
                "Profiling options require --profile, --profile-brev, or --mode profile"
            ));
        }
        Ok(())
    }
}

fn gpu_names(gpu: &str) -> Result<(&'static str, &'static str)> {
    match gpu.to_ascii_lowercase().as_str() {
        "t4" => Ok(("T4", "T4")),
        "l4" => Ok(("L4", "L4")),
        "l4x4" | "l4:4" => Ok(("L4x4", "L4:4")),
        "a100" | "a100-80gb" => Ok(("A100", "A100-80GB")),
        "h100" | "h100!" => Ok(("H100", "H100!")),
        "b200" => Ok(("B200", "B200")),
        "b200_brev" => Err(anyhow!("Brev profiling requires --profile-brev; --profile uses Modal only")),
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

fn save_profile_result(payload: &Value, output_dir: &Path) -> Result<String> {
    let result = &payload["result"];
    if result["success"].as_bool() != Some(true) {
        return Err(anyhow!("Modal profiling failed: {}", result["error"]));
    }
    let runs = result["runs"]
        .as_object()
        .filter(|runs| !runs.is_empty())
        .ok_or_else(|| anyhow!("Modal returned no profile runs"))?;
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "leaderboard": payload["leaderboard"],
            "gpu": payload["gpu"],
            "system": result["system"],
            "problem_directory": payload["problem_directory"],
            "reference_kernels_ref": payload["reference_kernels_ref"],
            "kernelbot_ref": payload["kernelbot_ref"],
            "profile_options": payload["profile_options"],
            "benchmark_specs": payload["benchmark_specs"],
        }))?,
    )?;
    let mut lines = vec![format!(
        "Modal Nsight Compute profile: {} on {}",
        payload["leaderboard"].as_str().unwrap_or("unknown"),
        payload["gpu"].as_str().unwrap_or("unknown")
    )];
    let mut failures = Vec::new();
    for (index, (key, run)) in runs.iter().enumerate() {
        if let Some(failure) = run_failure(run) {
            failures.push(format!("{}: {}", key, failure));
        }
        let Some(trace) = run.pointer("/profile/trace").and_then(Value::as_str) else {
            failures.push(format!("{}: No NCU report was produced", key));
            continue;
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(trace)
            .context("Modal returned an invalid profile archive")?;
        // Use a locally generated filename, never a path from the remote response.
        let zip_path = output_dir.join(format!("profile-{}.zip", index));
        std::fs::write(&zip_path, &bytes)?;
        let extracted = service::extract_profile_artifacts(&zip_path, &bytes)?;
        if extracted.reports.is_empty() {
            failures.push(format!("{}: NCU archive contains no .ncu-rep report", key));
        }
        lines.push(format!(
            "{}: {}",
            key,
            run.pointer("/run/result/benchmark.0.spec")
                .and_then(Value::as_str)
                .unwrap_or("unknown benchmark")
        ));
        for path in extracted.details.iter().chain(extracted.reports.iter()) {
            lines.push(format!("  {}", path.display()));
        }
    }
    lines.push(format!(
        "Manifest: {}",
        output_dir.join("manifest.json").display()
    ));
    if !failures.is_empty() {
        return Err(anyhow!("{}\n\n{}", lines.join("\n"), failures.join("\n")));
    }
    Ok(lines.join("\n"))
}

pub async fn run_modal_submission(
    submission_path: &Path,
    leaderboard: &str,
    gpu: &str,
    mode: &str,
    profile_options: &ProfileOptions,
) -> Result<String> {
    if !matches!(
        mode.to_ascii_lowercase().as_str(),
        "test" | "benchmark" | "leaderboard" | "profile"
    ) {
        return Err(anyhow!(
            "Local Modal mode supports test, benchmark, leaderboard, and profile; got '{}'",
            mode
        ));
    }
    profile_options.validate(Some(mode))?;
    let (kernelbot_gpu, modal_gpu) = gpu_names(gpu)?;
    if mode.eq_ignore_ascii_case("profile") && modal_gpu.contains(':') {
        return Err(anyhow!("Nsight Compute profiling requires a single GPU"));
    }
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
        .env("POPCORN_PROFILE_OPTIONS", serde_json::to_string(profile_options)?)
        .kill_on_drop(true)
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
    if mode.eq_ignore_ascii_case("profile") {
        let output_dir = tempfile::Builder::new()
            .prefix("popcorn-profile-")
            .tempdir_in(std::env::current_dir()?)?
            .into_path();
        save_profile_result(&payload, &output_dir)
    } else {
        format_local_result(&payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_payload(trace: &str) -> Value {
        serde_json::json!({
            "leaderboard": "qr_v2", "gpu": "B200", "result": {
                "success": true, "runs": {"profile.3": {
                    "run": {"success": true, "result": {"benchmark.0.spec": "n: 512"}},
                    "profile": {"trace": trace}
                }}
            }
        })
    }

    #[test]
    fn extracts_profile_artifacts_and_records_provenance() {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in ["profile.ncu-rep", "ncu-details.txt", "ncu-details.csv"] {
            zip.start_file(
                format!("profile_data/{}", name),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"captured data").unwrap();
        }
        let trace =
            base64::engine::general_purpose::STANDARD.encode(zip.finish().unwrap().into_inner());
        let mut payload = profile_payload(&trace);
        payload["reference_kernels_ref"] = serde_json::json!("verified-sha");
        payload["profile_options"] = serde_json::json!({"benchmark_index": 3});
        let directory = tempfile::tempdir().unwrap();
        let output = save_profile_result(&payload, directory.path()).unwrap();
        assert!(output.contains("profile.3: n: 512"));
        assert_eq!(
            std::fs::read(directory.path().join("profile-0/profile.ncu-rep")).unwrap(),
            b"captured data"
        );
        assert!(output.contains("ncu-details.csv"));
        let manifest = std::fs::read_to_string(directory.path().join("manifest.json")).unwrap();
        assert!(manifest.contains("verified-sha"));
        assert!(manifest.contains("benchmark_index"));
    }

    #[test]
    fn rejects_missing_reports_and_failed_runs() {
        let directory = tempfile::tempdir().unwrap();
        let mut payload = profile_payload("");
        payload["result"]["runs"]["profile.3"]["profile"] = Value::Null;
        payload["result"]["runs"]["profile.3"]["run"]["success"] = Value::Bool(false);
        payload["result"]["runs"]["profile.3"]["run"]["stderr"] = serde_json::json!("NCU denied");
        let error = save_profile_result(&payload, directory.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("NCU denied"));
        assert!(error.contains("No NCU report"));
        payload["result"]["runs"] = serde_json::json!({});
        assert!(save_profile_result(&payload, directory.path()).is_err());
    }

    #[test]
    fn validates_profile_options_before_launch() {
        let mut options = ProfileOptions {
            benchmark_index: Some(0),
            ..Default::default()
        };
        assert!(options.validate(Some("benchmark")).is_err());
        assert!(options.validate(Some("profile")).is_ok());
        options.ncu_launch_count = Some(0);
        assert!(options.validate(Some("profile")).is_err());
    }

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
