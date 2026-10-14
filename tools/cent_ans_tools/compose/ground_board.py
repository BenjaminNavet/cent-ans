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


def crop_bake(
    name: str,
    style_path: Path | None = None,
    map_dir: Path = MAP_DIR,
    context_px: int = 60,
    inputs=None,  # noqa: ANN001 - colormap.ColormapInputs, loaded once by the caller for several sites
) -> np.ndarray:
    """Carte de couleur d'un site cuite sur un recadrage (``context_px`` de contexte autour).

    Sert à juger un réglage de style sans cuire la carte entière ; renvoie le centre du recadrage.
    """
    import dataclasses

    from cent_ans_tools.geo import colormap

    cx, cy, half = site_px(name, map_dir)
    style = colormap.load_style(style_path or colormap.STYLE_PATH)
    full = inputs if inputs is not None else colormap.load_inputs(map_dir)
    r0, r1 = max(cy - half - context_px, 0), min(cy + half + context_px, full.shape[0])
    c0, c1 = max(cx - half - context_px, 0), min(cx + half + context_px, full.shape[1])
    kwargs = {
        f.name: getattr(full, f.name)
        for f in dataclasses.fields(full)
        if f.name not in ("row0", "shared")
    }
    window = (slice(r0, r1), slice(c0, c1))
    for key in (
        "land",
        "splat",
        "height_m",
        "coast_dist_px",
        "wetlands",
        "conifer",
        "lon",
        "lat",
        "biomes",
        "river_km",
    ):
        if kwargs[key] is not None:
            kwargs[key] = np.ascontiguousarray(kwargs[key][window])
    shift = np.array([c0, r0], dtype=np.float64)
    kwargs["settlements"] = kwargs["settlements"] - shift
    kwargs["roads"] = [(kind, pts - shift) for kind, pts in kwargs["roads"]]
    kwargs["towns"] = [(x - c0, y - r0, ha) for x, y, ha in kwargs["towns"]]
    kwargs["reservoirs"] = kwargs["reservoirs"] - shift
    baked = colormap.bake(colormap.ColormapInputs(**kwargs), style, 512)
    scale = int(style["scale"])
    x0, y0 = (cx - half - c0) * scale, (cy - half - r0) * scale
    return baked[y0 : y0 + 2 * half * scale, x0 : x0 + 2 * half * scale]


def compare_boards(
    names: list[str], out_dir: Path, tag: str, size: int = 900
) -> list[Path]:
    """Planches « actuelle (fichier) | style courant (recadrage cuit) » pour plusieurs sites."""
    from cent_ans_tools.geo import colormap

    inputs = colormap.load_inputs(MAP_DIR)
    blocks, _, _ = read_colormap_level0(MAP_DIR)
    paths = []
    for name in names:
        cx, cy, half = site_px(name)
        current = crop_colormap(
            blocks, (cx - half) * 2, (cy - half) * 2, (cx + half) * 2, (cy + half) * 2
        )
        baked = crop_bake(name, inputs=inputs)
        sheet = Image.new("RGB", (size, size // 2))
        for index, array in enumerate((current, baked)):
            panel = Image.fromarray(array).resize(
                (size // 2, size // 2), Image.Resampling.BICUBIC
            )
            sheet.paste(panel, (index * size // 2, 0))
        out = out_dir / f"{tag}_{name}.png"
        out_dir.mkdir(parents=True, exist_ok=True)
        sheet.save(out)
        paths.append(out)
    return paths


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
    if (
        sys.argv[1] == "compare"
    ):  # compare <tag> <site>... : planches dans ~/dev/cent-ans-raw/compose/cs
        compare_boards(
            sys.argv[3:], Path.home() / "dev/cent-ans-raw/compose/cs", sys.argv[2]
        )
    else:
        board(sys.argv[1], Path(sys.argv[2]))
