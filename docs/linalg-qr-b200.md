# Submit To The Linear Algebra QR v2 Competition

First install and register Popcorn:

```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
popcorn register discord
```

Get the starter B200 QR v2 submission:

```bash
curl -O https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/linalg/qr_v2/submission.py
```

Run a correctness test:

```bash
popcorn submit --leaderboard qr_v2 --gpu B200 --mode test submission.py
```

Profile the first benchmark shape with Nsight Compute:

```bash
popcorn register discord
popcorn submit --leaderboard qr_v2 --profile --benchmark-index 0 submission.py
```

The CLI downloads a `.zip` and extracts `ncu-details.txt`, `ncu-details.csv`,
and the optional `profile.ncu-rep` GUI report. See [profiling.md](profiling.md)
for the complete QR v2 profiling flow.

Submit to the leaderboard:

```bash
popcorn submit --leaderboard qr_v2 --gpu B200 --mode leaderboard submission.py
```

Questions: ask in the `linalg` channel on [discord.gg/gpumode](https://discord.gg/gpumode).
