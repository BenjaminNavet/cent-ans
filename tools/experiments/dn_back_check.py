"""DN-TROUS: render generated models from the front and from behind and compare luminance.

``uv run --with pillow --with numpy python tools/experiments/dn_back_check.py --out OUT.json
ID=path/to/model.glb ...`` (headless Blender through ``ga3_compare_render.py``). A model whose darker
side renders darker than 0.6 x the brighter one is flagged ``dark_back`` (kept anyway: a town with a
black part beats a holed or missing one).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image

REPO = Path(__file__).resolve().parents[2]


def mean_luma(panel: np.ndarray, background: np.ndarray) -> float:
    """Mean luminance of the pixels that differ from the background colour."""
    mask = np.abs(panel.astype(int) - background.astype(int)).max(axis=2) > 14
    if mask.sum() < 50:
        return 0.0
    return float((panel[..., :3] @ np.array([0.299, 0.587, 0.114]))[mask].mean())


def check(model: str, glb: Path, keep: Path | None) -> dict:
    """Front/back luminance of one glb."""
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "r.png"
        command = [
            "blender", "-b", "--factory-startup", "-P",
            str(REPO / "tools/blender_scripts/ga3_compare_render.py"), "--",
            "--out", str(out), "--fit-width", "1.8",
            "--panel", str(glb), f"{model} front", "--yaw", "0",
            "--panel", str(glb), f"{model} back", "--yaw", "180",
        ]  # fmt: skip
        subprocess.run(command, capture_output=True, check=False)
        image = np.asarray(Image.open(out).convert("RGB"))
        if keep:
            Image.open(out).save(keep)
    half = image.shape[1] // 2
    rows = image.shape[0] // 2
    top = image[40:rows]
    background = image[rows - 5, 2]
    front = mean_luma(top[:, :half], background)
    back = mean_luma(top[:, half:], background)
    ratio = min(front, back) / max(front, back, 1e-6)  # darker side / brighter side
    return {"id": model, "front": round(front, 1), "back": round(back, 1),
            "ratio": round(ratio, 2), "dark_back": ratio < 0.6}  # fmt: skip


def main() -> None:
    """CLI."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--keep-dir", default="")
    parser.add_argument("models", nargs="+", help="ID=glb")
    args = parser.parse_args()
    results = []
    for item in args.models:
        model, path = item.split("=", 1)
        keep = Path(args.keep_dir) / f"{model}.png" if args.keep_dir else None
        results.append(check(model, Path(path), keep))
        print(results[-1], flush=True)
    Path(args.out).write_text(json.dumps(results, indent=1))


if __name__ == "__main__":
    main()
