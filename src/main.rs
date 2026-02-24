mod cmd;
mod models;
mod service;
mod utils;
mod views;

use crate::cmd::Cli;
use clap::Parser;
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    // Set the API URL FIRST - before anything else
    if env::var("POPCORN_API_URL").is_err() {
        env::set_var(
            "POPCORN_API_URL",
            "https://site--bot--dxfjds728w5v.code.run",
        );
    }
    // Parse command line arguments
    let cli = Cli::parse();

    // Execute the parsed command
    if let Err(e) = cmd::execute(cli).await {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}
