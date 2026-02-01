fn main() {
    // CI sets CLI_VERSION env var from git tag, otherwise show "dev"
    let version = std::env::var("CLI_VERSION")
        .map(|v| v.trim_start_matches('v').to_string())
        .unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=CLI_VERSION={}", version);
}
