# Nsight Compute Profiling

This profiles GPU Mode submissions on the hosted B200 Nsight Compute service and
downloads agent-readable `ncu-details.txt` / `ncu-details.csv` artifacts. The
full `.ncu-rep` GUI report is still included for local inspection.

The profiler uses the `benchmarks:` list from the active `reference-kernels`
checkout. `--benchmark-index N` profiles `benchmarks[N]`; omitting
`--benchmark-index` profiles every benchmark entry for that leaderboard.

## 1. Install and Register

```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
popcorn register discord
```

Restart your terminal if `popcorn` is not found after installation.

## 2. Set the Hosted Profiler URL

```bash
export POPCORN_BREV_PROFILER_URL=https://http--brev-profiler-proxy--dxfjds728w5v.code.run
```

`BREV_PROFILER_URL` is also accepted as a fallback, but
`POPCORN_BREV_PROFILER_URL` is preferred.

## 3. Profile QR v2

Get the QR v2 starter submission:

```bash
mkdir -p qr-v2-profile
cd qr-v2-profile
curl -O https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/linalg/qr_v2/submission.py
```

Profile one benchmark shape:

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

## 4. Profile Eigh

Get the `eigh` starter submission:

```bash
mkdir -p eigh-profile
cd eigh-profile
curl -O https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/linalg/eigh_py/submission.py
```

Profile the dense `n=512` leverage row:

```bash
popcorn submit submission.py \
  --leaderboard eigh \
  --profile-brev \
  --benchmark-index 3 \
  --no-tui
```

The hosted profiler uses a deeper Nsight Compute launch window for `eigh` than
for QR v2 so PyTorch/cuSOLVER submissions can reach solver-path kernels after
clone/setup launches.

Current `eigh` benchmark index table from `reference-kernels` main
`4a1153e`:

| Index | Shape |
| ---: | --- |
| 0 | `batch: 20; n: 32; cond: 1; seed: 43214` |
| 1 | `batch: 40; n: 176; cond: 1; seed: 423011` |
| 2 | `batch: 40; n: 352; cond: 1; seed: 123456` |
| 3 | `batch: 640; n: 512; cond: 2; seed: 1029` |
| 4 | `batch: 60; n: 1024; cond: 2; seed: 75342` |
| 5 | `batch: 8; n: 2048; cond: 1; seed: 224466` |
| 6 | `batch: 640; n: 512; cond: 2; seed: 770001; case: mixed` |
| 7 | `batch: 60; n: 1024; cond: 2; seed: 770002; case: mixed` |
| 8 | `batch: 640; n: 512; cond: 0; seed: 770003; case: rankdef` |
| 9 | `batch: 640; n: 512; cond: 0; seed: 770004; case: clustered` |
| 10 | `batch: 60; n: 1024; cond: 0; seed: 770005; case: nearrank` |
| 11 | `batch: 640; n: 512; cond: 0; seed: 780001; case: lapack_dense_even_spectrum` |
| 12 | `batch: 60; n: 1024; cond: 0; seed: 780007; case: lapack_dense_geometric_spectrum` |

## 5. Read the Details

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

## Profile All Benchmark Shapes

Omit `--benchmark-index`:

```bash
popcorn submit submission.py \
  --leaderboard eigh \
  --profile-brev \
  --no-tui
```

This profiles every entry in the leaderboard's `benchmarks:` list, not the
`tests:` list. It will produce one zip plus extracted details and optional
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
