mod cmd;
mod models;
mod service;
mod utils;

use crate::cmd::Cli;
use clap::Parser;
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let cli = Cli::parse();

    // Popcorn API URL check (needed for most commands)
    // We might want to move this check inside specific commands later if some don't need it.
    if env::var("POPCORN_API_URL").is_err() {
        eprintln!("POPCORN_API_URL is not set. Please set it to the URL of the Popcorn API.");
        process::exit(1);
    }

    // Execute the parsed command
    if let Err(e) = cmd::execute(cli).await {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
