# Popcorn CLI

A command-line interface tool for submitting solutions to the [gpumode.com](https://gpumode.com)
<img width="1034" alt="Screenshot 2025-06-10 at 11 17 45 AM" src="https://github.com/user-attachments/assets/66414f12-a984-4a3d-b035-d31f8695a54d" />

Tested on linux and mac but should just work on Windows as well.

## New: Nsight Compute Profiling

Profile your kernels with `--mode profile` and get detailed metrics. Currently only available for the NVFP4 Blackwell competition (Modal, which we use for other competitions, does not support NCU). See [docs/profiling.md](docs/profiling.md) for details.

## Installation

### Option 1: One-Line Install (Recommended)

**Linux/macOS/Unix:**
```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
powershell -ExecutionPolicy Bypass -Command "iwr -UseBasicParsing https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.ps1 | iex"
```

After installation, restart your terminal (or run `source ~/.bashrc` / `source ~/.zshrc`).

### Option 2: Manual Installation

1. Download the binary for your OS from [releases](https://github.com/gpu-mode/popcorn-cli/releases/latest)
2. Extract the archive
3. Move the binary to a directory in your PATH
4. Make it executable (Linux/macOS): `chmod +x popcorn-cli`

### Option 3: Building from source

1. Download rust `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. `cd popcorn-cli && ./build.sh`

### Troubleshooting

**Command not found after installation:**
- Restart your terminal
- Check if the install directory is in your PATH:
  - Linux/macOS: `echo $PATH`
  - Windows: `echo $env:PATH`
- Check if POPCORN_API_URL is set to https://discord-cluster-manager-1f6c4782e60a.herokuapp.com
  - Linux/macOS: `echo $POPCORN_API_URL`
  - Windows: `echo $env:POPCORN_API_URL`

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

We regularly run competitions with clear due dates but for beginners we will always keep open the PMPP_v2 problem set https://github.com/gpu-mode/reference-kernels/tree/main/problems/pmpp_v2 

## Submission Format

Submissions are always a single Python file. If you want to submit native CUDA code, you can use PyTorch's `load_inline` feature (which uses nvcc) or the more experimental [`compile_kernel` API](https://x.com/gaunernst/status/2015242181049745607) for fast compilation. See [this example](https://github.com/gpu-mode/reference-kernels/blob/main/problems/pmpp_v2/vectoradd_py/solutions/correct/submission_cuda_inline.py) for reference.

For syntax highlighting of both C++ and Python in your IDE, you can use the [PyTorch Load Inline Highlighter](https://marketplace.visualstudio.com/items?itemName=msaroufim.pytorch-load-inline-highlighter) VS Code extension.

## Reference Kernels

All reference kernels are available at [gpu-mode/reference-kernels](https://github.com/gpu-mode/reference-kernels). Each problem directory contains:
- `reference.py` - The reference implementation to beat
- `submission.py` - A sample submission you can use as a starting point
- `task.yml` - Input shapes and problem configuration

Our entire evaluation infrastructure is open source and you can learn more [here](https://github.com/gpu-mode/kernelbot). Development happens on the [KernelBot discord](https://discord.gg/FjYsdHDv7J)

## Stay Updated

Interested in new kernel competitions? Join [discord.gg/gpumode](https://discord.gg/gpumode) and check out the **#announcements** channel to be notified when new challenges drop.

## Discover Problems

The CLI supports everything Discord does, so you can also discover which leaderboards are available. To make discovery more pleasant we also offer a TUI experience.

```bash
popcorn-cli submit <submission-file>
```

glhf!
