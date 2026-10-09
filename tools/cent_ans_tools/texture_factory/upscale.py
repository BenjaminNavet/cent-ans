"""Upscaling of raw generated textures to the output size (T1d).

Methods: ``esrgan`` (Real-ESRGAN x4plus, then Lanczos down to the output size, ~2 min per
image on a M4 Pro), ``esrgan_light`` (realesr-animevideov3 x2, light model), ``lanczos``
(plain resampling). The catalogue may pick one per family with its ``upscale`` field;
``native`` means the raw image already has the wanted size.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
from collections.abc import Callable
from pathlib import Path

from PIL import Image

METHODS = ("esrgan", "esrgan_light", "lanczos")
DEFAULT_BINARY = Path("~/models/realesrgan/realesrgan-ncnn-vulkan")
BINARY_VARIABLE = "CENT_ANS_REALESRGAN"

Runner = Callable[[list[str]], None]


class UpscaleError(RuntimeError):
    """Unknown method or missing Real-ESRGAN binary."""


def binary_path() -> Path:
    """The Real-ESRGAN ncnn binary (``CENT_ANS_REALESRGAN`` or the default location)."""
    return Path(os.environ.get(BINARY_VARIABLE) or DEFAULT_BINARY).expanduser()


def _run(command: list[str]) -> None:
    subprocess.run(command, check=True, capture_output=True)


def esrgan_command(binary: Path, method: str, src: Path, dst: Path) -> list[str]:
    """Command line of one Real-ESRGAN pass (x4plus x4, or animevideov3 x2)."""
    models = str(binary.parent / "models")
    if method == "esrgan":
        model, scale = "realesrgan-x4plus", "4"
    else:
        model, scale = "realesr-animevideov3", "2"
    return [
        str(binary), "-i", str(src), "-o", str(dst),
        "-n", model, "-s", scale, "-m", models, "-f", "png",
    ]  # fmt: skip


def upscale(
    src: Path,
    dst: Path,
    method: str,
    out_size: int = 2048,
    runner: Runner | None = None,
) -> Path:
    """Write ``src`` upscaled to ``out_size`` x ``out_size`` at ``dst`` (PNG)."""
    if method not in METHODS:
        raise UpscaleError(
            f"méthode inconnue : {method} (attendu : {', '.join(METHODS)})"
        )
    src, dst = Path(src), Path(dst)
    dst.parent.mkdir(parents=True, exist_ok=True)
    if method == "lanczos":
        large = Image.open(src).convert("RGB")
    else:
        binary = binary_path()
        if runner is None and not (binary.is_file() and os.access(binary, os.X_OK)):
            raise UpscaleError(
                f"Real-ESRGAN introuvable : {binary} "
                f"(définir {BINARY_VARIABLE}, voir docs/pipeline-assets-3d.md)"
            )
        with tempfile.TemporaryDirectory(prefix="cent_ans_esrgan_") as work_dir:
            raw = Path(work_dir) / "esrgan.png"
            (runner or _run)(esrgan_command(binary, method, src, raw))
            large = Image.open(raw).convert("RGB")
            large.load()
    if large.size != (out_size, out_size):
        large = large.resize((out_size, out_size), Image.Resampling.LANCZOS)
    large.save(dst, format="PNG")
    return dst


def method_for(document: dict) -> str:
    """Upscale method of a catalogue document (default ``lanczos``; ``native`` too)."""
    return document.get("upscale", "lanczos")


def upscale_family(
    document: dict,
    only: list[str] | None = None,
    method: str | None = None,
    raw_dir: Path | None = None,
    runner: Runner | None = None,
) -> list[Path]:
    """Upscale the latest raw image of each selected entry into ``<raw>/upscaled/<id>.png``.

    Entries without an ``ok`` record in the manifest are skipped and existing outputs are
    kept. With ``native`` nothing is done: the raw image already has the output size.
    """
    from cent_ans_tools.texture_factory.catalog import select
    from cent_ans_tools.texture_factory.generate import image_path, load_manifest

    chosen = method or method_for(document)
    if chosen == "native":
        return []
    manifest = load_manifest(document, raw_dir)
    written: list[Path] = []
    for entry in select(document, only):
        record = manifest.get(entry["id"])
        if not record or record.get("status") != "ok":
            continue
        src = image_path(document, entry["id"], record["attempt"], raw_dir)
        dst = src.parent / "upscaled" / f"{entry['id']}.png"
        if not dst.is_file():
            upscale(src, dst, chosen, document["output_size"], runner)
        written.append(dst)
    return written
