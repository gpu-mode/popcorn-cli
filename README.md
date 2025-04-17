# Popcorn CLI

A command-line interface tool for submitting solutions to the [Popcorn Discord Bot](https://github.com/gpu-mode/discord-cluster-manager)

## Installation

### Option 1: Using pre-built binaries (Recommended)

1. Download the latest release for your platform from the releases page
2. Extract the archive
3. Move the binary to a location in your PATH

### Option 2: Building from source

This app is written in Rust, so you can just install it via `cargo install`

## Usage

Set the `POPCORN_API_URL` environment variable to the URL of the Popcorn API. You can get this from the [GPU Mode Discord server](https://discord.gg/gpumode) - go to the submissions chanel and `/get-api-url`

Then, you need to be registered to use this app. You can register by running: `popcorn-cli register [discord|github]`. We strongly reccomend using your Discord account to register, as this will match your submissions to your Discord account.
Once you're registered, there is a file created in your `$HOME` called `.popcorn-cli.yaml` that contains your registration token. This token is sent with each request.

If you want to re-register (you can do this any number of times), you can run `popcorn-cli reregister [discord|github]`.

After this, you can submit a solution by running:

```bash
popcorn-cli submit <submission-file>
```

The interactive CLI will guide you through the process of:
1. Selecting a leaderboard
2. Selecting GPU options
3. Setting submission mode
4. Submitting your work
