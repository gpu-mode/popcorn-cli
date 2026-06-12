# Submit To The Linear Algebra QR Competition

First install and register Popcorn:

```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
popcorn register discord
```

Get the starter B200 QR submission:

```bash
curl -O https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/linalg/qr_py/submission.py
```

Run a correctness test:

```bash
popcorn submit --leaderboard qr --gpu B200 --mode test submission.py
```

Submit to the leaderboard:

```bash
popcorn submit --leaderboard qr --gpu B200 --mode leaderboard submission.py
```

Questions: ask in the `linalg` channel on [discord.gg/gpumode](https://discord.gg/gpumode).
