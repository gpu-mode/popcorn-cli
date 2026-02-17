---
name: load-inline-native-code
description: Helps write CUDA and HIP kernels using torch.utils.cpp_extension.load_inline(). Use when users want to write native GPU code (CUDA/HIP) inside a Python submission file.
compatibility: Intended for popcorn-cli submissions targeting NVIDIA or AMD GPUs with native kernel code.
---

# Writing Native GPU Kernels with load_inline()

Use this skill when the user wants to write a custom CUDA or HIP kernel inside their Python submission file using `torch.utils.cpp_extension.load_inline()`.

## Overview

`load_inline()` compiles C++/CUDA/HIP source code at runtime and loads it as a Python module. This lets you write raw GPU kernels directly in your `submission.py` without a separate build system.

## CUDA Template (NVIDIA GPUs)

```python
import torch
from torch.utils.cpp_extension import load_inline
from task import input_t, output_t

CUDA_SRC = """
template <typename scalar_t>
__global__ void my_kernel(const scalar_t* __restrict__ input,
                          scalar_t* __restrict__ output,
                          int N) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < N) {
        output[idx] = input[idx];
    }
}

torch::Tensor my_op(torch::Tensor input, torch::Tensor output) {
    int N = input.numel();
    const int threads = 256;
    const int blocks = (N + threads - 1) / threads;

    AT_DISPATCH_FLOATING_TYPES_AND_HALF(input.scalar_type(), "my_kernel", ([&] {
        my_kernel<scalar_t><<<blocks, threads>>>(
            input.data_ptr<scalar_t>(),
            output.data_ptr<scalar_t>(),
            N
        );
    }));

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) {
        throw std::runtime_error(cudaGetErrorString(err));
    }
    return output;
}
"""

CPP_SRC = """
torch::Tensor my_op(torch::Tensor input, torch::Tensor output);
"""

module = load_inline(
    name='my_module',
    cpp_sources=[CPP_SRC],
    cuda_sources=[CUDA_SRC],
    functions=['my_op'],
    verbose=True,
)

def custom_kernel(data: input_t) -> output_t:
    input, output = data
    return module.my_op(input, output)
```

## HIP Template (AMD GPUs)

```python
import os
os.environ['PYTORCH_ROCM_ARCH'] = 'gfx942'
os.environ['CXX'] = 'clang++'

import torch
from torch.utils.cpp_extension import load_inline
from task import input_t, output_t

CUDA_SRC = """
#include <hip/amd_detail/amd_hip_bf16.h>

__global__ void my_kernel(const float* input, float* output, int N) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < N) {
        output[idx] = input[idx];
    }
}

void my_op(torch::Tensor input, torch::Tensor output) {
    int N = input.numel();
    const int threads = 256;
    const int blocks = (N + threads - 1) / threads;
    my_kernel<<<blocks, threads>>>(
        input.data_ptr<float>(),
        output.data_ptr<float>(),
        N
    );
}
"""

CPP_SRC = """
void my_op(torch::Tensor input, torch::Tensor output);
"""

module = load_inline(
    name='my_module',
    cpp_sources=[CPP_SRC],
    cuda_sources=[CUDA_SRC],
    functions=['my_op'],
    verbose=True,
    extra_cuda_cflags=["--offload-arch=gfx942", "-std=c++20"],
)

def custom_kernel(data: input_t) -> output_t:
    input, output = data
    module.my_op(input, output)
    return output
```

## Key Points

- **cpp_sources**: C++ header declaring the functions you want to call from Python. These are the bindings.
- **cuda_sources**: The actual CUDA/HIP kernel code and the C++ wrapper that launches it.
- **functions**: List of function names to expose to Python. Must match the C++ function signatures exactly.
- **verbose=True**: Prints compilation output so you can debug build errors.
- **extra_cuda_cflags**: Pass extra compiler flags. Needed for AMD HIP (`--offload-arch=gfx942`) or C++ standard selection.

## Common Patterns

- Use `AT_DISPATCH_FLOATING_TYPES_AND_HALF` to handle multiple dtypes in CUDA kernels.
- For AMD/HIP, set `PYTORCH_ROCM_ARCH` and `CXX` env vars **before** importing torch.
- Always check `cudaGetLastError()` after kernel launches for NVIDIA targets.
- The `load_inline` call compiles on first run and caches the result. Subsequent runs reuse the cache unless the source changes.
- Keep the module-level `load_inline()` call **outside** `custom_kernel()` so compilation happens once at import time, not on every call.

## Guardrails
- The `custom_kernel` function signature must match `def custom_kernel(data: input_t) -> output_t:`.
- The module is compiled at import time. Do not call `load_inline()` inside `custom_kernel()`.
- For AMD GPUs, always set `PYTORCH_ROCM_ARCH` before any torch import.
- Use `torch::Tensor` in C++ signatures for seamless Python-C++ tensor passing.
