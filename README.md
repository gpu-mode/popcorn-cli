# Popcorn CLI

A command-line interface tool for submitting solutions to the [Popcorn Discord Bot](https://github.com/gpu-mode/discord-cluster-manager)

## Installation

### Option 1: Using pre-built binaries (Recommended)

1. Download the latest release for your platform from the releases page
2. Extract the archive
3. Move the binary to a location in your PATH

### Option 2: Building from source

If you want to build from source, you'll need:
1. Install [Go](https://golang.org/doc/install)
2. Run:
```bash
GOPROXY=direct go install github.com/S1ro1/popcorn-cli@latest
```
3. Make sure the `popcorn-cli` binary is in your PATH

## Usage

Set the `POPCORN_API_URL` environment variable to the URL of the Popcorn API

Then, simply run the binary:
```bash
popcorn-cli <submission-file>
```

The interactive CLI will guide you through the process of:
1. Selecting a leaderboard
2. Choosing a runner
3. Selecting GPU options
4. Setting submission mode
5. Submitting your work

