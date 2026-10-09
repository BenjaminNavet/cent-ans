"""Procedural sea normal map of the campaign (own work, CC0) and the normal+roughness packer.

The campaign ground textures are no longer downloaded from Poly Haven (ADR 0244): the regional
TX packs are the only path (``cent-ans textures``). What stays here:

``water_normal.png`` (lot GA4)
    Tileable procedural sea normal map: sum of wave trains with integer wave vectors, so the
    tile wraps exactly. Size from ``water.normal_size`` of
    ``data/fx/campaign_terrain_textures.json``.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.geo import download

REPO_DIR = download.TOOLS_DIR.parent
TEXTURE_DIR = REPO_DIR / "game" / "assets" / "textures" / "terrain"
SPEC_PATH = REPO_DIR / "data" / "fx" / "campaign_terrain_textures.json"


def load_spec(path: Path = SPEC_PATH) -> dict:
    """Read the campaign terrain texture data file."""
    return json.loads(path.read_text(encoding="utf-8"))


def pack_normal_rough(normal_rgb: np.ndarray, rough: np.ndarray) -> np.ndarray:
    """Pack normal X/Y and roughness into one ``uint8`` RGB image.

    Args:
        normal_rgb: ``(h, w, 3)`` OpenGL normal map (``uint8``).
        rough: ``(h, w)`` roughness (``uint8``).
    """
    packed = normal_rgb[..., :3].copy()
    packed[..., 2] = rough
    return packed


def water_normal(size: int = 1024, seed: int = 1337) -> np.ndarray:
    """Tileable sea normal map (``uint8`` RGB, OpenGL convention).

    Sum of wave trains whose wave vectors are integer multiples of the tile
    frequency (exact wrap), amplitude falling with frequency, with a dominant
    wind direction; normals from the analytic gradient.
    """
    rng = np.random.default_rng(seed)
    coords = np.arange(size, dtype=np.float64) / size
    x, y = np.meshgrid(coords, coords)
    grad_x = np.zeros((size, size))
    grad_y = np.zeros((size, size))
    wind = np.array([0.8, 0.6])
    for _ in range(64):
        while True:
            k = rng.integers(-24, 25, size=2)
            magnitude = float(np.hypot(*k))
            if 2.0 <= magnitude <= 24.0:
                break
        alignment = abs(float(np.dot(k / magnitude, wind)))
        amplitude = (0.35 + 0.65 * alignment**2) / magnitude**1.6
        phase = rng.uniform(0.0, 2.0 * np.pi)
        arg = 2.0 * np.pi * (k[0] * x + k[1] * y) + phase
        derivative = -amplitude * 2.0 * np.pi * np.sin(arg)
        grad_x += derivative * k[0]
        grad_y += derivative * k[1]
    scale = 1.0 / max(float(np.abs(grad_x).max()), float(np.abs(grad_y).max()), 1e-9)
    nx, ny = -grad_x * scale, -grad_y * scale
    nz = np.ones_like(nx)
    length = np.sqrt(nx * nx + ny * ny + nz * nz)
    normal = np.stack([nx / length, ny / length, nz / length], axis=-1)
    return np.clip((normal * 0.5 + 0.5) * 255.0 + 0.5, 0, 255).astype(np.uint8)


def build(texture_dir: Path = TEXTURE_DIR) -> list[Path]:
    """Write the procedural sea normal map."""
    texture_dir.mkdir(parents=True, exist_ok=True)
    water_path = texture_dir / "water_normal.png"
    water_size = int(load_spec()["water"]["normal_size"])
    Image.fromarray(water_normal(water_size), mode="RGB").save(water_path, optimize=True)
    return [water_path]
