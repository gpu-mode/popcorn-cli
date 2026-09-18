# Hosted NCU profiling validation

`--profile` uses Popcorn authentication and the KernelBot API. KernelBot owns
Modal credentials and executes NCU; the client only uploads code and saves
returned artifacts. `--profile-brev` remains explicit. A capture error never
switches providers.

The CLI calls `POST /profile/{leaderboard}/{gpu}` with multipart capture options.
This new endpoint requires the companion KernelBot update. Deploy the API and
GPU runner before releasing the CLI; older services do not support this route.
The existing `--local` evaluation workflow is independent of hosted profiling.

Validation covers:

- Rust tests for authenticated HTTP submission, multipart options, SSE errors,
  artifact extraction, and malformed/missing captures.
- KernelBot tests for authenticated API requests, option validation, benchmark
  selection, child-process capture, and report exports.
- An isolated integration run exercised the real FastAPI route, submission
  preparation, KernelBackend, GPU `run_config`, and CLI artifact extraction,
  with a fake database and an ephemeral B200 launcher. The CLI used normal
  Popcorn header authentication with an empty `PATH` and no `MODAL_*` values.
  It returned a 7,593,257-byte report plus text/CSV exports.
- NCU 2025.2.1 captured 39 passes. The exported report contained 322.78 us
  duration, 12.51% achieved occupancy, 0.42% SM throughput, 70.09% L2 hit rate,
  and 1,565,428 executed instructions. Six `ctc__*` metrics were unavailable.
- NCU 2026.2.0 from CUDA 13.3 produced many NaN counters on this Modal B200.
  The server image therefore pins the tested 2025.2.1 version. The earlier
  2026.2 report was evidence of artifact delivery, not healthy counter coverage.

GPU fixture provenance: `problems/linalg/qr_v2`, benchmark 0
(`batch=20, n=32, cond=1, seed=43214`), reference-kernels
`51e22db671d36c1c76091c43c36a44546ba324a1` (the subsequent guide commit changes
only documentation). The initial KernelBot baseline was
`30ba5ce79107e5405b0cc1eda48ca551e7a51b16`.

Only QR v2 has been tested end to end for this change. Other problems need an
NCU-compatible evaluator; accepting `profile` with only a PyTorch profiler path
is insufficient. Single-GPU NVIDIA capture is supported. Some requested metrics
can be unavailable on a particular GPU.

The tested image used CUDA 13.3 and PyTorch 2.12.0+cu130, with
`regex:geqr2`, demangled kernel names, and launch count 1. The isolated test
is not a production deployment. [Operator validation run](https://modal.com/apps/coreauto/main/ap-p8eodYfHkZAMp0ZT5btR1y)
requires access to the operator's workspace.
