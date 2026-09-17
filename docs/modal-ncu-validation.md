# Modal NCU validation

Validated on September 17, 2026, with no special Modal privileges or runtime flags.

- Direct B200 matmul smoke: NCU 2025.2.1 captured one kernel in 39 passes and saved a report.
- CLI end-to-end: QR v2 benchmark 0 (`batch=20, n=32, cond=1, seed=43214`) on NVIDIA B200, using NCU 2026.2.0 and PyTorch 2.12.0+cu130.
- Resolved problem: `problems/linalg/qr_v2`; its evaluator supports profile mode with the `custom_kernel` NVTX range.
- `reference-kernels`: `51e22db671d36c1c76091c43c36a44546ba324a1`.
- `kernelbot`: `30ba5ce79107e5405b0cc1eda48ca551e7a51b16`.

```bash
popcorn submit submission.py --profile --leaderboard qr_v2 \
  --benchmark-index 0 --ncu-kernel-name 'regex:geqr2' \
  --ncu-kernel-name-base demangled --ncu-launch-count 1
```

The successful QR profile run produced an approximately 7.9 MiB `profile.ncu-rep`, `ncu-details.txt`,
`ncu-details.csv`, and a provenance manifest. The details export reopened the
report and identified `geqr2_batch_kernel_shmem` with measured GPU counters.
The matmul smoke reported six unavailable `ctc__*` metrics; successful capture
does not imply that every metric in `--set full` is populated.

The installed release binary also passed with `--mode profile`, no explicit
`--gpu`, no kernel-name filter, and `--output summary.txt`.
It selected B200, saved all three report formats, and wrote the summary file.

The change is contained in popcorn-cli; it uses KernelBot's task builder and
per-shape evaluator. The CLI overrides the NCU capture function in its ephemeral
worker to configure filters, child-process tracing, unlocked clocks, and detail
exports. It does not modify or deploy the hosted KernelBot service.

`--profile` and `--mode profile` select Modal. `--profile-brev` explicitly selects
Brev. A failed Modal profile returns an error and never switches providers.
Omitting `--benchmark-index` preserves the task's entire `benchmarks` list;
selection and invalid indices are covered by CPU tests.

Validation commands:

```bash
cargo test                         # 58 tests passed
cargo clippy --all-targets -- -D warnings
python3 -m unittest discover -s tests -v   # 3 tests passed
cargo build --release
```
