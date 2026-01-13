# Nsight Compute Profiling

Profile your kernels directly from the CLI and get detailed Nsight Compute metrics. This is particularly useful for the NVIDIA NVFP4 Blackwell competition where you need to optimize tensor core utilization.

**Note:** Profiling is currently only available for the NVFP4 Blackwell competition. Modal, which we use for other competitions, does not support NCU.

## Quick Start

```bash
popcorn-cli submit submission.py --leaderboard nvfp4_dual_gemm --gpu NVIDIA --mode profile --no-tui
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

After profiling, a zip file is saved to your current directory:
```
profile_20260113_031052_run0.zip
```

This contains a `.ncu-rep` file (the full Nsight Compute report):
```
$ unzip -l profile_20260113_031052_run0.zip
  Length      Date    Time    Name
---------  ---------- -----   ----
  2178383  01-13-2026 03:10   profile.ncu-rep
```

You can open this file in the Nsight Compute GUI for detailed analysis:
```bash
ncu-ui profile.ncu-rep
```
