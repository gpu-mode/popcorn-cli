# QR v2 Nsight Compute Profiling

This profiles the GPU Mode QR v2 problem from `reference-kernels` and downloads
Nsight Compute details that AI agents can read directly. The full `.ncu-rep`
GUI report is still included for local inspection.

## 1. Install and Register

```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
popcorn register discord
```

Restart your terminal if `popcorn` is not found after installation.

## 2. Get the QR v2 Starter Submission

```bash
mkdir -p qr-v2-profile
cd qr-v2-profile
curl -O https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/linalg/qr_v2/submission.py
```

The profiler uses the hosted GPU Mode NCU service:

```bash
export POPCORN_BREV_PROFILER_URL=https://http--brev-profiler-proxy--dxfjds728w5v.code.run
```

## 3. Profile One QR v2 Shape

This profiles `benchmarks[0]` from
`reference-kernels/problems/linalg/qr_v2/task.yml`:

```bash
popcorn submit submission.py \
  --leaderboard qr_v2 \
  --profile-brev \
  --benchmark-index 0 \
  --no-tui
```

The first QR v2 benchmark shape is:

```text
batch: 20; n: 32; cond: 1; seed: 43214
```

## 4. Read the Details

After the run finishes, the CLI downloads and extracts files like:

```text
profile.0-batch-20-n-32-cond-1-seed-43214.zip
profile.0-batch-20-n-32-cond-1-seed-43214/ncu-details.txt
profile.0-batch-20-n-32-cond-1-seed-43214/ncu-details.csv
profile.0-batch-20-n-32-cond-1-seed-43214/profile.ncu-rep   # optional GUI report
```

Use `ncu-details.txt` or `ncu-details.csv` as the default artifact for AI
analysis. The CLI prints clickable links for these detail files.

The last line printed by the CLI opens the optional GUI report on macOS:

```bash
open -a "NVIDIA Nsight Compute" 'profile.0-batch-20-n-32-cond-1-seed-43214/profile.ncu-rep'
```

## Profile All QR v2 Benchmark Shapes

Omit `--benchmark-index`:

```bash
popcorn submit submission.py \
  --leaderboard qr_v2 \
  --profile-brev \
  --no-tui
```

This profiles every entry in the `benchmarks:` list in QR v2 `task.yml`, not
the `tests:` list. It will produce one zip plus extracted details and optional
`.ncu-rep` files per benchmark shape.

## Normal Submit Commands

For correctness testing:

```bash
popcorn submit submission.py --leaderboard qr_v2 --gpu B200 --mode test --no-tui
```

For leaderboard submission:

```bash
popcorn submit submission.py --leaderboard qr_v2 --gpu B200 --mode leaderboard --no-tui
```
