#!POPCORN leaderboard amd-fp8-mm
#!POPCORN gpu MI300

from task import input_t, output_t
import torch
import triton
import triton.language as tl

NUM_SMS = torch.cuda.get_device_properties("cuda").multi_processor_count


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
    TRANSPOSE: tl.constexpr = False,
):
    program_id = tl.program_id(0)
    num_pid_across_n = tl.cdiv(N, BLOCK_N)

    program_id_m = program_id // num_pid_across_n
    program_id_n = program_id % num_pid_across_n

    if not TRANSPOSE:
        A_stride_m, A_stride_k = 1, M
        B_stride_n, B_stride_k = 1, N
    else:
        A_stride_m, A_stride_k = K, 1
        B_stride_n, B_stride_k = K, 1
    C_stride_m, C_stride_n = N, 1
    # Scale matrices are stored in column-major order, with A being 1x128 and B being 128x128 chunks
    # BLOCK_Q is 128
    A_scale_stride_m, A_scale_stride_k = 1, M
    B_scale_stride_n, B_scale_stride_k = 1, tl.cdiv(N, BLOCK_Q)

    # Calculate the row and column indices in the output matrix for the current pid
    offset_m = program_id_m * BLOCK_M
    offset_n = program_id_n * BLOCK_N

    # Arange to make a row and column ptrs
    block_offsets_m = offset_m + tl.arange(0, BLOCK_M)
    block_offsets_n = offset_n + tl.arange(0, BLOCK_N)
    block_offsets_k = tl.arange(0, BLOCK_K)

    # ptrs for BLOCK_M rows of A and BLOCK_N columns of B
    A_block_ptrs = A_ptr + (
        block_offsets_m[:, None] * A_stride_m + block_offsets_k[None, :] * A_stride_k
    )
    B_block_ptrs = B_ptr + (
        block_offsets_k[:, None] * B_stride_k + block_offsets_n[None, :] * B_stride_n
    )
    # since a_scales are 1x128, a_scale_ptrs need to be of shape (BLOCK_M, 1)
    # since N, K <= BLOCK_Q, b_scale_ptrs is always a scalar ptr
    A_scale_block_ptrs = A_scale_ptr + (block_offsets_m[:, None] * A_scale_stride_m)
    B_scale_block_ptrs = B_scale_ptr + (offset_n // BLOCK_Q) * B_scale_stride_n

    # Initialize accumulator for the currrent pid (responsible for BLOCK_M * BLOCK_N elements)
    master_accumulator = tl.zeros((BLOCK_M, BLOCK_N), dtype=tl.float32)

    # In each iteration we we load BLOCK_Q elements from K dimension for BLOCK_M rows, resp. BLOCK_N columns
    # We choose this to use only 1 scale per iteration
    num_k_iters = K // BLOCK_Q
    for _ in range(0, num_k_iters):
        # Initialize accumulator for the current k iteration
        inner_accumulator = tl.zeros((BLOCK_M, BLOCK_N), dtype=tl.float32)
        # In each iteration we load BLOCK_K elements from K dimension for BLOCK_M rows, resp. BLOCK_N columns
        # We choose this to use small `tl.dot` for the inner accumulator
        for _ in tl.range(0, BLOCK_Q // BLOCK_K):
            A_block = tl.load(A_block_ptrs)  # (BLOCK_M, BLOCK_K)
            B_block = tl.load(B_block_ptrs)  # (BLOCK_K, BLOCK_N)
            inner_accumulator = tl.dot(
                A_block, B_block, inner_accumulator
            )  # (BLOCK_M, BLOCK_N)

            # Move along the K dimension of A, B
            A_block_ptrs += BLOCK_K * A_stride_k
            B_block_ptrs += BLOCK_K * B_stride_k

        A_scales = tl.load(A_scale_block_ptrs)  # (BLOCK_M, 1)
        B_scales = tl.load(B_scale_block_ptrs)  # ()
        master_accumulator += inner_accumulator * (A_scales * B_scales)

        # Move along the K dimension of A, B scales
        A_scale_block_ptrs += A_scale_stride_k
        B_scale_block_ptrs += B_scale_stride_k

    # Store the result for the current pid
    block_offsets_m = (
        program_id_m * BLOCK_M + tl.arange(0, BLOCK_M)[:, None]
    )  # (BLOCK_M, 1)
    block_offsets_n = (
        program_id_n * BLOCK_N + tl.arange(0, BLOCK_N)[None, :]
    )  # (1, BLOCK_N)
    mask = (block_offsets_m < M) & (block_offsets_n < N)  # (BLOCK_M, BLOCK_N)
    C_block_ptrs = C_ptr + (block_offsets_m * C_stride_m + block_offsets_n * C_stride_n)
    tl.store(C_block_ptrs, master_accumulator, mask=mask)


@torch.compile(dynamic=False, mode="max-autotune-no-cudagraphs")
def contiguous(x):
    return x.contiguous()


def get_config(M, N, K):
    num_blocks_ref = (M // 128) * (N // 128)
    TRANSPOSE = False
    matrix_instr_nonkdim = 16
    BLOCK_M, BLOCK_N, BLOCK_K = (128, 128, 64)
    if num_blocks_ref * 8 < NUM_SMS:  # 2 and 7
        BLOCK_M, BLOCK_N, BLOCK_K = (32, 64, 128)
        matrix_instr_nonkdim = 16
    elif num_blocks_ref < NUM_SMS:
        BLOCK_M, BLOCK_N, BLOCK_K = (64, 64, 64)
    
    config = dict(
        BLOCK_M=BLOCK_M,
        BLOCK_N=BLOCK_N,
        BLOCK_K=BLOCK_K,
        waves_per_eu=2,
        matrix_instr_nonkdim=matrix_instr_nonkdim,
        num_warps=4,
        num_stages=2,
        TRANSPOSE=TRANSPOSE,
    )
    return config


def custom_kernel(data: input_t) -> output_t:
    A_tensor, B_tensor, A_scale_tensor, B_scale_tensor, C_tensor = data

    M, K = A_tensor.shape
    N, _ = B_tensor.shape

    # heuristic
    config = get_config(M, N, K)

    num_blocks = triton.cdiv(M, config["BLOCK_M"]) * triton.cdiv(N, config["BLOCK_N"])
    kernel[(num_blocks,)](
        A_tensor, B_tensor, A_scale_tensor, B_scale_tensor, C_tensor, M, N, K, **config
    )

    return C_tensor
