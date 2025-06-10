#!POPCORN leaderboard amd-fp8-mm
#!POPCORN gpu MI300

from task import input_t, output_t
import torch
import triton
import triton.language as tl


@triton.jit
def kernel(
    A_ptr,
    B_ptr,
    A_scale_ptr,
    B_scale_ptr,
    C_ptr,
    M: tl.constexpr,
    N: tl.constexpr,
    K: tl.constexpr,
    BLOCK_M: tl.constexpr,
    BLOCK_N: tl.constexpr,
    BLOCK_K: tl.constexpr,
    BLOCK_Q: tl.constexpr = 128,
):
    program_id = tl.program_id(0)
    num_pid_across_n = tl.cdiv(N, BLOCK_N)

    program_id_m = program_id // num_pid_across_n
    program_id_n = program_id % num_pid_across_n

    # Simple stride assumptions (no transpose)
    A_stride_m, A_stride_k = 1, M
    B_stride_n, B_stride_k = 1, N
    C_stride_m, C_stride_n = N, 1
    
    # Scale matrices: A is 1x128, B is 128x128 chunks
    A_scale_stride_m, A_scale_stride_k = 1, M
    B_scale_stride_n, B_scale_stride_k = 1, tl.cdiv(N, BLOCK_Q)

    # Calculate output block position
    offset_m = program_id_m * BLOCK_M
    offset_n = program_id_n * BLOCK_N

    # Create block offset arrays
    block_offsets_m = offset_m + tl.arange(0, BLOCK_M)
    block_offsets_n = offset_n + tl.arange(0, BLOCK_N)
    block_offsets_k = tl.arange(0, BLOCK_K)

    # Create pointers for A and B blocks
    A_block_ptrs = A_ptr + (
        block_offsets_m[:, None] * A_stride_m + block_offsets_k[None, :] * A_stride_k
    )
    B_block_ptrs = B_ptr + (
        block_offsets_k[:, None] * B_stride_k + block_offsets_n[None, :] * B_stride_n
    )
    
    # Scale pointers
    A_scale_block_ptrs = A_scale_ptr + (block_offsets_m[:, None] * A_scale_stride_m)
    B_scale_block_ptrs = B_scale_ptr + (offset_n // BLOCK_Q) * B_scale_stride_n

    # Main accumulator
    master_accumulator = tl.zeros((BLOCK_M, BLOCK_N), dtype=tl.float32)

    # Process K dimension in BLOCK_Q chunks (128 elements at a time)
    num_k_iters = K // BLOCK_Q
    for _ in range(0, num_k_iters):
        # Inner accumulator for current 128-element K chunk
        inner_accumulator = tl.zeros((BLOCK_M, BLOCK_N), dtype=tl.float32)
        
        # Process the 128-element chunk in smaller BLOCK_K pieces
        for _ in tl.range(0, BLOCK_Q // BLOCK_K):
            A_block = tl.load(A_block_ptrs)  # (BLOCK_M, BLOCK_K)
            B_block = tl.load(B_block_ptrs)  # (BLOCK_K, BLOCK_N)
            inner_accumulator = tl.dot(A_block, B_block, inner_accumulator)

            # Move to next K chunk
            A_block_ptrs += BLOCK_K * A_stride_k
            B_block_ptrs += BLOCK_K * B_stride_k

        # Load scales and apply to inner result
        A_scales = tl.load(A_scale_block_ptrs)  # (BLOCK_M, 1)
        B_scales = tl.load(B_scale_block_ptrs)  # scalar
        master_accumulator += inner_accumulator * (A_scales * B_scales)

        # Move to next scale block
        A_scale_block_ptrs += A_scale_stride_k
        B_scale_block_ptrs += B_scale_stride_k

    # Store final result
    block_offsets_m = (program_id_m * BLOCK_M + tl.arange(0, BLOCK_M)[:, None])
    block_offsets_n = (program_id_n * BLOCK_N + tl.arange(0, BLOCK_N)[None, :])
    mask = (block_offsets_m < M) & (block_offsets_n < N)
    C_block_ptrs = C_ptr + (block_offsets_m * C_stride_m + block_offsets_n * C_stride_n)
    tl.store(C_block_ptrs, master_accumulator, mask=mask)


def custom_kernel(data: input_t) -> output_t:
    A_tensor, B_tensor, A_scale_tensor, B_scale_tensor, C_tensor = data

    M, K = A_tensor.shape
    N, _ = B_tensor.shape

    # Fixed, simple configuration - no dynamic tuning
    BLOCK_M = 64
    BLOCK_N = 64  
    BLOCK_K = 32
    
    # Launch grid
    num_blocks = triton.cdiv(M, BLOCK_M) * triton.cdiv(N, BLOCK_N)
    
    kernel[(num_blocks,)](
        A_tensor, 
        B_tensor, 
        A_scale_tensor, 
        B_scale_tensor, 
        C_tensor,
        M, N, K,
        BLOCK_M=BLOCK_M,
        BLOCK_N=BLOCK_N, 
        BLOCK_K=BLOCK_K,
        num_warps=4,
        num_stages=2,
    )

    return C_tensor