use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use crate::service; // Assuming service::create_client is needed

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
    println!("Attempting authentication via {}...", auth_provider);

    let popcorn_api_url = std::env::var("POPCORN_API_URL")
        .map_err(|_| anyhow!("POPCORN_API_URL environment variable not set"))?;

    let client = service::create_client(None)?;

    let init_url = format!("{}/auth/init?provider={}", popcorn_api_url, auth_provider);
    println!("Requesting CLI ID from {}", init_url);

    let init_resp = client.get(&init_url).send().await?;

    let status = init_resp.status();

    if !status.is_success() {
        let error_text = init_resp.text().await?;
        return Err(anyhow!(
            "Failed to initialize auth ({}): {}",
            status,
            error_text
        ));
    }

    let auth_init_data: AuthInitResponse = init_resp.json().await?;
    let cli_id = auth_init_data.state;
    println!("Received CLI ID: {}", cli_id);

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
            let redirect_uri =
                "https://site--bot--dxfjds728w5v.code.run/auth/cli/github";
            let encoded_redirect_uri = urlencoding::encode(redirect_uri);
            format!(
                "https://github.com/login/oauth/authorize?client_id={}&state={}&redirect_uri={}",
                client_id, state_b64, encoded_redirect_uri
            )
        }
        _ => {
            return Err(anyhow!(
                "Unsupported authentication provider: {}",
                auth_provider
            ))
        }
    };

    println!(
        "\n>>> Please open the following URL in your browser to log in via {}:",
        auth_provider
    );
    println!("{}", auth_url);
    println!("\nWaiting for you to complete the authentication in your browser...");
    println!(
        "After successful authentication with {}, the CLI ID will be saved.",
        auth_provider
    );

    if webbrowser::open(&auth_url).is_err() {
        println!(
            "Could not automatically open the browser. Please copy the URL above and paste it manually."
        );
    }

    // Save the cli_id to config file optimistically
    let mut config = load_config().unwrap_or_default();
    config.cli_id = Some(cli_id.clone());
    save_config(&config)?;

    println!(
        "\nSuccessfully initiated authentication. Your CLI ID ({}) has been saved to {}. To use the CLI on different machines, you can copy the config file.",
        cli_id,
        get_config_path()?.display()
    );
    println!("You can now use other commands that require authentication.");

    Ok(())
}
