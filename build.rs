fn main() {
    // CI sets POPCORN_VERSION env var from git tag, otherwise show "dev"
    let version = std::env::var("POPCORN_VERSION")
        .map(|v| v.trim_start_matches('v').to_string())
        .unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=POPCORN_VERSION={}", version);
}
