use anyhow::Result;
use std::fs;
use std::path::Path;

pub struct PopcornDirectives {
    pub leaderboard_name: String,
    pub gpus: Vec<String>,
}

pub fn get_popcorn_directives<P: AsRef<Path>>(filepath: P) -> Result<(PopcornDirectives, bool)> {
    let content = fs::read_to_string(filepath)?;

    let mut gpus: Vec<String> = Vec::new();
    let mut leaderboard_name = String::new();
    let mut has_multiple_gpus = false;

    for line in content.lines() {
        if !line.starts_with("//") && !line.starts_with("#") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        if parts[0] == "//!POPCORN" || parts[0] == "#!POPCORN" {
            let arg = parts[1].to_lowercase();
            if arg == "gpu" || arg == "gpus" {
                gpus = parts[2..].iter().map(|s| s.to_string()).collect();
            } else if arg == "leaderboard" && parts.len() > 2 {
                leaderboard_name = parts[2].to_string();
            }
        }
    }

    if gpus.len() > 1 {
        has_multiple_gpus = true;
        gpus = vec![gpus[0].clone()];
    }

    Ok((
        PopcornDirectives {
            leaderboard_name,
            gpus,
        },
        has_multiple_gpus,
    ))
}

pub fn get_ascii_art_frame(frame: u16) -> String {
    let frame = frame % 3;
    match frame {
        0 => r#"
            ▗▖ ▗▖▗▄▄▄▖▗▄▄▖ ▗▖  ▗▖▗▄▄▄▖▗▖   ▗▄▄▖  ▗▄▖ ▗▄▄▄▖
            ▐▌▗▞▘▐▌   ▐▌ ▐▌▐▛▚▖▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌ ▐▌  █  
            ▐▛▚▖ ▐▛▀▀▘▐▛▀▚▖▐▌ ▝▜▌▐▛▀▀▘▐▌   ▐▛▀▚▖▐▌ ▐▌  █  
            ▐▌ ▐▌▐▙▄▄▖▐▌ ▐▌▐▌  ▐▌▐▙▄▄▖▐▙▄▄▖▐▙▄▞▘▝▚▄▞▘  █  

                      POPCORN CLI - GPU MODE
             
          ┌────────────────────────────────────────────┐
          │  ╔══════════════════════════════════╗    ϟ │
          │  ║ ▄▄ Graphics Processing Unit  ▄▄║ ║      │▒
          │  ║ ██████  80GB HBM3 MEMORY      █║ ║      │▒
          │  ║ ▀▀▀▀▀▀  700W TDP              █║ ║      │▒
          │  ╚══════════════════════════════════╝      │▒
          │   ┌─────┐┌─────┐┌─────┐┌─────┐┌─────┐     │▒
          │   │:::::││:::::││:::::││:::::││:::::│     │▒
          │   └─────┘└─────┘└─────┘└─────┘└─────┘     │▒
          │  ┌──────────────────────────────────┐      │▒
          │  │    discord.com/invite/gpumode    │      │▒
          │  │    ═══╧═══╧═══╧═══╧═══╧═══╧═══   │      │▒
          │  └──────────────────────────────────┘      │▒
          └────────────────────────────────────────────┘▒
           ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
             ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀"#
            .to_string(),
        1 => r#"
            ▗▖ ▗▖▗▄▄▄▖▗▄▄▖ ▗▖  ▗▖▗▄▄▄▖▗▖   ▗▄▄▖  ▗▄▖ ▗▄▄▄▖
            ▐▌▗▞▘▐▌   ▐▌ ▐▌▐▛▚▖▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌ ▐▌  █  
            ▐▛▚▖ ▐▛▀▀▘▐▛▀▚▖▐▌ ▝▜▌▐▛▀▀▘▐▌   ▐▛▀▚▖▐▌ ▐▌  █  
            ▐▌ ▐▌▐▙▄▄▖▐▌ ▐▌▐▌  ▐▌▐▙▄▄▖▐▙▄▄▖▐▙▄▞▘▝▚▄▞▘  █  

                      POPCORN CLI - GPU MODE
             
          ┌────────────────────────────────────────────┐
          │  ╔══════════════════════════════════╗   ϟϟ │
          │  ║ ▄▄ Graphics Processing Unit  ▄▄║ ║      │▒
          │  ║ ██████  80GB HBM3 MEMORY    ███║ ║      │▒
          │  ║ ▀▀▀▀▀▀  700W TDP            ███║ ║      │▒
          │  ╚══════════════════════════════════╝      │▒
          │   ┌─────┐┌─────┐┌─────┐┌─────┐┌─────┐     │▒
          │   │:::::││:::::││:::::││:::::││:::::│     │▒
          │   └─────┘└─────┘└─────┘└─────┘└─────┘     │▒
          │  ┌──────────────────────────────────┐      │▒
          │  │    discord.com/invite/gpumode    │      │▒
          │  │    ═══╧═══╧═══╧═══╧═══╧═══╧═══   │      │▒
          │  └──────────────────────────────────┘      │▒
          └────────────────────────────────────────────┘▒
           ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
             ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀"#
            .to_string(),
        _ => r#"
            ▗▖ ▗▖▗▄▄▄▖▗▄▄▖ ▗▖  ▗▖▗▄▄▄▖▗▖   ▗▄▄▖  ▗▄▖ ▗▄▄▄▖
            ▐▌▗▞▘▐▌   ▐▌ ▐▌▐▛▚▖▐▌▐▌   ▐▌   ▐▌ ▐▌▐▌ ▐▌  █  
            ▐▛▚▖ ▐▛▀▀▘▐▛▀▚▖▐▌ ▝▜▌▐▛▀▀▘▐▌   ▐▛▀▚▖▐▌ ▐▌  █  
            ▐▌ ▐▌▐▙▄▄▖▐▌ ▐▌▐▌  ▐▌▐▙▄▄▖▐▙▄▄▖▐▙▄▞▘▝▚▄▞▘  █  

                      POPCORN CLI - GPU MODE
             
          ┌────────────────────────────────────────────┐
          │  ╔══════════════════════════════════╗  ϟϟϟ │
          │  ║ ▄▄ Graphics Processing Unit  ▄▄║ ║      │▒
          │  ║ ██████  80GB HBM3 MEMORY  █████║ ║      │▒
          │  ║ ▀▀▀▀▀▀  700W TDP          █████║ ║      │▒
          │  ╚══════════════════════════════════╝      │▒
          │   ┌─────┐┌─────┐┌─────┐┌─────┐┌─────┐     │▒
          │   │:::::││:::::││:::::││:::::││:::::│     │▒
          │   └─────┘└─────┘└─────┘└─────┘└─────┘     │▒
          │  ┌──────────────────────────────────┐      │▒
          │  │    discord.com/invite/gpumode    │      │▒
          │  │    ═══╧═══╧═══╧═══╧═══╧═══╧═══   │      │▒
          │  └──────────────────────────────────┘      │▒
          └────────────────────────────────────────────┘▒
           ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
             ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀"#
            .to_string(),
    }
}

pub fn wrap_text(text: &str, available_width: usize) -> Vec<String> {
    if text.len() <= available_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 <= available_width {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        } else {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}
