"""Modal entrypoint embedded by popcorn-cli's --local mode.

The image and evaluator intentionally track KernelBot's public Modal runner so a
local run exercises the same public task definition and evaluation sequence.
"""

import dataclasses
import glob
import json
import os
import re
import time
import traceback
import urllib.request
from pathlib import Path

import modal


RESULT_MARKER = "POPCORN_LOCAL_RESULT="
REFERENCE_REPO = "gpu-mode/reference-kernels"
KERNELBOT_REPO = "gpu-mode/kernelbot"


def _github_ref(repo: str, override_env: str) -> str:
    override = os.environ.get(override_env)
    if override:
        if not re.fullmatch(r"[A-Za-z0-9._/-]+", override):
            raise ValueError(f"Invalid git ref in {override_env}")
        return override

    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "popcorn-cli",
        "Cache-Control": "no-cache",
    }
    github_token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if github_token:
        headers["Authorization"] = f"Bearer {github_token}"

    errors = []
    for attempt in range(3):
        request = urllib.request.Request(
            f"https://api.github.com/repos/{repo}/git/ref/heads/main",
            headers=headers,
        )
        try:
            with urllib.request.urlopen(request, timeout=15) as response:
                sha = json.load(response)["object"]["sha"]
            if re.fullmatch(r"[0-9a-f]{40}", sha):
                return sha
            errors.append(f"invalid SHA: {sha!r}")
        except Exception as error:
            errors.append(str(error))
        if attempt < 2:
            time.sleep(attempt + 1)

    raise RuntimeError(
        f"Could not resolve the latest {repo} main commit after 3 attempts: "
        f"{errors[-1]}. Set {override_env} to an explicit commit to run pinned instead."
    )


reference_ref = _github_ref(REFERENCE_REPO, "POPCORN_REFERENCE_KERNELS_REF")
kernelbot_ref = _github_ref(KERNELBOT_REPO, "POPCORN_KERNELBOT_REF")

cuda_version = "13.3.0"
mathdx_version = "26.06.0"
mathdx_archive = f"nvidia-mathdx-{mathdx_version}-cuda13.tar.gz"
mathdx_url = (
    "https://developer.download.nvidia.com/compute/cublasdx/redist/"
    f"cublasdx/cuda13/{mathdx_archive}"
)
mathdx_sha256 = "042b7c57a636c271cca32dffcc0a822ed6b2abc0b8ef5703ab2445d58563a1e6"
cuda_image = (
    modal.Image.from_registry(
        f"nvidia/cuda:{cuda_version}-devel-ubuntu24.04", add_python="3.13"
    )
    .entrypoint([])
    .run_commands("ln -sf $(which python) /usr/local/bin/python3")
    .apt_install("git", "curl", "gcc-13", "g++-13", "clang-18")
    .uv_pip_install(
        "ninja~=1.11",
        "wheel~=0.45",
        "requests~=2.32.4",
        "packaging~=25.0",
        "numpy~=2.3",
        "pytest",
        "PyYAML",
    )
    .uv_pip_install(
        "tinygrad~=0.10",
        "helion",
    )
    .uv_pip_install(
        "nvidia-cutlass-dsl==4.5.2",
        "cuda-core[cu13]",
        "cuda-python[all]==13.0",
        "cuda-tile==1.4.0",
        "nvmath-python[cu13-dx]==0.9.0",
        "nvidia-libmathdx-cu13==0.3.2.6",
        "cuda-toolkit[cccl,nvrtc]==13.0.2",
    )
    .uv_pip_install(
        "torch==2.12.0",
    )
    .run_commands(
        "git clone --depth 1 --branch v4.5.2 https://github.com/NVIDIA/cutlass.git /opt/cutlass",
        (
            f"curl -fsSL {mathdx_url} -o /tmp/{mathdx_archive} && "
            f"echo '{mathdx_sha256}  /tmp/{mathdx_archive}' | sha256sum -c - && "
            "mkdir -p /opt/mathdx && "
            f"tar -xzf /tmp/{mathdx_archive} --strip-components=4 -C /opt/mathdx && "
            f"rm /tmp/{mathdx_archive}"
        ),
        (
            "git clone --filter=blob:none https://github.com/"
            f"{KERNELBOT_REPO}.git /opt/kernelbot && "
            f"git -C /opt/kernelbot checkout {kernelbot_ref}"
        ),
        (
            "git clone --filter=blob:none https://github.com/"
            f"{REFERENCE_REPO}.git /opt/reference-kernels && "
            f"git -C /opt/reference-kernels checkout {reference_ref}"
        ),
    )
    .env(
        {
            "CUTLASS_PATH": "/opt/cutlass",
            "MATHDX_HOME": "/opt/mathdx",
            "CPLUS_INCLUDE_PATH": (
                "/opt/mathdx/include:/opt/mathdx/external/cutlass/include:"
                "/opt/cutlass/include:/opt/cutlass/tools/util/include"
            ),
            "PYTHONPATH": "/opt/kernelbot/src",
        }
    )
)

app = modal.App("popcorn-local-runner", image=cuda_image)
modal_gpu = os.environ["POPCORN_LOCAL_MODAL_GPU"]


def _find_problem(leaderboard: str) -> tuple[Path, list[str]]:
    import yaml

    matches = []
    for index_path in glob.glob("/opt/reference-kernels/problems/*.y*ml"):
        with open(index_path) as file:
            index = yaml.safe_load(file) or {}
        for problem in index.get("problems", []):
            directory = problem.get("directory", "")
            if problem.get("name") == leaderboard or Path(directory).name == leaderboard:
                matches.append((directory, problem.get("gpus", [])))

    unique_matches = list(dict.fromkeys(directory for directory, _ in matches))
    if not unique_matches:
        raise ValueError(f"Leaderboard '{leaderboard}' was not found in reference-kernels")
    if len(unique_matches) > 1:
        raise ValueError(
            f"Leaderboard '{leaderboard}' is ambiguous: {', '.join(unique_matches)}"
        )

    directory = unique_matches[0]
    supported_gpus = next(gpus for candidate, gpus in matches if candidate == directory)
    return Path("/opt/reference-kernels/problems") / directory / "task.yml", supported_gpus


@app.function(gpu=modal_gpu, timeout=3600)
def evaluate(submission: str, leaderboard: str, gpu: str, mode: str) -> dict:
    try:
        from libkernelbot.consts import GPU_TO_SM, SubmissionMode
        from libkernelbot.run_eval import run_config
        from libkernelbot.task import build_task_config, make_task_definition

        task_path, supported_gpus = _find_problem(leaderboard)
        if supported_gpus and gpu not in supported_gpus:
            raise ValueError(
                f"Leaderboard '{leaderboard}' does not declare GPU '{gpu}'. "
                f"Supported GPUs: {', '.join(supported_gpus)}"
            )

        definition = make_task_definition(task_path)
        config = build_task_config(
            task=definition.task,
            submission_content=submission,
            arch=GPU_TO_SM[gpu],
            mode=SubmissionMode(mode),
        )
        result = run_config(config)
        return {
            "leaderboard": leaderboard,
            "problem_directory": str(task_path.parent.relative_to("/opt/reference-kernels/problems")),
            "gpu": gpu,
            "mode": mode,
            "ranking_by": definition.task.ranking_by.value,
            "reference_kernels_ref": reference_ref,
            "kernelbot_ref": kernelbot_ref,
            "result": dataclasses.asdict(result),
        }
    except Exception as error:
        return {
            "leaderboard": leaderboard,
            "gpu": gpu,
            "mode": mode,
            "reference_kernels_ref": reference_ref,
            "kernelbot_ref": kernelbot_ref,
            "result": {
                "success": False,
                "error": "".join(traceback.format_exception(error)),
                "runs": {},
                "system": {},
            },
        }


@app.local_entrypoint()
def main():
    submission_path = Path(os.environ["POPCORN_LOCAL_SUBMISSION"])
    payload = evaluate.remote(
        submission_path.read_text(),
        os.environ["POPCORN_LOCAL_LEADERBOARD"],
        os.environ["POPCORN_LOCAL_GPU"],
        os.environ["POPCORN_LOCAL_MODE"],
    )
    print(RESULT_MARKER + json.dumps(payload, default=str, separators=(",", ":")))
