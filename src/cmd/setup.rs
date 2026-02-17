use anyhow::{Context, Result};
use serde_json::json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SKILL_NAME: &str = "popcorn-submission-workflow";
const SUBMISSION_FILENAME: &str = "submission.py";
const SKILL_TEMPLATE: &str =
    include_str!("../../templates/setup/skills/popcorn-submission-workflow/SKILL.md");
const AGENTS_TEMPLATE: &str = include_str!("../../templates/setup/AGENTS.md");
const SUBMISSION_TEMPLATE: &str = include_str!("../../templates/setup/submission.py");

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

pub fn run_setup(force: bool) -> Result<()> {
    let cwd = env::current_dir().context("Failed to determine current directory")?;
    let popcorn_dir = cwd.join(".popcorn");
    let skill_dir = popcorn_dir.join("skills").join(SKILL_NAME);
    let skill_path = skill_dir.join("SKILL.md");
    let manifest_path = popcorn_dir.join("setup.json");
    let submission_path = cwd.join(SUBMISSION_FILENAME);
    let agents_path = cwd.join("AGENTS.md");

    fs::create_dir_all(&skill_dir).with_context(|| {
        format!(
            "Failed to create skill directory at {}",
            skill_dir.to_string_lossy()
        )
    })?;

    let readme_path = cwd.join("README.md");
    let readme_content = fs::read_to_string(&readme_path).unwrap_or_default();
    let skill_markdown = build_skill_markdown(&readme_content);
    let skill_status = write_text_file(&skill_path, &skill_markdown, force)?;

    let manifest = json!({
        "schema_version": 1,
        "setup_source": "popcorn setup",
        "skills": [{
            "name": SKILL_NAME,
            "path": format!(".popcorn/skills/{SKILL_NAME}")
        }],
        "agents": ["codex", "claude"]
    });
    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    let manifest_status = write_text_file(&manifest_path, &manifest_text, force)?;

    let agents_md = build_agents_markdown(&skill_path);
    let agents_status = write_text_file(&agents_path, &agents_md, force)?;

    let codex_link_status = create_agent_skill_view(&cwd, "codex", &skill_dir, force)?;
    let claude_link_status = create_agent_skill_view(&cwd, "claude", &skill_dir, force)?;

    let submission_status = write_text_file(
        &submission_path,
        &build_submission_template(),
        force,
    )?;

    println!(
        "{} {}",
        skill_status.label(),
        relative_display(&cwd, &skill_path)
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
        relative_display(
            &cwd,
            &cwd.join(".codex").join("skills").join(SKILL_NAME)
        )
    );
    println!(
        "{} {}",
        claude_link_status.label(),
        relative_display(
            &cwd,
            &cwd.join(".claude").join("skills").join(SKILL_NAME)
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
    let agent_skills_dir = cwd.join(format!(".{}", agent_name)).join("skills");
    fs::create_dir_all(&agent_skills_dir)?;

    let link_path = agent_skills_dir.join(SKILL_NAME);
    let existed_before = path_exists(&link_path);
    if existed_before && !force {
        return Ok(ActionStatus::Skipped);
    }

    if existed_before {
        remove_existing_path(&link_path)?;
    }

    let relative_target = PathBuf::from("../../.popcorn/skills").join(SKILL_NAME);
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

fn build_agents_markdown(skill_path: &Path) -> String {
    let skill_path_text = skill_path.to_string_lossy().to_string();
    render_template(
        AGENTS_TEMPLATE,
        &[("{{SKILL_NAME}}", SKILL_NAME), ("{{SKILL_PATH}}", &skill_path_text)],
    )
}

fn build_submission_template() -> String {
    SUBMISSION_TEMPLATE.to_string()
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = template.to_string();
    for (needle, value) in replacements {
        output = output.replace(needle, value);
    }
    output
}
