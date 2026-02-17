use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const SKILL_NAME: &str = "popcorn-submission-workflow";
const NATIVE_SKILL_NAME: &str = "load-inline-native-code";
const SUBMISSION_FILENAME: &str = "submission.py";
const SKILL_TEMPLATE: &str =
    include_str!("../../templates/setup/skills/popcorn-submission-workflow/SKILL.md");
const NATIVE_SKILL_TEMPLATE: &str =
    include_str!("../../templates/setup/skills/load-inline-native-code/SKILL.md");
const AGENTS_TEMPLATE: &str = include_str!("../../templates/setup/AGENTS.md");

const COMPETITION_YAMLS: &[&str] = &[
    "pmpp_v2.yaml",
    "nvidia.yaml",
    "amd.yaml",
    "amd_distributed.yaml",
    "bioml.yaml",
];

const RAW_GITHUB_BASE: &str =
    "https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems";

#[derive(Deserialize)]
struct CompetitionIndex {
    name: String,
    problems: Vec<ProblemEntry>,
}

#[derive(Deserialize)]
struct ProblemEntry {
    directory: String,
    name: String,
    gpus: Vec<String>,
}

#[derive(Clone, Copy)]
enum ActionStatus {
    Created,
    Updated,
    Skipped,
}

impl ActionStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Skipped => "skipped",
        }
    }
}

async fn fetch_competition_index(client: &reqwest::Client) -> Result<Vec<(String, ProblemEntry)>> {
    let mut entries = Vec::new();
    for filename in COMPETITION_YAMLS {
        let url = format!("{}/{}", RAW_GITHUB_BASE, filename);
        let resp = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch {}", url))?;
        if !resp.status().is_success() {
            eprintln!(
                "Warning: could not fetch {} (status {})",
                filename,
                resp.status()
            );
            continue;
        }
        let text = resp.text().await?;
        let index: CompetitionIndex =
            serde_yaml::from_str(&text).with_context(|| format!("Failed to parse {}", filename))?;
        let comp_name = index.name.clone();
        for problem in index.problems {
            entries.push((comp_name.clone(), problem));
        }
    }
    if entries.is_empty() {
        return Err(anyhow!(
            "No competitions found. Check your network connection."
        ));
    }
    Ok(entries)
}

async fn download_submission(
    client: &reqwest::Client,
    directory: &str,
    leaderboard_name: &str,
    gpu: &str,
) -> Result<String> {
    let url = format!("{}/{}/submission.py", RAW_GITHUB_BASE, directory);
    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch {}", url))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "Failed to download submission.py from {} (status {})",
            url,
            resp.status()
        ));
    }
    let body = resp.text().await?;

    // Strip existing #!POPCORN directives and leading blank lines
    let content: String = body
        .lines()
        .skip_while(|line| line.starts_with("#!POPCORN") || line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
        "#!POPCORN leaderboard {}\n#!POPCORN gpu {}\n\n{}\n",
        leaderboard_name, gpu, content
    ))
}

fn prompt_choice(prompt_text: &str, max: usize) -> Result<usize> {
    loop {
        print!("{}", prompt_text);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim().parse::<usize>() {
            Ok(n) if n >= 1 && n <= max => return Ok(n - 1),
            _ => println!("Please enter a number between 1 and {}", max),
        }
    }
}

pub async fn run_setup() -> Result<()> {
    let cwd = env::current_dir().context("Failed to determine current directory")?;

    // Fetch competitions from GitHub
    println!("Fetching competitions from gpu-mode/reference-kernels...");
    let client = reqwest::Client::new();
    let entries = fetch_competition_index(&client).await?;

    // Build unique competition list preserving order
    let mut comp_names: Vec<String> = Vec::new();
    for (name, _) in &entries {
        if !comp_names.contains(name) {
            comp_names.push(name.clone());
        }
    }

    // Select competition
    println!("\nAvailable competitions:");
    for (i, name) in comp_names.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
    let comp_idx = prompt_choice(
        &format!("\nSelect a competition [1-{}]: ", comp_names.len()),
        comp_names.len(),
    )?;
    let chosen_comp = &comp_names[comp_idx];

    // Filter problems for chosen competition
    let problems: Vec<&ProblemEntry> = entries
        .iter()
        .filter(|(name, _)| name == chosen_comp)
        .map(|(_, p)| p)
        .collect();

    // Select problem
    println!("\nProblems in \"{}\":", chosen_comp);
    for (i, p) in problems.iter().enumerate() {
        println!("  {}. {}", i + 1, p.name);
    }
    let prob_idx = prompt_choice(
        &format!("\nSelect a problem [1-{}]: ", problems.len()),
        problems.len(),
    )?;
    let chosen_problem = problems[prob_idx];

    // Select GPU
    println!("\nAvailable GPUs for \"{}\":", chosen_problem.name);
    for (i, gpu) in chosen_problem.gpus.iter().enumerate() {
        println!("  {}. {}", i + 1, gpu);
    }
    let gpu_idx = prompt_choice(
        &format!("\nSelect a GPU [1-{}]: ", chosen_problem.gpus.len()),
        chosen_problem.gpus.len(),
    )?;
    let chosen_gpu = &chosen_problem.gpus[gpu_idx];

    // Download submission template
    println!(
        "\nDownloading submission template for {} on {}...",
        chosen_problem.name, chosen_gpu
    );
    let submission_content = download_submission(
        &client,
        &chosen_problem.directory,
        &chosen_problem.name,
        chosen_gpu,
    )
    .await?;

    // Write scaffolding files
    let popcorn_dir = cwd.join(".popcorn");
    let skill_dir = popcorn_dir.join("skills").join(SKILL_NAME);
    let skill_path = skill_dir.join("SKILL.md");
    let native_skill_dir = popcorn_dir.join("skills").join(NATIVE_SKILL_NAME);
    let native_skill_path = native_skill_dir.join("SKILL.md");
    let manifest_path = popcorn_dir.join("setup.json");
    let submission_path = cwd.join(SUBMISSION_FILENAME);
    let agents_path = cwd.join("AGENTS.md");

    fs::create_dir_all(&skill_dir).with_context(|| {
        format!(
            "Failed to create skill directory at {}",
            skill_dir.to_string_lossy()
        )
    })?;
    fs::create_dir_all(&native_skill_dir).with_context(|| {
        format!(
            "Failed to create skill directory at {}",
            native_skill_dir.to_string_lossy()
        )
    })?;

    let readme_path = cwd.join("README.md");
    let readme_content = fs::read_to_string(&readme_path).unwrap_or_default();
    let skill_markdown = build_skill_markdown(&readme_content);
    let skill_status = write_text_file(&skill_path, &skill_markdown, true)?;

    let native_skill_status = write_text_file(&native_skill_path, NATIVE_SKILL_TEMPLATE, true)?;

    let manifest = json!({
        "schema_version": 1,
        "setup_source": "popcorn setup",
        "skills": [
            {
                "name": SKILL_NAME,
                "path": format!(".popcorn/skills/{SKILL_NAME}")
            },
            {
                "name": NATIVE_SKILL_NAME,
                "path": format!(".popcorn/skills/{NATIVE_SKILL_NAME}")
            }
        ],
        "agents": ["codex", "claude"]
    });
    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    let manifest_status = write_text_file(&manifest_path, &manifest_text, true)?;

    let agents_md = build_agents_markdown(&skill_path, &native_skill_path);
    let agents_status = write_text_file(&agents_path, &agents_md, true)?;

    let codex_link_status = create_agent_skill_view(&cwd, "codex", &skill_dir, true)?;
    let claude_link_status = create_agent_skill_view(&cwd, "claude", &skill_dir, true)?;
    let codex_native_link_status = create_agent_skill_view(&cwd, "codex", &native_skill_dir, true)?;
    let claude_native_link_status =
        create_agent_skill_view(&cwd, "claude", &native_skill_dir, true)?;

    let submission_status = write_text_file(&submission_path, &submission_content, true)?;

    println!(
        "{} {}",
        skill_status.label(),
        relative_display(&cwd, &skill_path)
    );
    println!(
        "{} {}",
        native_skill_status.label(),
        relative_display(&cwd, &native_skill_path)
    );
    println!(
        "{} {}",
        manifest_status.label(),
        relative_display(&cwd, &manifest_path)
    );
    println!(
        "{} {}",
        agents_status.label(),
        relative_display(&cwd, &agents_path)
    );
    println!(
        "{} {}",
        codex_link_status.label(),
        relative_display(&cwd, &cwd.join(".codex").join("skills").join(SKILL_NAME))
    );
    println!(
        "{} {}",
        codex_native_link_status.label(),
        relative_display(
            &cwd,
            &cwd.join(".codex").join("skills").join(NATIVE_SKILL_NAME)
        )
    );
    println!(
        "{} {}",
        claude_link_status.label(),
        relative_display(&cwd, &cwd.join(".claude").join("skills").join(SKILL_NAME))
    );
    println!(
        "{} {}",
        claude_native_link_status.label(),
        relative_display(
            &cwd,
            &cwd.join(".claude").join("skills").join(NATIVE_SKILL_NAME)
        )
    );
    println!(
        "{} {}",
        submission_status.label(),
        relative_display(&cwd, &submission_path)
    );

    Ok(())
}

fn relative_display(cwd: &Path, target: &Path) -> String {
    match target.strip_prefix(cwd) {
        Ok(relative) => relative.to_string_lossy().to_string(),
        Err(_) => target.to_string_lossy().to_string(),
    }
}

fn write_text_file(path: &Path, content: &str, force: bool) -> Result<ActionStatus> {
    let existed_before = path_exists(path);
    if existed_before && !force {
        return Ok(ActionStatus::Skipped);
    }

    if existed_before {
        remove_existing_path(path)?;
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;
    if existed_before {
        Ok(ActionStatus::Updated)
    } else {
        Ok(ActionStatus::Created)
    }
}

fn create_agent_skill_view(
    cwd: &Path,
    agent_name: &str,
    skill_source_dir: &Path,
    force: bool,
) -> Result<ActionStatus> {
    let skill_dir_name = skill_source_dir
        .file_name()
        .ok_or_else(|| anyhow!("skill source dir has no file name"))?;
    let agent_skills_dir = cwd.join(format!(".{}", agent_name)).join("skills");
    fs::create_dir_all(&agent_skills_dir)?;

    let link_path = agent_skills_dir.join(skill_dir_name);
    let existed_before = path_exists(&link_path);
    if existed_before && !force {
        return Ok(ActionStatus::Skipped);
    }

    if existed_before {
        remove_existing_path(&link_path)?;
    }

    let relative_target = PathBuf::from("../../.popcorn/skills").join(skill_dir_name);
    let symlink_result = create_symlink_dir(&relative_target, &link_path);
    if symlink_result.is_err() {
        copy_dir_all(skill_source_dir, &link_path)?;
    }

    if existed_before {
        Ok(ActionStatus::Updated)
    } else {
        Ok(ActionStatus::Created)
    }
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn remove_existing_path(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || file_type.is_file() {
        fs::remove_file(path)?;
    } else if file_type.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(from, to)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink_dir(target: &Path, link_path: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link_path)
}

#[cfg(windows)]
fn create_symlink_dir(target: &Path, link_path: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link_path)
}

fn extract_top_level_section(content: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == heading)
        .map(|idx| idx + 1)?;

    let mut end = lines.len();
    for (idx, line) in lines.iter().enumerate().skip(start) {
        if line.trim_start().starts_with("## ") {
            end = idx;
            break;
        }
    }

    let section = lines[start..end].join("\n").trim().to_string();
    if section.is_empty() {
        None
    } else {
        Some(section)
    }
}

fn build_skill_markdown(readme_content: &str) -> String {
    let authentication = extract_top_level_section(readme_content, "## Authentication")
        .unwrap_or_else(|| "See project README for authentication details.".to_string());
    let commands = extract_top_level_section(readme_content, "## Commands")
        .unwrap_or_else(|| "See project README for command usage.".to_string());
    let submission_format = extract_top_level_section(readme_content, "## Submission Format")
        .unwrap_or_else(|| "Submissions are expected as a single Python file.".to_string());

    render_template(
        SKILL_TEMPLATE,
        &[
            ("{{SKILL_NAME}}", SKILL_NAME),
            ("{{AUTHENTICATION_SECTION}}", &authentication),
            ("{{COMMANDS_SECTION}}", &commands),
            ("{{SUBMISSION_FORMAT_SECTION}}", &submission_format),
        ],
    )
}

fn build_agents_markdown(skill_path: &Path, native_skill_path: &Path) -> String {
    let skill_path_text = skill_path.to_string_lossy().to_string();
    let native_skill_path_text = native_skill_path.to_string_lossy().to_string();
    render_template(
        AGENTS_TEMPLATE,
        &[
            ("{{SKILL_NAME}}", SKILL_NAME),
            ("{{SKILL_PATH}}", &skill_path_text),
            ("{{NATIVE_SKILL_NAME}}", NATIVE_SKILL_NAME),
            ("{{NATIVE_SKILL_PATH}}", &native_skill_path_text),
        ],
    )
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = template.to_string();
    for (needle, value) in replacements {
        output = output.replace(needle, value);
    }
    output
}
