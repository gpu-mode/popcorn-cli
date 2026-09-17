use crate::service;
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

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

fn save_profile_result(payload: &Value, output_dir: &Path) -> Result<String> {
    let result = &payload["result"];
    if result["success"].as_bool() != Some(true) {
        return Err(anyhow!("Profiling failed: {}", result["error"]));
    }
    let runs = result["runs"]
        .as_object()
        .filter(|runs| !runs.is_empty())
        .ok_or_else(|| anyhow!("Server returned no profile runs"))?;
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "leaderboard": payload["leaderboard"],
            "gpu": payload["gpu"],
            "system": result["system"],
            "profile_metadata": result["profile_metadata"],
        }))?,
    )?;
    let mut lines = vec![format!(
        "Nsight Compute profile: {} on {}",
        payload["leaderboard"].as_str().unwrap_or("unknown"),
        payload["gpu"].as_str().unwrap_or("unknown")
    )];
    let mut failures = Vec::new();
    for (index, (key, run)) in runs.iter().enumerate() {
        if let Some(failure) = run_failure(run) {
            failures.push(format!("{}: {}", key, failure));
            if let Some(stdout) = run.pointer("/run/stdout").and_then(Value::as_str) {
                failures.push(stdout.to_string());
            }
        }
        let Some(trace) = run.pointer("/profile/trace").and_then(Value::as_str) else {
            failures.push(format!("{}: No NCU report was produced", key));
            continue;
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(trace)
            .context("Server returned an invalid profile archive")?;
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

pub fn save_hosted_results(response: &Value, leaderboard: &str, gpu: &str) -> Result<String> {
    let results = response["results"]
        .as_array()
        .filter(|results| !results.is_empty())
        .ok_or_else(|| anyhow!("Server returned no profiling results"))?;
    let output_dir = tempfile::Builder::new()
        .prefix("popcorn-profile-")
        .tempdir_in(std::env::current_dir()?)?
        .into_path();
    let mut summaries = Vec::new();
    for (index, result) in results.iter().enumerate() {
        summaries.push(save_profile_result(
            &serde_json::json!({
                "leaderboard": leaderboard, "gpu": gpu, "result": result,
            }),
            &output_dir.join(format!("result-{}", index)),
        )?);
    }
    Ok(summaries.join("\n\n"))
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
        payload["result"]["profile_metadata"] = serde_json::json!({"config_sha256": "verified-sha", "capture_options": {"benchmark_index": 3}});
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
}
