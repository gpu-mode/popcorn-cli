# Nsight Compute Profiling

Profile your kernels directly from the CLI and get detailed Nsight Compute metrics. This is particularly useful for the NVIDIA NVFP4 Blackwell competition where you need to optimize tensor core utilization.

**Note:** Modal does not expose NCU. For Modal-ranked competitions, use the Brev-backed B200 profiler below.

## Quick Start

```bash
popcorn-cli submit submission.py --leaderboard nvfp4_dual_gemm --gpu NVIDIA --mode profile --no-tui
```

For competitions whose ranked runs use Modal, use the Brev-backed B200 profiler:

```bash
POPCORN_BREV_PROFILER_URL=http://127.0.0.1:8765 popcorn-cli submission.py --profile-brev
```

`--profile-brev` requires `POPCORN_BREV_PROFILER_URL` or `BREV_PROFILER_URL`.
The endpoint should be a local/staging profiler or a hardened shared service.
Do not expose a shared Brev profiler to untrusted users until submissions run
in a per-job container or equivalent locked-down environment with no SSH keys,
operator secrets, or other users' submissions mounted.

This uses the `#!POPCORN leaderboard ...` directive in `submission.py`. If the file does not include a leaderboard directive, pass one explicitly:

```bash
POPCORN_BREV_PROFILER_URL=http://127.0.0.1:8765 popcorn-cli submit submission.py --leaderboard grayscale_v2 --profile-brev
```

For a quick single-shape QR profile:

```bash
POPCORN_BREV_PROFILER_URL=http://127.0.0.1:8765 popcorn-cli submit submission.py --leaderboard qr --profile-brev --benchmark-index 0
```

## Expected Output

The profiler returns three key metric tables for each benchmark:

**GPU Throughput** - Overall utilization:
```
Metric Name      Metric Unit Metric Value
---------------- ----------- ------------
Memory [%]                 %        32.48
Compute (SM) [%]           %        13.23
```

**Pipe Utilization** - Which pipelines are active:
```
Metric Name          Metric Unit Metric Value
-------------------- ----------- ------------
TC                             %        16.67
TMEM (Tensor Memory)           %        15.27
Tensor (FP)                    %        12.58
ALU                            %         2.38
TMA                            %         0.29
```

**Warp State** - Where your warps are stalling:
```
Metric Name              Metric Unit Metric Value
------------------------ ----------- ------------
Stall Long Scoreboard           inst        18.31
Stall Wait                      inst         1.88
Stall Short Scoreboard          inst         1.23
Selected                        inst         1.00
Stall Barrier                   inst         0.75
```

## Trace Files

After profiling, a zip file is saved to your current directory and the `.ncu-rep`
file is extracted next to it:
```
profile.0-batch-20-n-32-cond-1-seed-43214.zip
profile.0-batch-20-n-32-cond-1-seed-43214/profile.ncu-rep
```

This contains a `.ncu-rep` file (the full Nsight Compute report):
```
$ unzip -l profile.0-batch-20-n-32-cond-1-seed-43214.zip
  Length      Date    Time    Name
---------  ---------- -----   ----
  2178383  01-13-2026 03:10   profile.ncu-rep
```

The CLI prints a clickable terminal link to the extracted report and makes the
last line a macOS command that opens it in Nsight Compute:
```bash
open -a "NVIDIA Nsight Compute" profile.0-batch-20-n-32-cond-1-seed-43214/profile.ncu-rep
```

## Operator Notes

The CLI does not assume a Brev provider username or home directory. Configure
the profiler service with explicit paths, or derive them from `$HOME` on the
Brev machine, instead of hardcoding paths such as `/home/<user>`.

For SSH access, prefer a dedicated restricted SSH key for the profiler proxy.
If you use the Brev CLI to maintain host metadata, run `brev refresh` once and
then use normal `ssh`/`scp` against the refreshed host alias. Avoid putting
`brev shell` or `brev copy` in per-job paths because they refresh each time.

The Brev worker should run each untrusted `submission.py` inside a container or
similarly isolated runtime before a public profiler endpoint is enabled.
Container isolation is not a complete sandbox, but it materially reduces the
risk of submissions reading host secrets, SSH keys, or other submissions.
