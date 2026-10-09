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


def grid_shape(count: int) -> tuple[int, int]:
    """(columns, rows) with columns * rows == count, as square as possible, <= 16 k px."""
    columns = max(c for c in range(1, int(np.sqrt(count)) + 1) if count % c == 0)
    return columns, count // columns


def import_file(columns: int, rows: int, source: str, res_dir: str = RES_DIR) -> str:
    """Godot ``.import`` of a grid image as a VRAM-compressed Texture2DArray (as GA4)."""
    return f"""[remap]

importer="2d_array_texture"
type="CompressedTexture2DArray"

[deps]

source_file="{res_dir}{source}"

[params]

compress/mode=2
compress/high_quality=false
compress/lossy_quality=0.7
compress/uastc_level=0
compress/rdo_quality_loss=0.0
compress/hdr_compression=1
compress/channel_pack=0
mipmaps/generate=true
mipmaps/limit=-1
slices/horizontal={columns}
slices/vertical={rows}
"""


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
    columns, rows = grid_shape(len(materials))
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
    texture_dir.mkdir(parents=True, exist_ok=True)
    albedo_path = texture_dir / albedo_name
    normal_path = texture_dir / normal_name
    # `optimize` needs the whole JPEG in Pillow's output buffer (large grids fail otherwise).
    ImageFile.MAXBLOCK = max(
        ImageFile.MAXBLOCK, albedo_grid.width * albedo_grid.height * 3
    )
    albedo_grid.save(albedo_path, quality=albedo_quality, subsampling=0, optimize=True)
    normal_grid.save(normal_path, quality=normal_quality, subsampling=0, optimize=True)
    for path in (albedo_path, normal_path):
        imported = Path(f"{path}.import")
        slicing = f"slices/horizontal={columns}\nslices/vertical={rows}"
        # Keep Godot's own .import (uid, imported paths) while the grid is unchanged.
        if not imported.exists() or slicing not in imported.read_text():
            imported.write_text(import_file(columns, rows, path.name, res_dir))
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


def texture_import(res_source: str, *, normal: bool = False) -> str:
    """Godot ``.import`` of a plain texture used in 3D: VRAM compressed, mipmapped (QW-D rule)."""
    return f"""[remap]

importer="texture"
type="CompressedTexture2D"

[deps]

source_file="{res_source}"

[params]

compress/mode=2
compress/high_quality=false
compress/lossy_quality=0.7
compress/uastc_level=0
compress/rdo_quality_loss=0.0
compress/hdr_compression=1
compress/normal_map={1 if normal else 0}
compress/channel_pack=0
mipmaps/generate=true
mipmaps/limit=-1
roughness/mode=0
roughness/src_normal=""
process/fix_alpha_border=true
process/premult_alpha=false
process/normal_map_invert_y=false
process/hdr_as_srgb=false
process/hdr_clamp_exposure=false
process/size_limit=0
detect_3d/compress_to=1
"""


def write_texture_import(
    path: Path, game_dir: Path, *, normal: bool = False, overwrite: bool = False
) -> Path:
    """Write ``<path>.import`` unless Godot already did (its uid and paths are kept)."""
    imported = Path(f"{path}.import")
    if overwrite or not imported.exists():
        imported.write_text(
            texture_import(
                "res://" + path.relative_to(game_dir).as_posix(), normal=normal
            )
        )
    return imported
