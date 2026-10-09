"""TX T1d bench: four routes to a seamless 2048 texture on three materials.

Routes: a) Z-Image 1024 -> Real-ESRGAN x4plus -> Lanczos 2048; b) 1024 -> light model
(realesr-animevideov3 x2; realesr-general-x4v3 has no ncnn release); c) Z-Image native 1536
-> Lanczos 2048; d) 1024 -> Lanczos 2048. Each ends with ``make_seamless``. One GPU process
at a time, no paid call. Outputs outside the repository.

Usage: uv run --project tools python tools/experiments/tx_upscale_bench.py
"""

from __future__ import annotations

import json
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

from cent_ans_tools.local_art import render_image
from cent_ans_tools.texture_factory.seamless import make_seamless
from cent_ans_tools.texture_factory.upscale import upscale

OUT_DIR = Path.home() / "dev" / "cent-ans-raw" / "textures" / "bench"
MATERIALS = {
    "prairie": (
        "top-down view of meadow ground, short grass with clover and soil, "
        "even overcast light, seamless texture, photographic"
    ),
    "oak_bark": (
        "close-up of old oak tree bark, deep vertical furrows, "
        "even overcast light, seamless texture, photographic"
    ),
    "limestone_wall": (
        "straight-on view of a medieval rubble limestone wall, uneven stones, "
        "lime mortar, even light, seamless texture, photographic"
    ),
}
SEED = 20261009
ROUTES = (
    ("a", "1024 + ESRGAN x4plus", 1024, "esrgan"),
    ("b", "1024 + light x2", 1024, "esrgan_light"),
    ("c", "1536 natif + Lanczos", 1536, "lanczos"),
    ("d", "1024 + Lanczos", 1024, "lanczos"),
)
CELL = 512
HALF_BAND = 160


def _seamless_save(src: Path, dst: Path) -> float:
    started = time.perf_counter()
    image = np.asarray(Image.open(src).convert("RGB"), dtype=np.float64)
    tiled = make_seamless(image, HALF_BAND)
    Image.fromarray(np.clip(tiled, 0, 255).astype(np.uint8)).save(dst)
    return time.perf_counter() - started


def run_bench() -> dict:
    """Run every material x route; return timings (seconds) and the output paths."""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    results: dict = {}
    for material, prompt in MATERIALS.items():
        raw: dict[int, Path] = {}
        timings: dict[str, float] = {}
        for side in (1024, 1536):
            raw[side] = OUT_DIR / f"{material}_raw{side}.png"
            if not raw[side].is_file():
                started = time.perf_counter()
                data = render_image(
                    prompt, aspect_ratio="1:1", seed=SEED, size=(side, side)
                )
                raw[side].write_bytes(data)
                timings[f"gen{side}"] = time.perf_counter() - started
                print(
                    material, "gen", side, f"{timings[f'gen{side}']:.1f}s", flush=True
                )
        for key, _label, side, method in ROUTES:
            large = OUT_DIR / f"{material}_{key}_up.png"
            final = OUT_DIR / f"{material}_{key}_2048.png"
            started = time.perf_counter()
            upscale(raw[side], large, method)
            timings[f"{key}_upscale"] = time.perf_counter() - started
            timings[f"{key}_seamless"] = _seamless_save(large, final)
            print(
                material, key, f"up {timings[f'{key}_upscale']:.1f}s",
                f"seamless {timings[f'{key}_seamless']:.1f}s", flush=True,
            )  # fmt: skip
        results[material] = timings
    (OUT_DIR / "timings.json").write_text(json.dumps(results, indent=2))
    return results


def build_board(results: dict) -> Path:
    """One PNG: a row per material, a column per route, centre 512 crops, captions."""
    caption_height = 44
    width = CELL * len(ROUTES)
    height = (CELL + caption_height) * len(MATERIALS)
    board = Image.new("RGB", (width, height), (24, 24, 24))
    draw = ImageDraw.Draw(board)
    for row, material in enumerate(MATERIALS):
        for column, (key, label, _side, _method) in enumerate(ROUTES):
            image = Image.open(OUT_DIR / f"{material}_{key}_2048.png")
            left = (image.width - CELL) // 2
            crop = image.crop((left, left, left + CELL, left + CELL))
            x, y = column * CELL, row * (CELL + caption_height)
            board.paste(crop, (x, y + caption_height))
            timings = results[material]
            side = 1536 if key == "c" else 1024
            total = timings[f"gen{side}"] + timings[f"{key}_upscale"]
            draw.text(
                (x + 6, y + 4), f"{material} - {key}) {label}", fill=(255, 255, 255)
            )
            draw.text(
                (x + 6, y + 22),
                f"gen {timings[f'gen{side}']:.0f}s + up {timings[f'{key}_upscale']:.0f}s"
                f" = {total:.0f}s",
                fill=(255, 220, 120),
            )
    path = OUT_DIR / "board.png"
    board.save(path)
    return path


if __name__ == "__main__":
    print(build_board(run_bench()))
