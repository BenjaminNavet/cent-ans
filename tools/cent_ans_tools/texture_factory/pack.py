"""Texture-array packing and manifest (T1b); defaults are the HB ground values."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image, ImageFile

from cent_ans_tools.texture_factory.color import srgb_to_linear

RES_DIR = "res://assets/textures/terrain/"
ALBEDO_NAME = "hb_ground_albedo_array.jpg"
NORMAL_NAME = "hb_ground_normal_array.jpg"
MAX_PACK_BYTES = 40_000_000
ALBEDO_QUALITY = 88
NORMAL_QUALITY = 90


MAX_GRID_PX = 16_384


def grid_shape(count: int, layer_size: int = 1024) -> tuple[int, int]:
    """(columns, rows) of the smallest grid of ``n >= count`` cells that fits 16 384 px.

    The most square factorisation of ``n`` is taken; ``n == count`` whenever that fits
    (42 -> 6x7). A count that only factors badly (43 is prime: 1x43 is too tall) is padded
    to the next ``n`` (44 -> 4x11); the packer fills the extra cells with a copy of the
    last layer.
    """
    limit = max(1, MAX_GRID_PX // layer_size)
    n = count
    while True:
        columns = max(c for c in range(1, int(np.sqrt(n)) + 1) if n % c == 0)
        rows = n // columns
        if rows <= limit or n >= count + limit * limit:
            return columns, rows
        n += 1


def _resize_normal(normal: Image.Image, size: int) -> Image.Image:
    """Downsample a normal + roughness tile, renormalising the XY normal."""
    small = (
        np.asarray(normal.resize((size, size), Image.Resampling.BOX), dtype=np.float64)
        / 255.0
    )
    xy = small[..., :2] * 2 - 1
    length = np.sqrt((xy**2).sum(-1) + np.clip(1 - (xy**2).sum(-1), 0, 1))
    small[..., :2] = xy / np.maximum(length, 1e-6)[..., np.newaxis] * 0.5 + 0.5
    return Image.fromarray(np.clip(np.rint(small * 255), 0, 255).astype(np.uint8))


def pack(
    document: dict[str, Any],
    raw_dir: Path,
    texture_dir: Path,
    manifest_path: Path,
    *,
    albedo_name: str = ALBEDO_NAME,
    normal_name: str = NORMAL_NAME,
    res_dir: str = RES_DIR,
    max_bytes: int = MAX_PACK_BYTES,
    albedo_quality: int = ALBEDO_QUALITY,
    normal_quality: int = NORMAL_QUALITY,
) -> dict[str, Any]:
    """Assemble the tiles into the two grid images, their imports and the manifest."""
    materials = document["materials"]
    size = document["layer_size"]
    normal_size = document.get("normal_size", size)
    columns, rows = grid_shape(len(materials), size)
    albedo_grid = Image.new("RGB", (columns * size, rows * size))
    normal_grid = Image.new("RGB", (columns * normal_size, rows * normal_size))
    layers = []
    tiles = raw_dir / "tiles"
    for entry in materials:
        albedo = Image.open(tiles / f"{entry['id']}_albedo.png").convert("RGB")
        normal = Image.open(tiles / f"{entry['id']}_normal.png").convert("RGB")
        if albedo.size != (size, size) or normal.size != (size, size):
            raise ValueError(
                f"{entry['id']} : tuile de taille {albedo.size}, {size}² attendu"
            )
        column, row = entry["layer"] % columns, entry["layer"] // columns
        albedo_grid.paste(albedo, (column * size, row * size))
        if normal_size != size:
            normal = _resize_normal(normal, normal_size)
        normal_grid.paste(normal, (column * normal_size, row * normal_size))
        last_cell = (albedo, normal)
        mean = srgb_to_linear(np.asarray(albedo, dtype=np.float64) / 255.0)
        layers.append(
            {
                "id": entry["id"],
                "layer": entry["layer"],
                "role": entry["role"],
                "biomes": entry["biomes"],
                **({"regions": entry["regions"]} if "regions" in entry else {}),
                **({"species": entry["species"]} if "species" in entry else {}),
                "tile_m": entry["tile_m"],
                "mean_linear": [
                    round(float(v), 4) for v in mean.reshape(-1, 3).mean(0)
                ],
            }
        )
    # Padding cells (grid larger than the layer count) repeat the last layer.
    for cell in range(len(materials), columns * rows):
        albedo_grid.paste(last_cell[0], ((cell % columns) * size, (cell // columns) * size))
        normal_grid.paste(
            last_cell[1],
            ((cell % columns) * normal_size, (cell // columns) * normal_size),
        )
    texture_dir.mkdir(parents=True, exist_ok=True)
    albedo_path = texture_dir / albedo_name
    normal_path = texture_dir / normal_name
    # `optimize` needs the whole JPEG in Pillow's output buffer (large grids fail otherwise).
    ImageFile.MAXBLOCK = max(
        ImageFile.MAXBLOCK, albedo_grid.width * albedo_grid.height * 3
    )
    albedo_grid.save(albedo_path, quality=albedo_quality, subsampling=0, optimize=True)
    normal_grid.save(normal_path, quality=normal_quality, subsampling=0, optimize=True)
    total = albedo_path.stat().st_size + normal_path.stat().st_size
    if total > max_bytes:
        raise ValueError(
            f"tableaux trop lourds : {total / 1e6:.1f} Mo > {max_bytes / 1e6:.0f} Mo"
        )
    manifest = {
        "layer_size": size,
        "normal_size": normal_size,
        "grid": [columns, rows],
        "albedo": res_dir + albedo_name,
        "normal": res_dir + normal_name,
        "layers": layers,
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=1, ensure_ascii=False) + "\n")
    return {"grid": [columns, rows], "bytes": total, "layers": len(layers)}
