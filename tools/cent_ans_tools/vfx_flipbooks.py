"""Fire and smoke flipbooks baked from free simulated sequences (lot FA2).

Provides the fire/smoke flipbooks from Unity Labs Paris sequences ("Free VFX
image sequences & flipbooks", CC0), keeping the encoding the shaders expect (8x8 frames):

- `flame_flipbook.png`: a looping flame. RGB = heat (0 cold edge, 1 white core), A = coverage.
  The source frames are twice as tall as wide: each sits centred in a square frame.
- `smoke_flipbook.png`: a smoke puff over its life (frame 0 = young and dense, 63 = spread and
  thin). R = density, G = self-shadowed lighting, A = density. The source is a looping billowing
  cloud: the puff's growth and thinning with age are applied here.

Sources and settings live in `data/fx/fire_flipbooks.json`. Raw sequences are downloaded once into
the raw directory (outside the repository).

Usage: uv run --project tools python -m cent_ans_tools.vfx_flipbooks [raw_dir] [output_dir]
"""

from __future__ import annotations

import io
import json
import sys
import urllib.request
import zipfile
from pathlib import Path

import numpy as np
from PIL import Image
from cent_ans_tools.paths import REPO_DIR as ROOT

GRID = 8
FRAMES = GRID * GRID
SETTINGS = ROOT / "data" / "fx" / "fire_flipbooks.json"
DEFAULT_OUT = ROOT / "game" / "assets" / "textures" / "fx"
DEFAULT_RAW = Path.home() / "dev" / "cent-ans-raw" / "fa" / "vfx" / "unity"
LUMA = np.array([0.3, 0.59, 0.11], dtype=np.float32)


def source_frames(raw_dir: Path, source: dict, source_url: str) -> list[np.ndarray]:
    """Frames of a source sequence as float RGBA in [0, 1] (downloaded if missing)."""
    name = source["name"]
    folder = raw_dir / name
    sheets = sorted(folder.glob("*.tga"))
    if not sheets:
        folder.mkdir(parents=True, exist_ok=True)
        request = urllib.request.Request(
            source_url.format(name=name), headers={"User-Agent": "cent-ans-game/1.0"}
        )
        with urllib.request.urlopen(request, timeout=300) as response:
            zipfile.ZipFile(io.BytesIO(response.read())).extractall(folder)
        sheets = sorted(folder.rglob("*.tga"))
    sheet = np.asarray(Image.open(sheets[0]).convert("RGBA"), dtype=np.float32) / 255.0
    columns, rows = int(source["columns"]), int(source["rows"])
    height, width = sheet.shape[0] // rows, sheet.shape[1] // columns
    frames = [
        sheet[row * height : (row + 1) * height, column * width : (column + 1) * width]
        for row in range(rows)
        for column in range(columns)
    ]
    if len(frames) != FRAMES:
        raise ValueError(f"{name}: {len(frames)} frames, expected {FRAMES}")
    return frames


def _resized(frame: np.ndarray, width: int, height: int) -> np.ndarray:
    """Resamples a float RGBA frame (colour premultiplied so edges do not darken)."""
    premultiplied = frame.astype(np.float32)
    premultiplied[..., :3] *= frame[..., 3:4]
    channels = [
        np.asarray(
            Image.fromarray(
                np.ascontiguousarray(premultiplied[..., index]), "F"
            ).resize((width, height), Image.Resampling.LANCZOS)
        )
        for index in range(4)
    ]
    out = np.clip(np.stack(channels, axis=-1), 0.0, 1.0)
    alpha = np.maximum(out[..., 3:4], 1e-4)
    out[..., :3] = np.clip(out[..., :3] / alpha, 0.0, 1.0)
    return out


def flame_sheet_frames(frames: list[np.ndarray], settings: dict) -> list[np.ndarray]:
    """Square (heat, heat, heat, coverage) frames from the tall flame frames."""
    size = int(settings["frame_px"])
    luminance = [frame[..., :3] @ LUMA for frame in frames]
    lit = np.concatenate(
        [lum[frame[..., 3] > 0.1] for lum, frame in zip(luminance, frames, strict=True)]
    )
    white = float(np.percentile(lit, float(settings["white_percentile"])))
    fade = int(settings["base_fade_px"])
    out = []
    for lum, frame in zip(luminance, frames, strict=True):
        heat = np.clip(lum / white, 0.0, 1.0) ** float(settings["heat_gamma"])
        heat = heat * float(settings.get("heat_max", 1.0))
        alpha = frame[..., 3] ** float(settings.get("coverage_gamma", 1.0))
        if fade > 0:
            alpha[-fade:] *= np.linspace(1.0, 0.0, fade, dtype=np.float32)[:, None]
        tall = np.stack([heat, heat, heat, alpha], axis=-1)
        width = round(
            size * frame.shape[1] / frame.shape[0] * float(settings["width_scale"])
        )
        tall = _resized(tall, width, size)
        square = np.zeros((size, size, 4), dtype=np.float32)
        left = (size - width) // 2
        square[:, left : left + width] = tall
        out.append(square)
    return out


def _edge_mask(size: int, edge_fade: float) -> np.ndarray:
    """Radial mask of a square frame: 1 inside, eased to 0 over the outer `edge_fade` of the radius."""
    if edge_fade <= 0.0:
        return np.ones((size, size), dtype=np.float32)
    axis = (np.arange(size, dtype=np.float32) + 0.5) / size * 2.0 - 1.0
    radius = np.sqrt(axis[None, :] ** 2 + axis[:, None] ** 2)
    ramp = np.clip((1.0 - radius) / edge_fade, 0.0, 1.0)
    return (ramp * ramp * (3.0 - 2.0 * ramp)).astype(np.float32)


def smoke_sheet_frames(frames: list[np.ndarray], settings: dict) -> list[np.ndarray]:
    """(density, lighting, lighting, density) frames of a puff growing and thinning with age."""
    size = int(settings["frame_px"])
    luminance = np.concatenate(
        [(frame[..., :3] @ LUMA)[frame[..., 3] > 0.1] for frame in frames]
    )
    dark, bright = np.percentile(luminance, [2.0, 98.0])
    floor = float(settings["lighting_floor"])
    young = float(settings["young_scale"])
    out = []
    for index, frame in enumerate(frames):
        age = index / (FRAMES - 1)
        lighting = (frame[..., :3] @ LUMA - dark) / max(float(bright - dark), 1e-6)
        lighting = np.clip(lighting, 0.0, 1.0)
        lighting = floor + (1.0 - floor) * lighting
        thinning = float(settings["old_density"]) + (
            1.0 - float(settings["old_density"])
        ) * (1.0 - age) ** float(settings["fade_power"])
        density_floor = float(settings.get("density_floor", 0.0))
        density = np.clip(frame[..., 3] - density_floor, 0.0, None) / (
            1.0 - density_floor
        )
        density = density ** float(settings.get("density_gamma", 1.0)) * thinning
        density = density * _edge_mask(
            density.shape[0], float(settings.get("edge_fade", 0.0))
        )
        puff = np.stack([lighting, lighting, lighting, density], axis=-1)
        side = max(8, round(size * (young + (1.0 - young) * np.sqrt(age))))
        puff = _resized(puff, side, side)
        square = np.zeros((size, size, 4), dtype=np.float32)
        square[..., :3] = floor
        left = (size - side) // 2
        square[left : left + side, left : left + side] = puff
        out.append(
            np.stack(
                [square[..., 3], square[..., 1], square[..., 1], square[..., 3]],
                axis=-1,
            )
        )
    return out


def _sheet(frames: list[np.ndarray]) -> np.ndarray:
    size = frames[0].shape[0]
    sheet = np.zeros((size * GRID, size * GRID, 4), dtype=np.float32)
    for index, frame in enumerate(frames):
        row, column = divmod(index, GRID)
        sheet[row * size : (row + 1) * size, column * size : (column + 1) * size] = (
            frame
        )
    return sheet


def build(raw_dir: Path, out_dir: Path) -> list[Path]:
    """Writes both sheets into `out_dir` and returns their paths."""
    settings = json.loads(SETTINGS.read_text(encoding="utf-8"))
    out_dir.mkdir(parents=True, exist_ok=True)
    url = settings["source_url"]
    sheets = {
        "flame_flipbook.png": flame_sheet_frames(
            source_frames(raw_dir, settings["flame"], url), settings["flame"]
        ),
        "smoke_flipbook.png": smoke_sheet_frames(
            source_frames(raw_dir, settings["smoke"], url), settings["smoke"]
        ),
    }
    paths = []
    for name, frames in sheets.items():
        sheet = (np.clip(_sheet(frames), 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)
        path = out_dir / name
        Image.fromarray(sheet, "RGBA").save(path, optimize=True)
        paths.append(path)
    return paths


if __name__ == "__main__":
    raw = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_RAW
    target = Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_OUT
    for written in build(raw, target):
        print(written)
