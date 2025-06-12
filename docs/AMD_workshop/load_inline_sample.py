# Credit https://github.com/Snektron/gpumode-amd-fp8-mm/blob/main/fp8_gemm.py
#!POPCORN leaderboard amd-fp8-mm
#!POPCORN gpu MI300
from task import input_t, output_t
import torch
from torch.utils.cpp_extension import load_inline
import time
import os
import sys

if "PYTORCH_ROCM_ARCH" not in os.environ:
    os.environ["PYTORCH_ROCM_ARCH"] = "gfx942:xnack-"

TESTING = os.environ.get("GPUMODE_TESTING", None)

with open('solution.hip', 'r') as f:
    kernel_cpp = f.read()

hip_module = load_inline(
    name="fp8",
    cpp_sources="",
    cuda_sources=kernel_cpp,
    with_cuda=True,
    verbose=False,
    extra_cuda_cflags=(["-save-temps"] if TESTING is not None else []) + ["-std=c++20", "-Werror"],
    build_directory="/workspace/build/" if TESTING == "vscode" else "/gpumode/amd/fp8/build/" if TESTING is not None else None,
    **({'no_implicit_headers': True} if TESTING != "vscode" else {}),
