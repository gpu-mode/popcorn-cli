use std::fs;
use std::path::Path;
use anyhow::Result;

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
        has_multiple_gpus
    ))
}

pub fn display_ascii_art() {
    let art = r#"
 _   __                      _  ______          _   
| | / /                     | | | ___ \        | |  
| |/ /  ___ _ __ _ __   ___ | | | |_/ /  ___  _| |_ 
|    \ / _ \ '__| '_ \ / _ \| | | ___ \ / _ \| | __|
| |\  \  __/ |  | | | |  __/| | | |_/ /| (_) | | |_ 
\_| \_/\___|_|  |_| |_|\___|_/ \____/ \___/|_|\__|
                                                  
    POPCORN CLI - GPU MODE
    
 ┌───────────────────────────────────────┐
 │  ┌─────┐ ┌─────┐ ┌─────┐              │
 │  │ooOoo│ │ooOoo│ │ooOoo│              │▒
 │  │oOOOo│ │oOOOo│ │oOOOo│              │▒
 │  │ooOoo│ │ooOoo│ │ooOoo│   ┌────────┐ │▒
 │  └─────┘ └─────┘ └─────┘   │████████│ │▒
 │                            │████████│ │▒
 │ ┌────────────────────────┐ │████████│ │▒
 │ │                        │ │████████│ │▒
 │ │  POPCORN GPU COMPUTE   │ └────────┘ │▒
 │ │                        │            │▒
 │ └────────────────────────┘            │▒
 │                                       │▒
 └───────────────────────────────────────┘▒
  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
    ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀
"#;
    println!("{}", art);
}
