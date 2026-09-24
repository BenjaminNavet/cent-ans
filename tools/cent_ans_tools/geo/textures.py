"""CC0 PBR textures of the campaign terrain, downloaded from Poly Haven.

Each material layer gets two 1k JPEG files in ``game/assets/textures/terrain/``:

``<layer>_albedo.jpg``
    Poly Haven ``Diffuse`` map (sRGB).
``<layer>_normal_rough.jpg``
    R, G = OpenGL normal map X, Y (``nor_gl``), B = roughness (``Rough``); the
    shader rebuilds Z. One file per layer keeps the Godot ``Texture2DArray`` at
    two arrays (albedo, normal + roughness).

API: ``https://api.polyhaven.com/files/<asset id>`` (free, CC0, no key).
"""

from __future__ import annotations

import io
from pathlib import Path

import httpx
import numpy as np
from PIL import Image

from cent_ans_tools.geo import download

REPO_DIR = download.TOOLS_DIR.parent
TEXTURE_DIR = REPO_DIR / "game" / "assets" / "textures" / "terrain"
API_URL = "https://api.polyhaven.com/files/{asset}"
RESOLUTION = "1k"
SIZE = 1024
JPEG_QUALITY = 92

# Ordre = index de couche dans les Texture2DArray du shader (terrain.gdshader).
LAYERS: dict[str, str] = {
    "grass": "aerial_grass_rock",
    "farmland": "aerial_mud_1",
    "forest": "forrest_ground_01",
    "rock": "aerial_rocks_01",
    "heath": "sparse_grass",
    "snow": "snow_field_aerial",
    "sand": "aerial_beach_01",
}


def pack_normal_rough(normal_rgb: np.ndarray, rough: np.ndarray) -> np.ndarray:
    """Pack normal X/Y and roughness into one ``uint8`` RGB image.

    Args:
        normal_rgb: ``(h, w, 3)`` OpenGL normal map (``uint8``).
        rough: ``(h, w)`` roughness (``uint8``).
    """
    packed = normal_rgb[..., :3].copy()
    packed[..., 2] = rough
    return packed


def _fetch(client: httpx.Client, url: str) -> Image.Image:
    response = client.get(url, follow_redirects=True, timeout=120.0)
    response.raise_for_status()
    return Image.open(io.BytesIO(response.content))


def _map_url(files: dict, key: str) -> str:
    return files[key][RESOLUTION]["jpg"]["url"]


def build(force: bool = False, texture_dir: Path = TEXTURE_DIR) -> list[Path]:
    """Download every layer (skipped if present unless ``force``) and return the paths."""
    texture_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    with httpx.Client() as client:
        for layer, asset in LAYERS.items():
            albedo_path = texture_dir / f"{layer}_albedo.jpg"
            packed_path = texture_dir / f"{layer}_normal_rough.jpg"
            if albedo_path.exists() and packed_path.exists() and not force:
                written += [albedo_path, packed_path]
                continue
            response = client.get(API_URL.format(asset=asset), timeout=60.0)
            response.raise_for_status()
            files = response.json()
            albedo = _fetch(client, _map_url(files, "Diffuse")).convert("RGB")
            normal = _fetch(client, _map_url(files, "nor_gl")).convert("RGB")
            rough = _fetch(client, _map_url(files, "Rough")).convert("L")
            albedo = albedo.resize((SIZE, SIZE), Image.Resampling.LANCZOS)
            normal = normal.resize((SIZE, SIZE), Image.Resampling.LANCZOS)
            rough = rough.resize((SIZE, SIZE), Image.Resampling.LANCZOS)
            albedo.save(albedo_path, quality=JPEG_QUALITY, subsampling=0)
            packed = pack_normal_rough(np.asarray(normal), np.asarray(rough))
            Image.fromarray(packed, mode="RGB").save(
                packed_path, quality=JPEG_QUALITY, subsampling=0
            )
            written += [albedo_path, packed_path]
    return written
