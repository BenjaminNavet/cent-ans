"""Synthetic relief pyramid for ZG2 engine tests (ADR 0036), not real data.

Builds E1-E4 tiles over a small area (Paris / lower Seine by default) from the
versioned E0 tiles of ``data/map/height/``: bicubic upsampling plus
deterministic value-noise ridges (a few metres per level) so that finer levels
visibly add relief. Writes ``<out>/relief_pyramid.json`` and
``<out>/pyramid/E{k}/{col}_{row}.png`` (16-bit grayscale, same encoding as E0).

Usage (from the repository root)::

    uv run --project tools python game/tests/fixtures/zg2/make_pyramid.py OUT_DIR

then run the game with ``--pyramid-dir=OUT_DIR``.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[4]
E0_DIR = ROOT / "data" / "map" / "height"
TILE = 512
H_MIN = -200.0
H_RANGE = 5000.0
# E0 chunks (col, row) covered per level (Paris is in chunk (8, 7), Rouen too).
AREAS = {
    1: [(7, 6), (8, 6), (7, 7), (8, 7)],
    2: [(7, 6), (8, 6), (7, 7), (8, 7)],
    3: [(8, 7)],
}
# E4 only around Paris: world square [x0, x1) x [z0, z1).
E4_BOX = (2176.0, 2240.0, 1888.0, 1952.0)
# Noise octave added at each level: (wavelength in level pixels, amplitude in metres).
OCTAVES = {1: (4.0, 9.0), 2: (4.0, 5.0), 3: (4.0, 2.5), 4: (4.0, 1.2)}


def load_e0(col: int, row: int) -> np.ndarray:
    """E0 tile in metres, or sea level if missing."""
    path = E0_DIR / f"h_{col}_{row}.png"
    if not path.exists():
        return np.zeros((TILE, TILE), dtype=np.float32)
    raw = np.array(Image.open(path)).astype(np.float32)
    return H_MIN + raw / 65535.0 * H_RANGE


def value_noise(x: np.ndarray, y: np.ndarray, seed: int) -> np.ndarray:
    """Smooth value noise in [-1, 1] on an integer lattice."""
    xi = np.floor(x).astype(np.int64)
    yi = np.floor(y).astype(np.int64)
    fx = x - xi
    fy = y - yi
    ux = fx * fx * (3 - 2 * fx)
    uy = fy * fy * (3 - 2 * fy)

    def h(ix: np.ndarray, iy: np.ndarray) -> np.ndarray:
        v = (ix * 374761393 + iy * 668265263 + seed * 144269504) & 0xFFFFFFFF
        v = (v ^ (v >> 13)) * 1274126177 & 0xFFFFFFFF
        return ((v ^ (v >> 16)) & 0xFFFF) / 32767.5 - 1.0

    a, b = h(xi, yi), h(xi + 1, yi)
    c, d = h(xi, yi + 1), h(xi + 1, yi + 1)
    top = a + (b - a) * ux
    bottom = c + (d - c) * ux
    return top + (bottom - top) * uy


def detail(level: int, wx: np.ndarray, wz: np.ndarray) -> np.ndarray:
    """Sum of ridged octaves of levels 1..level at world coordinates (metres)."""
    total = np.zeros_like(wx)
    for k in range(1, level + 1):
        wave_px, amp = OCTAVES[k]
        wave_units = wave_px * 0.5 / (1 << k)
        n = value_noise(wx / wave_units, wz / wave_units, k)
        total += amp * (1.0 - 2.0 * np.abs(n))
    return total


def mosaic(col: int, row: int, margin: int) -> np.ndarray:
    """E0 tile with a margin of neighbouring pixels (metres)."""
    big = np.zeros((3 * TILE, 3 * TILE), dtype=np.float32)
    for dr in (-1, 0, 1):
        for dc in (-1, 0, 1):
            big[
                (dr + 1) * TILE : (dr + 2) * TILE, (dc + 1) * TILE : (dc + 2) * TILE
            ] = load_e0(col + dc, row + dr)
    return big[TILE - margin : 2 * TILE + margin, TILE - margin : 2 * TILE + margin]


def write_tile(out: Path, level: int, col: int, row: int, metres: np.ndarray) -> None:
    """Encode a tile as 16-bit grayscale PNG."""
    raw = np.clip(np.round((metres - H_MIN) / H_RANGE * 65535.0), 0, 65535).astype(
        "<u2"
    )
    path = out / "pyramid" / f"E{level}" / f"{col}_{row}.png"
    path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(raw).save(path)


def build_level(
    out: Path, level: int, chunks: list[tuple[int, int]], box=None
) -> list[tuple[int, int]]:
    """Write every tile of ``level`` inside ``chunks`` (and ``box`` if given)."""
    written = []
    f = 1 << level
    ps = 0.5 / f
    margin = 4
    for c0, r0 in chunks:
        src = mosaic(c0, r0, margin)
        size = src.shape[0] * f
        up = np.array(Image.fromarray(src).resize((size, size), Image.BICUBIC))
        up = up[margin * f : margin * f + TILE * f, margin * f : margin * f + TILE * f]
        for j in range(f):
            for i in range(f):
                col, row = c0 * f + i, r0 * f + j
                t = 256.0 / f
                ox, oz = col * t - 0.5, row * t - 0.5
                if box and not (box[0] <= ox < box[1] and box[2] <= oz < box[3]):
                    continue
                block = up[j * TILE : (j + 1) * TILE, i * TILE : (i + 1) * TILE]
                px = (np.arange(TILE) + 0.5) * ps
                wx, wz = np.meshgrid(ox + px, oz + px)
                land = block > 0.5
                bumps = detail(level, wx, wz) * np.clip(block / 20.0, 0, 1)
                tile = np.where(land, block + bumps, block)
                write_tile(out, level, col, row, tile.astype(np.float32))
                written.append((col, row))
    return written


def rle(tiles: list[tuple[int, int]]) -> list[dict]:
    """Rows of [start, length] runs, as in relief_pyramid.schema.json."""
    rows: dict[int, list[int]] = {}
    for col, row in tiles:
        rows.setdefault(row, []).append(col)
    result = []
    for row in sorted(rows):
        runs: list[list[int]] = []
        for col in sorted(rows[row]):
            if runs and runs[-1][0] + runs[-1][1] == col:
                runs[-1][1] += 1
            else:
                runs.append([col, 1])
        result.append({"row": row, "runs": runs})
    return result


def main() -> None:
    """Entry point."""
    out = Path(sys.argv[1]).resolve()
    manifest = json.loads((ROOT / "data" / "map" / "relief_pyramid.json").read_text())
    manifest["description"] = (
        "Pyramide SYNTHÉTIQUE de test ZG2 (make_pyramid.py), pas des données réelles."
    )
    for entry in manifest["levels"]:
        level = entry["level"]
        if level in AREAS:
            entry["tiles_rle"] = rle(build_level(out, level, AREAS[level]))
        elif level == 4:
            entry["tiles_rle"] = rle(build_level(out, 4, [(8, 7)], E4_BOX))
        else:
            entry["tiles_rle"] = []
        count = sum(run[1] for row in entry["tiles_rle"] for run in row["runs"])
        print(f"E{level}: {count} tiles")
    (out / "relief_pyramid.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2)
    )


if __name__ == "__main__":
    main()
