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

## Architecture Overview

Popcorn CLI is a command-line tool for submitting GPU kernel optimization solutions to [gpumode.com](https://gpumode.com) competitions.

### Directory Structure

```
src/
├── main.rs              # Entry point, sets POPCORN_API_URL
├── cmd/                 # Command handling
│   ├── mod.rs           # CLI argument parsing (clap), config loading
│   ├── auth.rs          # OAuth authentication (Discord/GitHub)
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
