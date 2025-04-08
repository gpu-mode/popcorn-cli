mod cmd;
mod models;
mod service;
mod utils;

use std::env;
use std::process;

#[tokio::main]
async fn main() {
    if env::var("POPCORN_API_URL").is_err() {
        eprintln!("POPCORN_API_URL is not set. Please set it to the URL of the Popcorn API.");
        process::exit(1);
    }
    
    if let Err(e) = cmd::execute().await {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}