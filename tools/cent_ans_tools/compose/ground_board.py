"""Planches 2D du sol (lot CC CS) : recadrage de la carte de couleur, du splat et du relief.

Sert à juger le sol de près sans lancer Godot : la carte de couleur BC1 (niveau 0) est décodée par
bandes de blocs (jamais en entier), le splat et le relief sont recadrés autour d'un lieu
(lon / lat réels, projetés par le cartogramme).

    uv run --project tools python -m cent_ans_tools.compose.ground_board paris out.png
"""

from __future__ import annotations

import json
import sys
import zlib
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.geo import block_compress, download

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"

SITES = {
    "paris": (2.35, 48.85, 70),
    "apennin": (14.0, 41.9, 90),
    "calabre": (16.3, 39.0, 70),
    "konya": (32.5, 37.9, 90),
    "moscou": (37.6, 55.7, 90),
    "caire": (31.2, 30.0, 90),
}


def read_colormap_level0(
    map_dir: Path = MAP_DIR,
) -> tuple[np.memmap | np.ndarray, int, int]:
    """Blocs BC1 du niveau 0 ``(rows, cols, 8)`` et taille en pixels (décompression par parties)."""
    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))["colormap"][
        "bc1"
    ]
    width, height = meta["size_px"]
    need = ((width + 3) // 4) * ((height + 3) // 4) * 8
    chunks, have = [], 0
    index = 0
    while have < need:
        raw = zlib.decompress((map_dir / f"colormap_bc1_{index}.bin").read_bytes())
        chunks.append(raw)
        have += len(raw)
        index += 1
    data = b"".join(chunks)[:need]
    del chunks
    blocks = np.frombuffer(data, dtype=np.uint8).reshape(
        (height + 3) // 4, (width + 3) // 4, 8
    )
    return blocks, width, height


def crop_colormap(blocks: np.ndarray, x0: int, y0: int, x1: int, y1: int) -> np.ndarray:
    """RGB uint8 du recadrage en texels (multiples de 4 recommandés)."""
    b0x, b1x = x0 // 4, -(-x1 // 4)
    b0y, b1y = y0 // 4, -(-y1 // 4)
    sub = np.ascontiguousarray(blocks[b0y:b1y, b0x:b1x])
    image = block_compress.decode_bc1(sub.tobytes(), (b1y - b0y) * 4, (b1x - b0x) * 4)
    return np.clip(np.rint(image), 0, 255).astype(np.uint8)


def site_px(name: str, map_dir: Path = MAP_DIR) -> tuple[int, int, int]:
    """Centre ``(x, y)`` en pixels carte et demi-côté (px carte) d'un site."""
    from cent_ans_tools.geo.project import grid_from_metadata

    lon, lat, half = SITES[name]
    grid = grid_from_metadata(
        json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    )
    x, y = grid.lonlat_to_pixel(lon, lat)
    return int(x), int(y), half


def board(name: str, out: Path, size: int = 900, map_dir: Path = MAP_DIR) -> Path:
    """Planche : carte de couleur | splat (RGB = prairie, cultures, forêt) | relief ombré."""
    cx, cy, half = site_px(name, map_dir)
    x0, y0 = max(cx - half, 0), max(cy - half, 0)
    x1, y1 = x0 + 2 * half, y0 + 2 * half
    blocks, _, _ = read_colormap_level0(map_dir)
    colour = crop_colormap(blocks, x0 * 2, y0 * 2, x1 * 2, y1 * 2)
    del blocks
    with Image.open(map_dir / "splat.png") as image:
        splat = np.asarray(image.crop((x0, y0, x1, y1)).convert("RGBA"))
    panels = [
        Image.fromarray(colour).resize(
            (size // 2, size // 2), Image.Resampling.BICUBIC
        ),
        Image.fromarray(splat[..., :3]).resize(
            (size // 2, size // 2), Image.Resampling.NEAREST
        ),
    ]
    sheet = Image.new("RGB", (size, size // 2))
    for index, panel in enumerate(panels):
        sheet.paste(panel, (index * size // 2, 0))
    out.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out)
    return out


if __name__ == "__main__":
    board(sys.argv[1], Path(sys.argv[2]))
