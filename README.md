# Popcorn CLI

A command-line interface tool for submitting solutions to the [Popcorn Discord Bot](https://github.com/gpu-mode/discord-cluster-manager)
<img width="1034" alt="Screenshot 2025-06-10 at 11 17 45 AM" src="https://github.com/user-attachments/assets/66414f12-a984-4a3d-b035-d31f8695a54d" />

Tested on linux and mac but should just work on Windows as well.

## Installation

### Option 1: Using pre-built binaries (Recommended)

1. Download the latest release for your platform from the releases page
2. Extract the archive
3. Move the binary to a location in your PATH

### Option 2: Building from source

1. Download rust `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. `cd popcorn-cli && ./build.sh`

## Authentication

Since we're effectively giving out GPUs for free we rely on either github or discord authentication to prove that you're a real human before you access our service.

1. Go to the [GPU Mode Discord server](https://discord.gg/gpumode) and type in `/get-api-url`
2. Copy paste that url out `export POPCORN_API_URL="result_of_get_api_url"`
3. We recommend you authenticate via your Discord as this will guarantee that your name will show up correctly on the leaderboard, you can do this via `popcorn-cli register discord`. However in case this doesn't work for you we also support Github based authentication with `popcorn-cli register github`
4. To ensure the above worked you can run `cat $HOME/.popcorn.yaml` which should print your client ID which is what will be sent to us on every request

Sometimes you'll get an error that you're already authenticated despite being unable to submit in which case you can run `popcorn-cli reregister [discord|github]`.

## Make your first submission

```bash
wget https://raw.githubusercontent.com/gpu-mode/reference-kernels/refs/heads/main/problems/pmpp/grayscale_py/submission.py
popcorn-cli submit --gpu A100 --leaderboard grayscale --mode leaderboard submission.py
```

## Discover new problems

The CLI supports (almost) everything Discord does, so you can also discovery which leaderboards are available. To make discovery more pleasant we also offer a TUI experience.

```bash
popcorn-cli submit <submission-file>
```

glhf!
