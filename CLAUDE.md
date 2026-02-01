# CLAUDE.md

## Contributing

### Development Setup

```bash
# Build
cargo build

# Run tests
cargo test

# Format code (required before commits)
cargo fmt --all

# Lint (must pass with no warnings)
cargo clippy --all-targets --all-features -- -D warnings
```

### CI Requirements

All PRs must pass:
- `cargo fmt --all -- --check` - Code formatting
- `cargo clippy -- -D warnings` - No clippy warnings allowed
- `cargo test` - All tests pass
- Builds on Linux, macOS, and Windows

### Testing

#### Unit Tests

Tests are in the same file as the code (Rust convention):
- `src/service/mod.rs` - API client tests
- `src/utils/mod.rs` - Utility function tests

Run all tests:
```bash
cargo test
```

Run specific tests:
```bash
cargo test test_name
```

#### Test Requirements

When adding new functionality:

1. **Service functions** (`src/service/mod.rs`):
   - Add tests in the `#[cfg(test)] mod tests` block
   - Test error handling, response parsing

2. **Command handlers** (`src/cmd/`):
   - Integration testing via E2E regression tests

#### E2E Regression Testing

Use a local instance of kernelbot to test CLI functionality end-to-end:

```bash
# Start local kernelbot server (see kernelbot repo)
cd ../kernelbot
docker compose up -d
uv run uvicorn src.kernelbot.api.main:app --reload --port 8000

# Test CLI against local instance
export POPCORN_API_URL=http://localhost:8000
cargo run -- submissions list --leaderboard test-leaderboard
cargo run -- submissions show 123
cargo run -- submissions delete 123 --force
```

## Architecture Overview

Popcorn CLI is a command-line tool for submitting GPU kernel optimization solutions to [gpumode.com](https://gpumode.com) competitions.

### Directory Structure

```
src/
├── main.rs              # Entry point, sets POPCORN_API_URL
├── cmd/                 # Command handling
│   ├── mod.rs           # CLI argument parsing (clap), config loading
│   ├── admin.rs         # Admin commands (requires POPCORN_ADMIN_TOKEN)
│   ├── auth.rs          # OAuth authentication (Discord/GitHub)
│   ├── submissions.rs   # User submission management (list, show, delete)
│   └── submit.rs        # Submission logic, TUI app state machine
├── service/
│   └── mod.rs           # HTTP client, API calls, SSE streaming
├── models/
│   └── mod.rs           # Data structures (LeaderboardItem, GpuItem, AppState)
├── utils/
│   └── mod.rs           # Directive parsing, text wrapping, ASCII art
└── views/
    ├── loading_page.rs  # TUI loading screen with progress bar
    └── result_page.rs   # TUI results display with scrolling
```

### Before Adding New Features

**Important:** Before implementing new functionality, check for existing code in both repos:

1. **Check discord-cluster-manager** for existing Discord commands and database methods:
   - `src/kernelbot/cogs/` - Discord bot commands
   - `src/libkernelbot/leaderboard_db.py` - Database methods
   - `src/kernelbot/api/main.py` - Existing API endpoints

2. **Check popcorn-cli** for existing service functions and commands:
   - `src/service/mod.rs` - API client functions
   - `src/cmd/` - CLI command handlers

3. **Reuse existing functionality** where possible:
   - Database methods (e.g., `get_submission_by_id`, `delete_submission`)
   - API response handling patterns
   - Authentication validation (`validate_user_header`, `validate_cli_header`)

### Core Flow

1. **Authentication** (`cmd/auth.rs`): User registers via Discord/GitHub OAuth. CLI ID stored in `~/.popcorn.yaml`.

2. **Submission** (`cmd/submit.rs`):
   - TUI mode: Interactive selection of leaderboard → GPU → mode
   - Plain mode (`--no-tui`): Direct submission with CLI flags
   - Reads solution file with optional `#!POPCORN` directives for defaults

3. **API Communication** (`service/mod.rs`):
   - Fetches available leaderboards and GPUs
   - Submits solutions via multipart form POST
   - Handles SSE (Server-Sent Events) streaming for real-time results
   - Supports modes: `test`, `benchmark`, `leaderboard`, `profile`

### File Directives

Users can embed defaults in their solution files:

```python
#!POPCORN leaderboard amd-fp8-mm
#!POPCORN gpu MI300

def solution():
    ...
```

Or C++ style:
```cpp
//!POPCORN leaderboard nvidia-matmul
//!POPCORN gpu H100
```

### Key Dependencies

- `clap` - CLI argument parsing
- `ratatui` + `crossterm` - Terminal UI
- `reqwest` - HTTP client with SSE streaming
- `tokio` - Async runtime
- `serde` / `serde_yaml` / `serde_json` - Serialization
