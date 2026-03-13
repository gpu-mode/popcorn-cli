# Helion Kernel Challenge

Run all the below instructions on your local laptop, you don't need to login to a GPU to make submissions to KernelBot.

Submit [Helion](https://github.com/pytorch/helion) kernels to the GPU MODE leaderboard on B200 GPUs. The challenge has 5 problems based on production LLM kernel patterns.

**Deadline:** March 14, 2026

**GPU:** B200_Nebius

## Problems

| # | Leaderboard Name | Description |
|---|-----------------|-------------|
| 1 | `causal_conv1d` | Causal depthwise 1D convolution (Mamba/Mamba-2) |
| 2 | `fp8_quant` | Per-token-group FP8 E4M3 quantization (DeepSeek-V3, Llama 3, Qwen3) |
| 3 | `gated_deltanet_chunk_fwd_h` | Inter-chunk state recurrence for Gated DeltaNet |
| 4 | `gated_deltanet_chunk_fwd_o` | Output computation for Gated DeltaNet |
| 5 | `gated_deltanet_recompute_w_u` | WY-transform forward kernel for Gated DeltaNet |

## Scoring

### Point Allocation

| Kernel | Correctness Points | Performance Points |
|---|---|---|
| **FP8 Quantization** | 100 | 0 (unscored) |
| **Causal Depthwise 1D Convolution** | 100 | 1000 |
| **Gated DeltaNet chunk\_fwd\_h** | 100 | 1000 |
| **Gated DeltaNet chunk\_fwd\_o** | 100 | 1000 |
| **Gated DeltaNet recompute\_w\_u** | 100 | 1000 |

### Scoring Rules

- **Performance Metric**: For each benchmark shape, the runtime is the geometric mean of 100 runs. The final runtime for a kernel is the geometric mean across all shapes, rounded to 4 significant figures.
- **Ranking**: Participants are ranked per kernel by runtime (fastest = rank 1).
- **Formula**: Score = CorrectnessPoints + (PerformancePoints × [1 − (rank - 1) / 10])
  - CorrectnessPoints are earned if the submission passes all test input shapes.
  - Only the top 10 performers per kernel (who pass all tests) can earn PerformancePoints.
  - Rank 1 → 100% of PerformancePoints, Rank 2 → 90%, …, Rank 10 → 10%.
- **Tiebreaker**: If two participants have the same metric value, the earlier submission wins.
- **Test case shapes**: Provided in `task.yml`; input data sampled from a random distribution.

**Total score** = Sum of points for all kernels.

## Rules & Requirements

- Kernel must pass all test input shapes (numerical accuracy within tolerance) with either default config or participant-provided config
- All benchmark shapes must have their best configs submitted for that kernel to be scored
- Implementations must use Helion DSL; `hl.inline_triton()`, `hl.triton_kernel()`, and `hl.inline_asm_elementwise()` are allowed but ≤30% LOC (excluding comments, measured after running `lint.sh fix`)
- Unlimited submissions per participant per kernel. Only your best submission counts. Each submission should include: your Helion kernel implementation, one config (or use default config) per test input shape, and one best autotuned config per benchmark input shape

## Quick Start

```bash
# 1. Install popcorn-cli
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash

# 2. Register
popcorn register discord

# 3. Join the challenge with your invite code
popcorn join <YOUR_INVITE_CODE>

# 4. Setup a project (downloads the submission template for you)
popcorn setup
# Select "Helion Kernel Challenge", then pick a problem and GPU
```

`popcorn setup` fetches the latest problems from reference-kernels, downloads the submission template with `#!POPCORN` directives pre-filled, and scaffolds agent skills for Codex/Claude Code.

Alternatively, you can clone the full reference-kernels repo to browse all problems locally:

```bash
git clone https://github.com/gpu-mode/reference-kernels.git
cd reference-kernels/problems/helion
```

Each problem directory (e.g. `causal_conv1d_py/`) contains:
- `reference.py` -- the reference implementation to beat
- `submission.py` -- your starting point
- `task.py` -- type definitions (`input_t`, `output_t`)
- `task.yml` -- input shapes, test cases, and benchmark configs

## Testing Locally

You can test and benchmark your submissions locally on your own GPU without submitting to KernelBot. This is useful for fast iteration during development.

From the `reference-kernels/problems/helion` directory, run:

```bash
# Correctness test (validates your kernel via CUDA graph capture)
python eval.py test causal_conv1d_py/

# Benchmark (measures kernel performance with L2 cache flushing)
python eval.py benchmark causal_conv1d_py/

# Both test and benchmark in one go
python eval.py both causal_conv1d_py/

# Profile (generates PyTorch profiler trace)
python eval.py profile causal_conv1d_py/
```

Replace `causal_conv1d_py/` with any problem directory.

## Writing a Helion Submission

Your submission must be a single Python file that defines `custom_kernel(data: input_t) -> output_t`. To use Helion, write a `@helion.kernel` decorated function and call it from `custom_kernel`.

Here's an example structure for `causal_conv1d`:

```python
from task import input_t, output_t
import torch
import helion
import helion.language as hl

@helion.kernel(config=helion.Config(
    block_sizes=[64, 64],
    num_warps=4,
    num_stages=3,
    # ... your tuned config here
))
def causal_conv1d_kernel(x: torch.Tensor, weight: torch.Tensor, bias: torch.Tensor) -> torch.Tensor:
    # Your Helion kernel implementation
    ...

def custom_kernel(data: input_t) -> output_t:
    x, weight, bias = data
    return causal_conv1d_kernel(x, weight, bias)
```

## Do NOT Autotune on KernelBot

When submitting to KernelBot, you must hardcode a single config in your `@helion.kernel` decorator. Do **not** rely on Helion's autotuner at submission time.

KernelBot runs your submission on shared infrastructure with timeouts. If your kernel triggers autotuning (which can take 10+ minutes and hundreds of trial runs), your submission will time out and fail.

### The correct workflow

1. **Autotune locally on your Nebius-provided B200 compute.** Run your Helion kernel without a fixed config (or with `autotune_effort="quick"`) to find the best configuration for the benchmark shapes.

2. **Copy the best config** from the autotuner output. Helion prints something like:
   ```
   One can hardcode the best config and skip autotuning with:
       @helion.kernel(config=helion.Config(block_sizes=[64, 64, 64], ...))
   ```

3. **Hardcode the config in your submission.** Pass it via `config=` in the `@helion.kernel` decorator:
   ```python
   @helion.kernel(config=helion.Config(
       block_sizes=[64, 64, 64],
       loop_orders=[[0, 1]],
       num_warps=8,
       num_stages=6,
       indexing='block_ptr',
       pid_type='flat',
       # ... rest of your tuned config
   ))
   def my_kernel(...):
       ...
   ```

4. **Submit the file** with the hardcoded config to KernelBot.

You can also use `autotune_effort="none"` during development to skip autotuning entirely and use the default config, but this will give worse performance.

## Submitting All 5 Problems

### Test first, then submit to leaderboard

```bash
# Test each problem (quick correctness check)
popcorn submit causal_conv1d_py/submission.py --gpu B200_Nebius --leaderboard causal_conv1d --mode test --no-tui
popcorn submit fp8_quant_py/submission.py --gpu B200_Nebius --leaderboard fp8_quant --mode test --no-tui
popcorn submit gated_deltanet_chunk_fwd_h_py/submission.py --gpu B200_Nebius --leaderboard gated_deltanet_chunk_fwd_h --mode test --no-tui
popcorn submit gated_deltanet_chunk_fwd_o_py/submission.py --gpu B200_Nebius --leaderboard gated_deltanet_chunk_fwd_o --mode test --no-tui
popcorn submit gated_deltanet_recompute_w_u_py/submission.py --gpu B200_Nebius --leaderboard gated_deltanet_recompute_w_u --mode test --no-tui

# Benchmark (see your perf without affecting the leaderboard)
popcorn submit causal_conv1d_py/submission.py --gpu B200_Nebius --leaderboard causal_conv1d --mode benchmark --no-tui

# Official leaderboard submission
popcorn submit causal_conv1d_py/submission.py --gpu B200_Nebius --leaderboard causal_conv1d --mode leaderboard --no-tui
popcorn submit fp8_quant_py/submission.py --gpu B200_Nebius --leaderboard fp8_quant --mode leaderboard --no-tui
popcorn submit gated_deltanet_chunk_fwd_h_py/submission.py --gpu B200_Nebius --leaderboard gated_deltanet_chunk_fwd_h --mode leaderboard --no-tui
popcorn submit gated_deltanet_chunk_fwd_o_py/submission.py --gpu B200_Nebius --leaderboard gated_deltanet_chunk_fwd_o --mode leaderboard --no-tui
popcorn submit gated_deltanet_recompute_w_u_py/submission.py --gpu B200_Nebius --leaderboard gated_deltanet_recompute_w_u --mode leaderboard --no-tui
```

### Using file directives

You can also embed the leaderboard and GPU in your submission file so you don't need CLI flags:

```python
#!POPCORN leaderboard causal_conv1d
#!POPCORN gpu B200_Nebius

from task import input_t, output_t
import torch
import helion
import helion.language as hl

@helion.kernel(config=helion.Config(...))
def causal_conv1d_kernel(...):
    ...

def custom_kernel(data: input_t) -> output_t:
    ...
```

Then submit with just:
```bash
popcorn submit causal_conv1d_py/submission.py
```

## Profiling

Nsight Compute profiling is available for Helion problems. Use `--mode profile` to get detailed GPU metrics:

```bash
popcorn submit causal_conv1d_py/submission.py --gpu B200_Nebius --leaderboard causal_conv1d --mode profile --no-tui
```

This returns GPU throughput, pipe utilization, and warp stall metrics, plus a downloadable `.ncu-rep` trace file you can open in the Nsight Compute GUI. See [profiling.md](profiling.md) for details on interpreting the output.

## Optional: Extra Performance Knobs

The sections below describe two **optional** techniques that can squeeze extra performance out of your kernels. Neither is required — you can place on the leaderboard without them. Try them after you have a working kernel with a tuned config.

### ACF Files

Each B200 instance has pre-tuned **PTXAS Advanced Controls Files (ACFs)** at `/opt/booster_pack/`. ACFs are low-level NVIDIA PTX assembler configurations that can improve performance beyond what Helion's standard autotuner finds. Available files:

```
/opt/booster_pack/
├── causal_conv_*.acf           (3 files)
├── chunk_fwd_h_*.acf           (2 files)
├── chunk_fwd_o_*.acf           (7 files)
├── fp8_group_quant_*.acf       (7 files)
└── recompute_w_u_fwd_*.acf     (5 files)
```

**Step 1: Autotune with ACFs.** Pass `autotune_search_acf` to include ACFs in the search space. Helion tries each ACF file (plus the default `-O3` baseline) as another tunable parameter:

```python
from pathlib import Path

acf_files = sorted(str(p) for p in Path("/opt/booster_pack").glob("causal_conv_*.acf"))

@helion.kernel(
    static_shapes=True,
    autotune_search_acf=acf_files,
)
def my_kernel(...):
    ...
```

> **Note:** `autotune_search_acf` only takes effect when the autotuner actually runs. It is ignored with `autotune_effort="none"` or a fixed `config=`.

**Step 2: Hardcode the best ACF in your submission.** After autotuning, look for the `advanced_controls_file` field in the best config and copy it:

```python
@helion.kernel(config=helion.Config(
    advanced_controls_file="/opt/booster_pack/causal_conv_0.acf",
    block_sizes=[1, 512],
    num_warps=4,
    num_stages=3,
    # ... rest of your tuned config
))
def my_kernel(...):
    ...
```

### TileIR Backend

The B200 instances also ship with **nvtriton**, NVIDIA's extended Triton compiler that includes a **TileIR** backend — an alternative compilation pipeline that bypasses LLVM and compiles directly to CUBIN via NVIDIA's `tileiras` compiler.

| | `ENABLE_TILE=0` (default) | `ENABLE_TILE=1` |
|---|---|---|
| **Helion backend** | `triton` | `tileir` |

**Step 1: Enable TileIR and autotune.** Set the env vars before importing Helion, then autotune as usual. Helion automatically adjusts the search space for the TileIR backend.

**Step 2: Hardcode the TileIR config in your submission.** Copy the best config from the autotuner output (it will include TileIR-specific fields like `num_ctas` and `occupancy`). The env vars must be set before imports:

```python
import os
os.environ["ENABLE_TILE"] = "1"
os.environ["HELION_BACKEND"] = "tileir"

import helion  # must be imported after setting env vars
import helion.language as hl

@helion.kernel(config=helion.Config(
    block_sizes=[64, 64],
    num_ctas=1,
    num_stages=5,
    occupancy=4,
    # ... rest of your tuned config
))
def my_kernel(...):
    ...
```

### Which should I use?

Try both `ENABLE_TILE=0` and `ENABLE_TILE=1`, with and without ACFs, then submit whichever gives the best benchmark numbers.

## Tips

- **Iterate locally first.** Use your Nebius B200 to develop and autotune. Only submit to KernelBot once you have a hardcoded config that works.
- **Check the reference.** Each `reference.py` shows the baseline implementation you're trying to beat. Understanding it helps you write a better kernel.
- **Use `--mode test` first.** Verify correctness before submitting to the leaderboard. This saves time and leaderboard quota.
- **Profile your kernels.** Use `--mode profile` to get Nsight Compute metrics and identify bottlenecks.
- **One config per submission.** If Helion found different best configs for different benchmark shapes, pick the one that works best across all of them -- the leaderboard uses geometric mean across benchmarks.
## Open-Ended Contribution Track

In addition to the kernel competition, there is a separate open-ended contribution track. Participants can earn recognition and prizes for contributions to Helion beyond kernel implementations. This track is scored independently and does not affect kernel competition standings. Examples:

| Contribution Type | Description |
|---|---|
| Autotuner Improvements | Enhancements to Helion's autotuning system |
| Bug Fixes | Bug fixes in Helion |
| Tooling/Infrastructure | Improvements to debugging, profiling, or developer experience |
| Documentation | Significant documentation contributions |
| Other Novel Contributions | Other impactful contributions at judges' discretion |

Contributions are uncapped and evaluated by a panel of judges based on impact and quality. Prizes for this track are awarded separately from the kernel competition.

## Resources

- [Helion Documentation](https://helionlang.com)
- [Helion GitHub](https://github.com/pytorch/helion)
- [Reference Kernels](https://github.com/gpu-mode/reference-kernels/tree/main/problems/helion)
- [GPU MODE Discord](https://discord.gg/gpumode)
