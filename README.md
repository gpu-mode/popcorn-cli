# Popcorn CLI

A command-line interface tool for submitting solutions to the [Popcorn Discord Bot](https://github.com/gpu-mode/discord-cluster-manager)

Tested on linux and mac but should just work on Windows as well.

## Installation

### Option 1: Using pre-built binaries (Recommended)

1. Download the latest release for your platform from the releases page
2. Extract the archive
3. Move the binary to a location in your PATH

### Option 2: Building from source

1. Download rust `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. `cd popcorn-cli && cargo install --path .`

## Authentication

Since we're effectively giving out GPUs for free we rely on either github or discord authentication to prove that you're a real human before you access our service.

1. Go to the [GPU Mode Discord server](https://discord.gg/gpumode) and type in `/get-api-url/`
2. Copy paste that url out `export POPCORN_API_URL="result_of_get_api_url"`
3. We recommend you authenticate via your Discord as this will guarantee that your name will show up correctly on the leaderboard, you can do this via `popcorn-cli register discord`. However in case this doesn't work for you we also support Github based authentication with `popcorn-cli register github`
4. To ensure the above worked you can run `cat $HOME/.popcorn.yaml` which should print your client ID which is what will be sent to us on every request

Sometimes you'll get an error that you're already authenticated despite being unable to submit in which case you can run `popcorn-cli reregister [discord|github]`.

Set the `POPCORN_API_URL` environment variable to the URL of the Popcorn API. You can get this from the [GPU Mode Discord server](https://discord.gg/gpumode).

## Make your first submission

After this, you can submit a solution by running:

```bash
popcorn-cli submit <submission-file>
```

The interactive CLI will guide you through the process of:
1. Selecting a leaderboard
2. Selecting GPU options
3. Setting submission mode
4. Submitting your work

glhf!