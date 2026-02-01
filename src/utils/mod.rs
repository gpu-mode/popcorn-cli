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

pub fn custom_wrap(
    initial_text: String,
    remaining_text: String,
    available_width: usize,
) -> Vec<String> {
    let mut lines = vec![initial_text];
    let mut current_line = String::with_capacity(available_width);
    for word in remaining_text.split_whitespace() {
        if word.len() > available_width {
            if !current_line.is_empty() {
                lines.push(current_line.clone());
                current_line.clear();
            }
            lines.push(word.to_string());
        } else if current_line.is_empty() {
            current_line.push_str(word);
        } else if current_line.len() + word.len() < available_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line.clone());
            current_line.clear();
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Tests for get_popcorn_directives

    #[test]
    fn test_parse_python_style_directives() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "#!POPCORN leaderboard my-leaderboard").unwrap();
        writeln!(file, "#!POPCORN gpu H100").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "def main():").unwrap();
        writeln!(file, "    pass").unwrap();

        let (directives, has_multiple_gpus) = get_popcorn_directives(file.path()).unwrap();

        assert_eq!(directives.leaderboard_name, "my-leaderboard");
        assert_eq!(directives.gpus, vec!["H100"]);
        assert!(!has_multiple_gpus);
    }

    #[test]
    fn test_parse_cpp_style_directives() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "//!POPCORN leaderboard amd-fp8-mm").unwrap();
        writeln!(file, "//!POPCORN gpu MI300").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "int main() {{ return 0; }}").unwrap();

        let (directives, has_multiple_gpus) = get_popcorn_directives(file.path()).unwrap();

        assert_eq!(directives.leaderboard_name, "amd-fp8-mm");
        assert_eq!(directives.gpus, vec!["MI300"]);
        assert!(!has_multiple_gpus);
    }

    #[test]
    fn test_parse_multiple_gpus_truncates_to_first() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "#!POPCORN leaderboard test").unwrap();
        writeln!(file, "#!POPCORN gpus H100 MI300 A100").unwrap();

        let (directives, has_multiple_gpus) = get_popcorn_directives(file.path()).unwrap();

        assert_eq!(directives.leaderboard_name, "test");
        assert_eq!(directives.gpus, vec!["H100"]);
        assert!(has_multiple_gpus);
    }

    #[test]
    fn test_parse_gpu_vs_gpus_keyword() {
        // Test "gpu" keyword
        let mut file1 = NamedTempFile::new().unwrap();
        writeln!(file1, "#!POPCORN gpu A100").unwrap();
        let (directives1, _) = get_popcorn_directives(file1.path()).unwrap();
        assert_eq!(directives1.gpus, vec!["A100"]);

        // Test "gpus" keyword
        let mut file2 = NamedTempFile::new().unwrap();
        writeln!(file2, "#!POPCORN gpus V100").unwrap();
        let (directives2, _) = get_popcorn_directives(file2.path()).unwrap();
        assert_eq!(directives2.gpus, vec!["V100"]);
    }

    #[test]
    fn test_parse_empty_file_returns_empty_directives() {
        let file = NamedTempFile::new().unwrap();

        let (directives, has_multiple_gpus) = get_popcorn_directives(file.path()).unwrap();

        assert_eq!(directives.leaderboard_name, "");
        assert!(directives.gpus.is_empty());
        assert!(!has_multiple_gpus);
    }

    #[test]
    fn test_parse_ignores_non_directive_comments() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# This is a regular comment").unwrap();
        writeln!(file, "// Another regular comment").unwrap();
        writeln!(file, "#!POPCORN leaderboard real-leaderboard").unwrap();
        writeln!(file, "# POPCORN gpu should-be-ignored").unwrap();

        let (directives, _) = get_popcorn_directives(file.path()).unwrap();

        assert_eq!(directives.leaderboard_name, "real-leaderboard");
        assert!(directives.gpus.is_empty());
    }

    #[test]
    fn test_parse_case_insensitive_directive_args() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "#!POPCORN GPU H100").unwrap();
        writeln!(file, "#!POPCORN LEADERBOARD TEST").unwrap();

        let (directives, _) = get_popcorn_directives(file.path()).unwrap();

        assert_eq!(directives.gpus, vec!["H100"]);
        assert_eq!(directives.leaderboard_name, "TEST");
    }

    #[test]
    fn test_parse_nonexistent_file_returns_error() {
        let result = get_popcorn_directives("/nonexistent/path/file.py");
        assert!(result.is_err());
    }

    // Tests for custom_wrap

    #[test]
    fn test_wrap_simple_text() {
        let result = custom_wrap("Header:".to_string(), "hello world".to_string(), 20);

        assert_eq!(result, vec!["Header:", "hello world"]);
    }

    #[test]
    fn test_wrap_breaks_at_width() {
        let result = custom_wrap("".to_string(), "one two three four".to_string(), 10);

        assert_eq!(result, vec!["", "one two", "three four"]);
    }

    #[test]
    fn test_wrap_handles_long_words() {
        let result = custom_wrap(
            "".to_string(),
            "short verylongwordthatexceedswidth short".to_string(),
            10,
        );

        assert_eq!(
            result,
            vec!["", "short", "verylongwordthatexceedswidth", "short"]
        );
    }

    #[test]
    fn test_wrap_empty_remaining_text() {
        let result = custom_wrap("Header".to_string(), "".to_string(), 20);

        assert_eq!(result, vec!["Header"]);
    }

    #[test]
    fn test_wrap_preserves_initial_text() {
        let result = custom_wrap("PREFIX: ".to_string(), "some text".to_string(), 20);

        assert_eq!(result[0], "PREFIX: ");
    }
}
