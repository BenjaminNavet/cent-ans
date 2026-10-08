"""Free local image generation with mflux (Z-Image Turbo on Apple Silicon, ADR 0190).

Any pipeline that calls :func:`cent_ans_tools.openrouter.request_image` with the model
:data:`MODEL_ID` gets its image from :func:`render_image` (no cost, no budget entry).
The model is the 8-bit copy saved by ``mflux-save --model z-image-turbo --quantize 8``
(path from ``CENT_ANS_MFLUX_MODEL``). A reference image is the image-to-image start.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
import zlib
from collections.abc import Callable
from pathlib import Path

from cent_ans_tools.portraits import PortraitJob

MODEL_ID = "local/z-image-turbo"
MFLUX_COMMAND = "mflux-generate-z-image-turbo"
DEFAULT_MODEL_PATH = Path.home() / "models" / "mflux" / "z-image-turbo-q8"
STEPS = 9
WIDTH = 1024
HEIGHT = 576
REFERENCE_STRENGTH = 0.4
# Output sizes (multiples of 16, about one megapixel) per OpenRouter ``aspect_ratio``.
ASPECT_SIZES = {
    "1:1": (1024, 1024),
    "16:9": (1024, 576),
    "9:16": (576, 1024),
    "4:3": (1024, 768),
    "3:4": (768, 1024),
    "3:2": (1152, 768),
    "2:3": (768, 1152),
    "21:9": (1344, 576),
    "4:1": (1536, 384),
}

Runner = Callable[[list[str]], None]


def model_path() -> Path:
    """Local quantized model, overridable with ``CENT_ANS_MFLUX_MODEL``."""
    return Path(os.environ.get("CENT_ANS_MFLUX_MODEL", DEFAULT_MODEL_PATH))


def seed_for(job: PortraitJob) -> int:
    """Stable seed per entry id, so a rerun gives the same image."""
    return zlib.crc32(job.character_id.encode("utf-8"))


def size_for(aspect_ratio: str | None) -> tuple[int, int]:
    """Output size for an OpenRouter-style ``aspect_ratio`` (default 16:9)."""
    if aspect_ratio is None:
        return WIDTH, HEIGHT
    if aspect_ratio not in ASPECT_SIZES:
        raise ValueError(f"Format non géré en local : {aspect_ratio}")
    return ASPECT_SIZES[aspect_ratio]


def command_for(
    job: PortraitJob,
    prompt_file: Path,
    output: Path,
    size: tuple[int, int] = (WIDTH, HEIGHT),
    seed: int | None = None,
    strength: float | None = None,
) -> list[str]:
    """The mflux command line for one job (seed from its id unless given).

    ``strength`` is the img2img strength when the job has a reference (default
    :data:`REFERENCE_STRENGTH`).
    """
    command = [
        MFLUX_COMMAND,
        "--model", str(model_path()),
        "--base-model", "z-image-turbo",
        "--prompt-file", str(prompt_file),
        "--steps", str(STEPS),
        "--seed", str(seed_for(job) if seed is None else seed),
        "--width", str(size[0]),
        "--height", str(size[1]),
        "--output", str(output),
    ]  # fmt: skip
    if job.reference is not None:
        command += [
            "--image-path", str(job.reference),
            "--image-strength",
            str(REFERENCE_STRENGTH if strength is None else strength),
        ]  # fmt: skip
    return command


def _run(command: list[str]) -> None:
    subprocess.run(command, check=True, capture_output=True)


def render_image(
    prompt: str,
    *,
    aspect_ratio: str | None = None,
    reference: bytes | None = None,
    seed: int | None = None,
    strength: float | None = None,
    runner: Runner | None = None,
) -> bytes:
    """One PNG image for ``prompt``; ``reference`` (PNG/JPEG) is the img2img start.

    Without ``seed`` the seed is derived from the prompt, so a rerun is identical.
    ``strength`` overrides :data:`REFERENCE_STRENGTH` (img2img, with ``reference``; high = the
    result stays close to the reference, ~0.15 keeps only its line style). Environment
    ``CENT_ANS_LOCAL_STRENGTH`` sets it for a whole batch (DN ui-prod icons: 0.15) and
    ``CENT_ANS_LOCAL_SEED_SALT`` changes the prompt-derived seed to redo a failed image.
    """
    if strength is None and os.environ.get("CENT_ANS_LOCAL_STRENGTH"):
        strength = float(os.environ["CENT_ANS_LOCAL_STRENGTH"])
    if seed is None and os.environ.get("CENT_ANS_LOCAL_SEED_SALT"):
        seed = zlib.crc32((prompt + os.environ["CENT_ANS_LOCAL_SEED_SALT"]).encode("utf-8"))
    with tempfile.TemporaryDirectory(prefix="cent_ans_mflux_") as work_dir:
        work = Path(work_dir)
        reference_path = None
        if reference is not None:
            suffix = ".jpg" if reference[:2] == b"\xff\xd8" else ".png"
            reference_path = work / f"reference{suffix}"
            reference_path.write_bytes(reference)
        job = PortraitJob(
            "image", prompt, work / "unused.jpg", reference=reference_path
        )
        prompt_file = work / "prompt.txt"
        prompt_file.write_text(prompt, encoding="utf-8")
        output = work / "image.png"
        (runner or _run)(
            command_for(
                job,
                prompt_file,
                output,
                size_for(aspect_ratio),
                zlib.crc32(prompt.encode("utf-8")) if seed is None else seed,
                strength,
            )
        )
        return output.read_bytes()
