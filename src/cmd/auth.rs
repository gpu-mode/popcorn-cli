use anyhow::{anyhow, Result};
use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use crate::service;

// Configuration structure
#[derive(Serialize, Deserialize, Debug, Default)]
struct Config {
    cli_id: Option<String>,
}

// Helper function to get the config file path
fn get_config_path() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|mut path| {
            path.push(".popcorn.yaml");
            path
        })
        .ok_or_else(|| anyhow!("Could not find home directory"))
}

// Helper function to load config
fn load_config() -> Result<Config> {
    let path = get_config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let file = File::open(path)?;
    serde_yaml::from_reader(file).map_err(|e| anyhow!("Failed to parse config file: {}", e))
}

// Helper function to save config
fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true) // Overwrite existing file
        .open(path)?;
    serde_yaml::to_writer(file, config).map_err(|e| anyhow!("Failed to write config file: {}", e))
}

// Structure for the API response
#[derive(Deserialize)]
struct AuthInitResponse {
    state: String, // This is the cli_id
}

// Function to handle the login logic
pub async fn run_auth(reset: bool, auth_provider: &str) -> Result<()> {
    println!(
        "{} Authenticating via {}...",
        "●".cyan(),
        auth_provider.bold()
    );

    let popcorn_api_url = std::env::var("POPCORN_API_URL").map_err(|_| {
        anyhow!(
            "{} POPCORN_API_URL environment variable not set",
            "error:".red().bold()
        )
    })?;

    let client = service::create_client(None)?;

    let init_url = format!("{}/auth/init?provider={}", popcorn_api_url, auth_provider);

    let init_resp = client.get(&init_url).send().await.map_err(|e| {
        anyhow!(
            "{} Could not reach auth server: {}",
            "error:".red().bold(),
            e
        )
    })?;

    let status = init_resp.status();

    if !status.is_success() {
        let error_text = init_resp.text().await?;
        eprintln!(
            "{} Failed to initialize auth ({}): {}",
            "error:".red().bold(),
            status.to_string().red(),
            error_text
        );
        return Err(anyhow!("Authentication initialization failed"));
    }

    let auth_init_data: AuthInitResponse = init_resp.json().await?;
    let cli_id = auth_init_data.state;

    let state_json = serde_json::json!({
        "cli_id": cli_id,
        "is_reset": reset
    })
    .to_string();
    let state_b64 = base64_url::encode(&state_json);

    let auth_url = match auth_provider {
        "discord" => {
            let base_auth_url = "https://discord.com/oauth2/authorize?client_id=1361364685491802243&response_type=code&redirect_uri=https%3A%2F%2Fsite--bot--dxfjds728w5v.code.run%2Fauth%2Fcli%2Fdiscord&scope=identify";
            format!("{}&state={}", base_auth_url, state_b64)
        }
        "github" => {
            let client_id = "Ov23lieFd2onYk4OnKIR";
            let redirect_uri = "https://site--bot--dxfjds728w5v.code.run/auth/cli/github";
            let encoded_redirect_uri = urlencoding::encode(redirect_uri);
            format!(
                "https://github.com/login/oauth/authorize?client_id={}&state={}&redirect_uri={}",
                client_id, state_b64, encoded_redirect_uri
            )
        }
        _ => {
            eprintln!(
                "{} Unsupported authentication provider: {}",
                "error:".red().bold(),
                auth_provider.yellow()
            );
            return Err(anyhow!(
                "Unsupported authentication provider: {}",
                auth_provider
            ));
        }
    };

    println!(
        "\n  {} Open this URL to log in via {}:\n",
        "▸".bold(),
        auth_provider.bold()
    );
    println!("  {}\n", auth_url.as_str().underlined().cyan());

    let mut browser_failed = false;

    if webbrowser::open(&auth_url).is_err() {
        browser_failed = true;
        println!(
            "  {} Could not open browser automatically — please copy the link above.",
            "!".yellow().bold()
        );
    } else {
        println!(
            "  {} Browser opened. Complete the login there.",
            "✓".green().bold()
        );
    }

    println!(
        "  {} Waiting for authentication to complete...\n",
        "…".dark_grey()
    );

    // Save the cli_id to config file optimistically
    let mut config = load_config().unwrap_or_default();
    config.cli_id = Some(cli_id.clone());
    save_config(&config)?;

    let config_path = get_config_path()?.display().to_string();

    if !browser_failed {
        println!(
            "  {} Authentication initiated! CLI ID saved to {}",
            "✓".green().bold(),
            config_path.underlined()
        );

        println!(
            "  {} You can now use commands that require authentication.\n",
            "●".cyan()
        );
    } else {
        println!(
            "{} You need to open the browser URL above to complete the authentication. After that, you can use the CLI as normal.",
            "?".yellow().bold(),
        );
    }

    Ok(())
}
