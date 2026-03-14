# Helion Kernel Challenge

Run all the below instructions on your local laptop, you don't need to login to a GPU to make submissions to KernelBot.

Submit [Helion](https://github.com/pytorch/helion) kernels to the GPU MODE leaderboard on B200 GPUs. The challenge has 5 problems based on production LLM kernel patterns.

**Deadline:** March 14, 2026

**GPU:** B200_Nebius

## Problems

| # | Leaderboard Name | Description |
|---|-----------------|-------------|
| 1 | `fp8_quant` | Per-token-group FP8 E4M3 quantization (DeepSeek-V3, Llama 3, Qwen3) |
| 2 | `causal_conv1d` | Causal depthwise 1D convolution (Mamba/Mamba-2) |
| 3 | `gated_deltanet_chunk_fwd_h` | Inter-chunk state recurrence for Gated DeltaNet |
| 4 | `gated_deltanet_chunk_fwd_o` | Output computation for Gated DeltaNet |
| 5 | `gated_deltanet_recompute_w_u` | WY-transform forward kernel for Gated DeltaNet |

## Scoring

Each scored problem awards points to the **top 3** fastest correct submissions:

| Place | Points |
|---|---|
| 1st | 5 |
| 2nd | 3 |
| 3rd | 1 |

> **Note:** Problem 1 (`fp8_quant`) is **not scored** — it is a warm-up problem only. Points are awarded for problems 2–5.

- **Performance Metric**: For each benchmark shape, the kernel is captured in a CUDA graph and replayed with L2 cache clearing before each invocation. The graph unrolls enough calls to fill ~100ms of GPU time, and this is repeated 10 times. The runtime is the arithmetic mean of those 10 measurements.
- **Correctness**: Submissions must pass all test input shapes to be eligible for points.
- **Tiebreaker**: If two participants have the same metric value, judges will decide based on the quality of the kernel.
- **Test case shapes**: Provided in `task.yml`; input data sampled from a random distribution.

**Total score** = Sum of points across problems 2–5 (max 20).

## Rules & Requirements

- Kernel must pass all test input shapes (numerical accuracy within tolerance) with participant-provided config
- All benchmark shapes must have their best configs submitted for that kernel to be scored
- Implementations must use Helion DSL. `hl.inline_triton()`, `hl.triton_kernel()`, and `hl.inline_asm_elementwise()` are allowed as escape hatches, but the majority of your kernel should be written in Helion. Submissions that are predominantly inline Triton/ASM with a thin Helion wrapper may be disqualified at judges' discretion
- Unlimited submissions per participant per kernel. Only your best submission counts. Each submission should include: your Helion kernel implementation, one config per test input shape, and one best autotuned config per benchmark input shape

## Quick Start

```bash
# 1. Install popcorn-cli
curl -fsSL https://raw.githubusercontent.com/gpu-mode/popcorn-cli/main/install.sh | bash

# 2. Register
popcorn register discord
# Please click on the link to authenticate on Discord

# 3. Join the challenge with your invite code
popcorn join <YOUR_INVITE_CODE>

# 4. Setup a project (downloads the submission template for you)
popcorn setup
# Select "Helion Kernel Challenge", then pick a problem and GPU
```

`popcorn setup` fetches the latest problems from reference-kernels, creates a project folder named after the selected problem (e.g. `causal_conv1d_py/`), downloads the submission template with `#!POPCORN` directives pre-filled, and scaffolds agent skills for Codex/Claude Code. If a folder with that name already exists, a `-N` suffix is appended (e.g. `causal_conv1d_py-1/`).

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

Use **per-shape configs** to optimize for each benchmark shape independently. The per-shape config pattern uses a factory function to create kernel variants with different configs, and dispatches based on input tensor shapes:

```python
from task import input_t, output_t
import torch
import helion
import helion.language as hl

# Map input shapes to optimized configs (autotune each shape locally).
# Include all test and benchmark shapes from task.yml.
SHAPE_CONFIGS: dict[tuple, helion.Config] = {
    # Test shapes
    (1, 64, 64, 4): helion.Config(...),  # TODO: replace with default config or any config that passes correctness check
    (2, 128, 128, 4): helion.Config(...),  # TODO: replace with default config or any config that passes correctness check
    # ... one entry per test shape
    # Benchmark shapes
    (1, 768, 512, 4): helion.Config(...),  # TODO: replace with your autotuned config
    (1, 768, 2048, 4): helion.Config(...),  # TODO: replace with your autotuned config
    # ... one entry per benchmark shape
}


def _make_kernel(config: helion.Config):
    @helion.kernel(static_shapes=True, config=config)
    def causal_conv1d_kernel(x: torch.Tensor, weight: torch.Tensor, bias: torch.Tensor) -> torch.Tensor:
        # Your Helion kernel implementation
        ...

    return causal_conv1d_kernel


_KERNELS = {shape: _make_kernel(cfg) for shape, cfg in SHAPE_CONFIGS.items()}


def custom_kernel(data: input_t) -> output_t:
    x, weight, bias = data
    B, D, S = x.shape
    W = weight.shape[1]
    kernel = _KERNELS[(B, D, S, W)]
    return kernel(x, weight, bias)
```

## Do NOT Autotune on KernelBot

When submitting to KernelBot, you must hardcode configs in your `@helion.kernel` decorator. Do **not** rely on Helion's autotuner at submission time.

KernelBot runs your submission on shared infrastructure with timeouts. If your kernel triggers autotuning (which can take 10+ minutes and hundreds of trial runs), your submission will time out and fail.

### Getting a default config (no autotuning)

During early development, you can use `autotune_effort="none"` to skip autotuning and use Helion's default config. When you run the kernel, Helion prints the default config to stderr:

```
Using default config: @helion.kernel(config=helion.Config(block_sizes=[64, 64], num_warps=4, num_stages=1), static_shapes=True)
```

Copy the `helion.Config(...)` portion into your `SHAPE_CONFIGS` dict. The default config is usually good enough for test input shapes to pass correctness checks, but won't be competitive for benchmark shapes on the leaderboard.

### Autotuning for benchmark shapes

1. **Autotune locally on your Nebius-provided B200 compute.** Run your Helion kernel without a fixed config (or with `autotune_effort="quick"`) to find the best configuration for each benchmark shape.

2. **Copy the best config** from the autotuner output. When autotuning completes, Helion prints:
   ```
   One can hardcode the best config and skip autotuning with:
       @helion.kernel(config=helion.Config(block_sizes=[64, 64, 64], num_warps=8, num_stages=3))
   ```

3. **Hardcode the config in your submission.** Copy the `helion.Config(...)` from step 2 into the corresponding benchmark shape entry in `SHAPE_CONFIGS`. Repeat steps 1-3 for each benchmark shape in `task.yml`.

4. **Submit the file** with the hardcoded configs to KernelBot.

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

| | `ENABLE_TILE=0` (default) | `ENABLE_TILE=1` + `HELION_BACKEND=tileir` |
|---|---|---|
| **Helion backend** | `triton` | `tileir` |

**Step 1: Enable TileIR and autotune.** Set both `ENABLE_TILE=1` and `HELION_BACKEND=tileir` env vars before importing Helion, then autotune as usual. Helion automatically adjusts the search space for the TileIR backend.

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

Try both the default backend (`ENABLE_TILE=0`) and the TileIR backend (`ENABLE_TILE=1` + `HELION_BACKEND=tileir`), with and without ACFs, then submit whichever gives the best benchmark numbers.

## Tips

- **Iterate locally first.** Use your Nebius B200 to develop and autotune. Only submit to KernelBot once you have a hardcoded config that works.
- **Check the reference.** Each `reference.py` shows the baseline implementation you're trying to beat. Understanding it helps you write a better kernel.
- **Use `--mode test` first.** Verify correctness before submitting to the leaderboard. This saves time and leaderboard quota.
- **Profile your kernels.** Use `--mode profile` to get Nsight Compute metrics and identify bottlenecks.
- **One config per shape.** Use the per-shape config pattern to provide an optimized config for each benchmark shape in `task.yml`.

## Working on Your GPU Machine

- **Use a GitHub repo for your kernels.** Push your work to a private GitHub repo so you don't lose progress if the GPU machine goes offline or loses data.
- **Use tmux for autotuning.** Autotuning can take a long time. Run it inside a `tmux` session so it survives SSH disconnections.
- **Use spawn mode for autotuning if you hit issues.** By default, Helion's autotuner uses `fork` mode for precompilation, which is faster but can hang or crash if a bad config corrupts process state. If that happens, switch to `spawn` mode, which runs each trial in an isolated subprocess with timeout protection — slower due to subprocess overhead, but one bad config can't take down your entire autotuning run. Enable it via environment variable or decorator:
  ```bash
  export HELION_AUTOTUNE_PRECOMPILE=spawn
  ```
  ```python
  @helion.kernel(autotune_precompile="spawn")
  def my_kernel(...):
      ...
  ```
  You can also control parallelism with `HELION_AUTOTUNE_PRECOMPILE_JOBS` (defaults to CPU count).
- **Machine frozen or crashed?** If your GPU machine becomes unresponsive and needs a reboot, let us know and we can reboot it for you.

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
