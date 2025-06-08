# 🍿 Popcorn CLI - Hackathon Quick Install

Get started with Popcorn CLI in seconds! Choose your installation method based on your operating system.

## 🚀 One-Line Install Commands

### For Linux/macOS/Unix:
```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
```

### For Windows (PowerShell):
```powershell
powershell -ExecutionPolicy Bypass -Command "iwr -UseBasicParsing https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.ps1 | iex"
```

## 📋 Quick Start After Installation

1. **Restart your terminal** (or run `source ~/.bashrc` / `source ~/.zshrc`)

2. **Register with GitHub** (one-time setup):
   ```bash
   popcorn-cli register github
   ```

3. **Submit your solution:**
   ```bash
   popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test example.py
   ```
   
4. **Interactive mode** (choose GPU and options):
   ```bash
   popcorn-cli submit my_solution.py
   ```

## 🛠️ Manual Installation

If the scripts don't work, you can manually install:

1. Download the binary for your OS from [releases](https://github.com/gpu-mode/popcorn-cli/releases/tag/v1.1.6)
2. Extract the archive
3. Move the binary to a directory in your PATH
4. Make it executable (Linux/macOS): `chmod +x popcorn-cli`

## 🆘 Troubleshooting

### Command not found after installation
- Restart your terminal
- Check if the install directory is in your PATH:
  - Linux/macOS: `echo $PATH`
  - Windows: `echo $env:PATH`
- Check if POPCORN_API_URL is set to https://discord-cluster-manager-1f6c4782e60a.herokuapp.com
  - Linux/macOS: `echo $POPCORN_API_URL`
  - Windows: `echo $env:POPCORN_API_URL`

## 💡 Need Help?

- Run `popcorn-cli --help` for usage information
- Check the [main repository](https://github.com/gpu-mode/popcorn-cli) and open an issue
- Join the [GPU Mode Discord](https://discord.gg/gpumode) and ask a question in #amd-competition