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
   popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test submission.py
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

### Windows execution policy error
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Permission denied (Linux/macOS)
Make the script executable:
```bash
chmod +x install.sh
./install.sh
```

## 🖥️ Operating System Support

| OS | Script | Requirements |
|---|---|---|
| **Linux** | `install.sh` | `curl` or `wget`, `tar` |
| **macOS** | `install.sh` | `curl` or `wget`, `tar` |
| **Windows** | `install.ps1` | PowerShell 5.1+ |
| **Windows WSL** | `install.sh` | `curl` or `wget`, `tar` |
| **Git Bash** | `install.sh` | `curl` or `wget`, `tar` |

## 🎯 Hackathon Features

This hackathon version includes:

- ✅ **Pre-configured API URL** - No need to get `/get-api-url` from Discord
- ✅ **GitHub authentication** - Simple OAuth flow, no Discord setup required
- ✅ **All modes available** - test, benchmark, leaderboard, profile
- ✅ **Auto-PATH setup** - Binary automatically added to your PATH
- ✅ **Cross-platform** - Works on Linux, macOS, and Windows

## 💡 Need Help?

- Run `popcorn-cli --help` for usage information
- Check the [main repository](https://github.com/gpu-mode/popcorn-cli) for issues
- Join the [GPU Mode Discord](https://discord.gg/gpumode) for support 