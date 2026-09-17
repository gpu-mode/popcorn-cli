# Nsight Compute Profiling

The default profiler runs GPU Mode submissions in your Modal account on B200 and
downloads agent-readable `ncu-details.txt` / `ncu-details.csv` artifacts. The
full `.ncu-rep` GUI report is still included for local inspection.

The profiler uses the `benchmarks:` list from the active `reference-kernels`
checkout. `--benchmark-index N` profiles `benchmarks[N]`; omitting
`--benchmark-index` profiles every benchmark entry for that leaderboard.

## Supported problems

The task evaluator must implement `profile` mode and launch the submission in
an NVTX push/pop range named `custom_kernel`. A PyTorch-profiler-only path is
not sufficient. QR, QR v2, Eigh, Cholesky, and the shared NVIDIA evaluator have
the required NCU path; only QR v2 has been tested end to end for this CLI change.
Check the actual evaluator selected by the task, since task-specific copies may
have different support. AMD and multi-GPU NCU profiling are unsupported.

Problem authors can follow the reference-kernels
[NCU integration guide](https://github.com/gpu-mode/reference-kernels/blob/docs/ncu-problem-guide/docs/ncu-profiling.md).

## 1. Install Popcorn and Modal

```bash
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash
pip install modal
modal setup
```

`--profile` and `--mode profile` use your own Modal
account and imply plain output. You do not need Popcorn registration or a
profiler URL. Modal bills this GPU run to your configured workspace. Use
`--gpu` to override B200, or put a GPU directive in the submission.

## 2. Source versions and artifacts

Each invocation resolves the latest `reference-kernels` and `kernelbot` main
commits. You can pin them with `POPCORN_REFERENCE_KERNELS_REF` and
`POPCORN_KERNELBOT_REF`. Every profile saves a `manifest.json` with both source
refs, the problem directory, GPU/system information, selected benchmark specs,
and NCU capture options alongside the reports in a unique `popcorn-profile-*`
directory. `--output` saves the text summary; the artifacts remain in that directory.

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
  --profile \
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
  --profile \
  --benchmark-index 3 \
  --no-tui
```

The default capture window is 10 kernel launches per benchmark. For late solver
kernels, select a kernel name or increase `--ncu-launch-count`.

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

## 5. Profile Cholesky

Get the `cholesky` starter submission:

```bash
mkdir -p cholesky-profile
cd cholesky-profile
curl -O https://raw.githubusercontent.com/gpu-mode/reference-kernels/main/problems/linalg/cholesky_py/submission.py
```

Profile the `batch=4096, n=32` benchmark:

```bash
popcorn submit submission.py \
  --leaderboard cholesky \
  --profile \
  --benchmark-index 0 \
  --no-tui
```

Capture late kernels with the same filters on any leaderboard:

```bash
popcorn submit submission.py --leaderboard cholesky --profile --benchmark-index 0 \
  --ncu-kernel-name 'regex:my_kernel' --ncu-kernel-name-base demangled \
  --ncu-launch-count 2
```

NCU follows child processes, captures the evaluator's `custom_kernel` NVTX range,
and leaves GPU clocks unchanged. Empty or failed captures return an error.
Multi-GPU profiling is unsupported.

## 6. Read the Details

After the run finishes, the CLI downloads and extracts files like:

```text
popcorn-profile-<run>/profile-0.zip
popcorn-profile-<run>/profile-0/ncu-details.txt
popcorn-profile-<run>/profile-0/ncu-details.csv
popcorn-profile-<run>/profile-0/profile.ncu-rep   # optional GUI report
```

Use `ncu-details.txt` or `ncu-details.csv` as the default artifact for AI
analysis. The CLI prints local paths for the detail files and report.

Open the GUI report on macOS:

```bash
open -a "NVIDIA Nsight Compute" 'popcorn-profile-<run>/profile-0/profile.ncu-rep'
```

## Profile All Benchmark Shapes

Omit `--benchmark-index`:

```bash
popcorn submit submission.py \
  --leaderboard eigh \
  --profile \
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

## Explicit Brev profiling

Use `--profile-brev` to select the hosted Brev service explicitly. `--profile`
uses Modal only; errors never trigger a switch to Brev. Brev requires Popcorn
registration:

```bash
popcorn register discord
export POPCORN_BREV_PROFILER_URL=https://http--brev-profiler-proxy--dxfjds728w5v.code.run
popcorn submit submission.py --leaderboard qr_v2 --profile-brev --benchmark-index 0
```

`BREV_PROFILER_URL` is also accepted. Brev profiling uses the service's deployed
reference-kernels checkout; Modal source-ref overrides do not apply to it.
