# Helion Kernel Challenge

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

## Tips

- **Iterate locally first.** Use your Nebius B200 to develop and autotune. Only submit to KernelBot once you have a hardcoded config that works.
- **Check the reference.** Each `reference.py` shows the baseline implementation you're trying to beat. Understanding it helps you write a better kernel.
- **Use `--mode test` first.** Verify correctness before submitting to the leaderboard. This saves time and leaderboard quota.
- **Profile your kernels.** Use `--mode profile` to get Nsight Compute metrics and identify bottlenecks.
- **One config per submission.** If Helion found different best configs for different benchmark shapes, pick the one that works best across all of them -- the leaderboard uses geometric mean across benchmarks.
## Resources

- [Helion Documentation](https://helionlang.com)
- [Helion GitHub](https://github.com/pytorch/helion)
- [Reference Kernels](https://github.com/gpu-mode/reference-kernels/tree/main/problems/helion)
- [GPU MODE Discord](https://discord.gg/gpumode)
