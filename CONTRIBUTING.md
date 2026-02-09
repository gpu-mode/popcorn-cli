# Contributing

## Development Setup

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

## Testing

### Unit Tests

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

### Test Requirements

When adding new functionality:

1. **Service functions** (`src/service/mod.rs`):
   - Add tests in the `#[cfg(test)] mod tests` block
   - Test error handling, response parsing

2. **Command handlers** (`src/cmd/`):
   - Integration testing via E2E regression tests

### E2E Regression Testing

Full end-to-end testing requires a running kernelbot API server. You can test against production or a local instance.

#### Option A: Test Against Production

```bash
export POPCORN_API_URL=https://discord-cluster-manager-1f6c4782e60a.herokuapp.com
cargo run -- submissions list --leaderboard grayscale
```

#### Option B: Test Against Local Server (Recommended for Development)

This tests the complete flow: CLI → API → Database → Modal runner.

**Step 1: Set up kernelbot server** (in the kernelbot repo):

```bash
# Start PostgreSQL
brew services start postgresql@14

# Create database and run migrations
createdb kernelbot
export DATABASE_URL="postgresql://$(whoami)@localhost:5432/kernelbot"
uv run yoyo apply --database "$DATABASE_URL" src/migrations/

# Create test user
psql "$DATABASE_URL" -c "INSERT INTO leaderboard.user_info (id, user_name, cli_id, cli_valid)
VALUES ('999999', 'testuser', 'test-cli-id-123', true)
ON CONFLICT (id) DO UPDATE SET cli_id = 'test-cli-id-123', cli_valid = true;"

# Start API server
cd src/kernelbot
export ADMIN_TOKEN="your-admin-token"  # Check .env for LOCAL_ADMIN_TOKEN
uv run python main.py --api-only
```

**Step 2: Sync leaderboards**:

```bash
curl -X POST "http://localhost:8000/admin/update-problems" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"problem_set": "pmpp_v2"}'
```

**Step 3: Configure CLI for local testing**:

```bash
# Backup and set test config
cp ~/.popcorn.yaml ~/.popcorn.yaml.bak
echo "cli_id: test-cli-id-123" > ~/.popcorn.yaml
```

**Step 4: Run CLI commands**:

```bash
export POPCORN_API_URL=http://localhost:8000

# Test submissions commands
cargo run --release -- submissions list --leaderboard vectoradd_v2
cargo run --release -- submissions show <ID>
cargo run --release -- submissions delete <ID>

# Test actual submission (requires Modal account for GPU execution)
cargo run --release -- submit solution.py --gpu H100 --leaderboard vectoradd_v2 --mode test
```

**Step 5: Restore original config**:

```bash
cp ~/.popcorn.yaml.bak ~/.popcorn.yaml && rm ~/.popcorn.yaml.bak
```

#### Troubleshooting

- **401 Unauthorized**: CLI ID not registered in database - create test user first
- **404 Not Found**: Leaderboards not synced - run update-problems endpoint
- **Connection refused**: API server not running on localhost:8000
- **"Device not configured"**: TTY issue - ensure POPCORN_API_URL is set

## Admin Commands

Admin commands require the `POPCORN_ADMIN_TOKEN` environment variable.

```bash
# Server control
popcorn admin start                    # Start accepting jobs
popcorn admin stop                     # Stop accepting jobs
popcorn admin stats                    # Get server statistics
popcorn admin stats --last-day         # Stats for last 24 hours only

# Submission management
popcorn admin get-submission <ID>      # Get any submission by ID
popcorn admin delete-submission <ID>   # Delete any submission

# Leaderboard management
popcorn admin create-leaderboard <dir> # Create leaderboard from problem directory
popcorn admin delete-leaderboard <name>        # Delete a leaderboard
popcorn admin delete-leaderboard <name> --force # Force delete with submissions

# Update problems from GitHub
popcorn admin update-problems
popcorn admin update-problems --problem-set nvidia --force
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

### Key Dependencies

- `clap` - CLI argument parsing
- `ratatui` + `crossterm` - Terminal UI
- `reqwest` - HTTP client with SSE streaming
- `tokio` - Async runtime
- `serde` / `serde_yaml` / `serde_json` - Serialization
