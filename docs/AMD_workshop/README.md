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

## 🏃 Run Examples

Try out the example implementations to get familiar with the system:

### For Linux/macOS:
```bash
# Download and test v1.py (reference implementation)
wget https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/docs/AMD_workshop/v1.py
popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test v1.py

# Download and test v2.py (basic optimization)
wget https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/docs/AMD_workshop/v2.py
popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test v2.py

# Download and test v3.py (advanced optimization)
wget https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/docs/AMD_workshop/v3.py
popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test v3.py
```

### For Windows (PowerShell):
```powershell
# Download and test v1.py (reference implementation)
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/docs/AMD_workshop/v1.py" -OutFile "v1.py"
popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test v1.py

# Download and test v2.py (basic optimization)
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/docs/AMD_workshop/v2.py" -OutFile "v2.py"
popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test v2.py

# Download and test v3.py (advanced optimization)
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/docs/AMD_workshop/v3.py" -OutFile "v3.py"
popcorn-cli submit --gpu MI300 --leaderboard amd-fp8-mm --mode test v3.py
```

### 💡 Pro Tips:
- Start with **v1.py** (reference implementation) to understand the baseline
- Try **v2.py** for basic optimizations
- Challenge yourself with **v3.py** for advanced Triton optimizations
- Use `--mode benchmark` instead of `--mode test` to see performance metrics


## 🛠️ Manual Installation

If the scripts don't work, you can manually install:

1. Download the binary for your OS from [releases](https://github.com/gpu-mode/popcorn-cli/releases/latest)
2. Extract the archive
3. Move the binary to a directory in your PATH
4. Make it executable (Linux/macOS): `chmod +x popcorn-cli`

## 🆘 Troubleshooting

### Command not found after installation
- Restart your terminal
- Check if the install directory is in your PATH:
  - Linux/macOS: `echo $PATH`
  - Windows: `echo $env:PATH`
- Check if POPCORN_API_URL is set to https://site--bot--dxfjds728w5v.code.run
  - Linux/macOS: `echo $POPCORN_API_URL`
  - Windows: `echo $env:POPCORN_API_URL`

## 💡 Need Help?

- Run `popcorn-cli --help` for usage information
- Check the [main repository](https://github.com/gpu-mode/popcorn-cli) and open an issue
- Join the [GPU Mode Discord](https://discord.gg/gpumode) and ask a question in #amd-competition

## 🧑‍🎓 Learn more from our favorite writeups

* https://github.com/luongthecong123/fp8-quant-matmul
* https://seb-v.github.io/optimization/update/2025/01/20/Fast-GPU-Matrix-multiplication.html
* https://akashkarnatak.github.io/amd-challenge/
* https://www.bilibili.com/read/cv41954307/?opus_fallback=1 
* https://github.com/Snektron/gpumode-amd-fp8-mm
