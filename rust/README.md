# Popcorn CLI (Rust Version)

A Rust implementation of the Popcorn CLI tool for interacting with the Popcorn GPU service.

## Features

- Submit code to Popcorn GPU service
- Select from available leaderboards
- Choose GPU configurations
- Multiple submission modes (test, benchmark, leaderboard)

## Requirements

- Rust 1.56.0 or later
- A valid Popcorn API URL

## Setup

```bash
# Clone the repository
git clone https://github.com/your-username/popcorn-cli
cd popcorn-cli/rust

# Build the project
cargo build --release

# Set the API URL
export POPCORN_API_URL=https://your-popcorn-api-url
```

## Usage

```bash
# Run the CLI tool
cargo run --release -- /path/to/your/file.py
```

### Popcorn Directives

You can add directives to your code files to pre-select leaderboards and GPUs:

```python
#!POPCORN leaderboard matrix_multiplication
#!POPCORN gpu A100
```

Or in other languages:

```cpp
//!POPCORN leaderboard matrix_multiplication
//!POPCORN gpu A100
```

## License

[Same as original project license]