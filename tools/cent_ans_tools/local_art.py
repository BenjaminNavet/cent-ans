"""Free local image generation with mflux (Z-Image Turbo on Apple Silicon, ADR 0190).

Same jobs as the paid OpenRouter batches (:class:`PortraitJob`), generated on this
machine by the ``mflux-generate-z-image-turbo`` command: no cost, no budget entry.
The model is the 8-bit copy saved by ``mflux-save --model z-image-turbo --quantize 8``
(path from ``CENT_ANS_MFLUX_MODEL``). Jobs with a reference image use it as the
image-to-image start.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
import zlib
from collections.abc import Callable
from pathlib import Path

from cent_ans_tools.portraits import PortraitJob

MFLUX_COMMAND = "mflux-generate-z-image-turbo"
DEFAULT_MODEL_PATH = Path.home() / "models" / "mflux" / "z-image-turbo-q8"
STEPS = 9
WIDTH = 1024
HEIGHT = 576
REFERENCE_STRENGTH = 0.4

Runner = Callable[[list[str]], None]


def model_path() -> Path:
    """Local quantized model, overridable with ``CENT_ANS_MFLUX_MODEL``."""
    return Path(os.environ.get("CENT_ANS_MFLUX_MODEL", DEFAULT_MODEL_PATH))


def seed_for(job: PortraitJob) -> int:
    """Stable seed per entry id, so a rerun gives the same image."""
    return zlib.crc32(job.character_id.encode("utf-8"))


def command_for(job: PortraitJob, prompt_file: Path, output: Path) -> list[str]:
    """The mflux command line for one job."""
    command = [
        MFLUX_COMMAND,
        "--model", str(model_path()),
        "--base-model", "z-image-turbo",
        "--prompt-file", str(prompt_file),
        "--steps", str(STEPS),
        "--seed", str(seed_for(job)),
        "--width", str(WIDTH),
        "--height", str(HEIGHT),
        "--output", str(output),
    ]  # fmt: skip
    if job.reference is not None:
        command += [
            "--image-path", str(job.reference),
            "--image-strength", str(REFERENCE_STRENGTH),
        ]  # fmt: skip
    return command


def _run(command: list[str]) -> None:
    subprocess.run(command, check=True, capture_output=True)


def generate(
    jobs: list[PortraitJob],
    convert: Callable[[bytes], bytes],
    *,
    runner: Runner = _run,
    on_progress: Callable[[PortraitJob], None] | None = None,
) -> tuple[list[Path], list[tuple[PortraitJob, str]]]:
    """Generate each job locally; returns (written files, failed jobs with reason)."""
    written: list[Path] = []
    failed: list[tuple[PortraitJob, str]] = []
    with tempfile.TemporaryDirectory(prefix="cent_ans_mflux_") as work_dir:
        for job in jobs:
            prompt_file = Path(work_dir) / f"{job.character_id}.txt"
            raw_image = Path(work_dir) / f"{job.character_id}.png"
            prompt_file.write_text(job.prompt, encoding="utf-8")
            try:
                runner(command_for(job, prompt_file, raw_image))
                job.out_path.parent.mkdir(parents=True, exist_ok=True)
                job.out_path.write_bytes(convert(raw_image.read_bytes()))
            except (subprocess.CalledProcessError, OSError) as error:
                failed.append((job, str(error)))
                continue
            written.append(job.out_path)
            if on_progress is not None:
                on_progress(job)
    return written, failed
