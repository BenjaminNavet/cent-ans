"""Relief pyramid of the campaign map, tiers 1-2 (chantier ZG, lot ZG1, ADR 0036).

Quadtree of 512² 16-bit tiles in ``data/map/pyramid/E{level}/{col}_{row}.png``
(gitignored), same encoding and EPSG:3035 grid as the E0 tiles of
``data/map/height/``: level ``k`` is a virtual ``8192 * 2**k`` pixel grid over
the ``map.json`` bounds, one tile covers ``256 / 2**k`` world units, i.e.
``359.49 / 2**k`` m per pixel, pixel centred. Tile ``(k, col, row)`` has the
children ``(k + 1, 2 col + dx, 2 row + dy)``.

- E1-E2 (tier 1): Copernicus GLO-90 area average, land of the fine bbox. Baked
  per E1 tile: E2 is computed on the 1024² footprint, E1 is its 2 x 2 mean.
- E3-E4 (tier 2): Copernicus GLO-30 corrected into a terrain model (see
  :mod:`cent_ans_tools.geo.surface`), land of the core bbox. Baked per E2
  footprint: E4 is computed on 2048² (plus a margin), E3 is its 2 x 2 mean.

Every level carries the render boost of ``heightmap_render.png`` (ADR 0019:
``h + 0.8 clamp(h - blur_5km(h), ±120 m)`` faded out between 600 and 1600 m of
blurred altitude). The blurred base is the very field E0 was boosted with (σ 5 km
on the 8192² Copernicus + ETOPO mosaic), sampled bilinearly at every level, so the
boost is continuous across levels: only the clamp is non-linear. The coast is
E0's up to E2 (land where the bilinear E0 surface is above sea level); from E3 it
follows the source near E0's shore, with a 2-pixel fade to the water.

``data/map/relief_pyramid.json`` is updated line by line: only the ``levels``
entries baked here and the ``cache`` line change (lot ZG3 writes E5-E7 the same
way in parallel).
"""

from __future__ import annotations

import json
import os
import shutil
import time
from collections.abc import Iterable
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path

import numpy as np
from PIL import Image
from rasterio.enums import Resampling
from scipy import ndimage

from cent_ans_tools.geo import (
    bake_stamp,
    copernicus,
    download,
    relief,
    relief_shade,
    terrain,
    world_frame,
)
from cent_ans_tools.geo.project import MapGrid

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
MANIFEST = "relief_pyramid.json"
PYRAMID_DIR_NAME = "pyramid"
WORK_DIR = download.RAW_DIR / "pyramid_work"
TILE_PX = 512
ROOT_TILE_UNITS = 256
#: Side of the legacy pyramid frame: 16 root tiles of 256 units (the 4096² map
#: before OM2). Since ADR 0115 the world is 28 x 24 root tiles. A cache whose
#: manifest carries a non-zero ``root_origin_tiles`` is in this legacy square
#: frame, placed at that offset (tile (k, col, row) of the cache = world tile
#: (col + dx·2^k, row + dy·2^k)); since ADR 0121 (OMR R7) the cache is baked in
#: the world frame (offset ``[0, 0]``, 28 x 24 root tiles). Geometry helpers
#: taking ``bounds`` derive the frame size from them (:func:`frame_tiles`).
E0_SIZE_PX = 8192
E0_TILES = E0_SIZE_PX // TILE_PX
#: Metres per world unit (map scale, fixed by ADR 0082) and per root tile.
UNIT_M = 718.9765625
ROOT_TILE_M = ROOT_TILE_UNITS * UNIT_M
#: Tier-1 land box: every land of the frame (ADR 0121; was the GLO-90 box of the
#: West, ``copernicus.FINE_BBOX``, before OMR R7).
TIER1_BBOX = (-180.0, -90.0, 180.0, 90.0)

TIER1_LEVELS = (1, 2)
TIER2_LEVELS = (3, 4)
#: ``(lon_min, lat_min, lon_max, lat_max)`` of tier 2 (ADR 0036 "cœur").
CORE_BBOX = (-6.0, 42.0, 9.0, 56.0)
#: Parts of :data:`CORE_BBOX` left out of tier 2 to hold the cache budget
#: (≤ 2.5 GB for E1-E4, ADR 0036: shrink the core before the resolution),
#: keeping France, England and Wales, the Low Countries and the left bank of the
#: Rhine. Boxes ``(lon_min, lat_min, lon_max, lat_max)``.
CORE_EXCLUDE: tuple[tuple[str, tuple[float, float, float, float]], ...] = (
    ("Espagne (Galice à la Navarre)", (-6.0, 42.0, -1.8, 44.0)),
    ("Espagne (Aragon, Catalogne)", (-6.0, 42.0, 9.0, 42.35)),
    ("Italie (Ligurie)", (7.55, 42.0, 9.0, 44.2)),
    ("Italie (Piémont, Val d'Aoste)", (7.2, 44.2, 9.0, 46.1)),
    ("Suisse", (6.8, 46.1, 9.0, 47.5)),
    ("Allemagne (rive droite du Rhin)", (8.0, 47.5, 9.0, 56.0)),
    ("Allemagne (Westerwald, Sauerland)", (7.2, 50.4, 8.0, 56.0)),
    ("Allemagne (Bergisches Land, Ruhr)", (7.0, 50.8, 7.2, 56.0)),
    ("Écosse", (-6.0, 55.1, -2.1, 56.0)),
    ("Irlande", (-6.0, 51.6, -5.4, 56.0)),
)
#: E4 pixels of margin around each tier-2 block (surface correction filters).
TIER2_MARGIN_PX = 128
COAST_FADE_PX = 2
GLO90_RES_DEG = 1.0 / 1200.0
#: Bump when the E1-E4 bake changes (stamped in ``pyramid/bake.json``, expected by
#: the manifest's ``bake_versions``, see :mod:`bake_stamp`): a stale tier is
#: rebaked by ``geo pyramid`` / ``geo relief-all`` without ``--force``.
#: 2 (SZ2): valley floors not dug (:func:`relief_shade.valley_floor`).
BAKE_VERSION = 2
TIER_NAMES = {TIER1_LEVELS: "tier1", TIER2_LEVELS: "tier2"}
#: Bake version per tier. Tier 1: 3 (OMR R7, ADR 0121): world frame, every land
#: of the 28 x 24 world from GLO-90 (was the West box only).
TIER_VERSIONS = {"tier1": 3, "tier2": BAKE_VERSION}
#: Tier-1 bake streamed by blocks of ``STREAM_BLOCK`` x ``STREAM_BLOCK`` E1 tiles:
#: download the block's GLO-90 tiles, bake, delete the raw tiles no later block
#: needs (outside ``copernicus.FINE_BBOX``, kept for ``geo relief-shade``).
STREAM_BLOCK = 4
#: Stop the streamed bake (cleanly, resumable) below this free disk space.
STREAM_MIN_FREE_BYTES = 25 * 1024**3
#: Manifest fields of E1-E2 (ADR 0121).
TIER1_MANIFEST = {
    "source": "Copernicus DEM GLO-90",
    "bbox_lonlat": [-11.0, 28.0, 61.0, 66.0],
}


@dataclass(frozen=True)
class TileKey:
    """Address of one pyramid tile."""

    level: int
    col: int
    row: int

    def children(self) -> list[TileKey]:
        """The four tiles of the next level covering this one."""
        return [
            TileKey(self.level + 1, 2 * self.col + dx, 2 * self.row + dy)
            for dy in (0, 1)
            for dx in (0, 1)
        ]

    def parent(self) -> TileKey:
        """The tile of the previous level containing this one."""
        return TileKey(self.level - 1, self.col // 2, self.row // 2)


@dataclass
class PyramidResult:
    """Summary of a pyramid build."""

    tiles_written: int
    total_bytes: int
    seconds: float
    #: Tiles written per level.
    per_level: dict[int, int] = field(default_factory=dict)
    skipped_units: int = 0


# --------------------------------------------------------------------------- geometry


def level_size_px(level: int) -> int:
    """Side of the virtual grid of ``level`` in the legacy square frame (E0 = 8192)."""
    return E0_SIZE_PX << level


def level_tiles(level: int) -> int:
    """Tiles per side at ``level`` in the legacy square frame (E0 = 16)."""
    return E0_TILES << level


def _root_count(length_m: float) -> int:
    """Root tiles along a side of ``length_m`` (legacy 16 for synthetic bounds)."""
    count = length_m / ROOT_TILE_M
    rounded = round(count)
    if rounded > 0 and abs(count - rounded) < 1e-6:
        return int(rounded)
    return E0_TILES


def frame_tiles(bounds: tuple[float, float, float, float]) -> tuple[int, int]:
    """``(cols, rows)`` of root tiles of the frame over ``bounds``.

    Real bounds are whole root tiles of :data:`ROOT_TILE_M` (16 x 16 for the
    legacy frame, 28 x 24 for the world); other (synthetic, test) bounds are
    taken as the legacy square of 16 root tiles.
    """
    cols = _root_count(bounds[2] - bounds[0])
    rows = _root_count(bounds[3] - bounds[1])
    if cols == E0_TILES or rows == E0_TILES:
        width, height = bounds[2] - bounds[0], bounds[3] - bounds[1]
        if abs(width - height) < 1e-6 * max(width, 1.0):
            return E0_TILES, E0_TILES
    return cols, rows


def frame_width_units(bounds: tuple[float, float, float, float]) -> float:
    """Width of the frame in world units (4096 legacy, 7168 world)."""
    return float(frame_tiles(bounds)[0] * ROOT_TILE_UNITS)


def level_shape_tiles(
    bounds: tuple[float, float, float, float], level: int
) -> tuple[int, int]:
    """``(cols, rows)`` of tiles of ``level`` over the frame."""
    cols, rows = frame_tiles(bounds)
    return cols << level, rows << level


def level_grid(bounds: tuple[float, float, float, float], level: int) -> MapGrid:
    """EPSG:3035 grid of ``level`` over the frame bounds."""
    cols, rows = level_shape_tiles(bounds, level)
    return MapGrid(tuple(bounds), cols * TILE_PX, rows * TILE_PX)


def level_meters_per_px(bounds: tuple[float, float, float, float], level: int) -> float:
    """Pixel size of ``level`` in metres."""
    return (bounds[2] - bounds[0]) / (level_shape_tiles(bounds, level)[0] * TILE_PX)


def tile_window(key: TileKey) -> tuple[int, int, int, int]:
    """``(col0, row0, cols, rows)`` of a tile in the pixels of its level grid."""
    return key.col * TILE_PX, key.row * TILE_PX, TILE_PX, TILE_PX


def tile_bounds(
    bounds: tuple[float, float, float, float], key: TileKey
) -> tuple[float, float, float, float]:
    """``(minx, miny, maxx, maxy)`` of a tile in EPSG:3035 metres."""
    side = (bounds[2] - bounds[0]) / level_shape_tiles(bounds, key.level)[0]
    minx = bounds[0] + key.col * side
    maxy = bounds[3] - key.row * side
    return minx, maxy - side, minx + side, maxy


def tile_path(map_dir: Path, key: TileKey) -> Path:
    """Path of a tile of level >= 1 in the gitignored cache."""
    return map_dir / PYRAMID_DIR_NAME / f"E{key.level}" / f"{key.col}_{key.row}.png"


def e0_coordinates(level: int, start: int, count: int) -> np.ndarray:
    """Continuous E0 pixel indices (pixel ``i`` centred on ``i``) of level pixels."""
    factor = float(1 << level)
    return (np.arange(start, start + count, dtype=np.float64) + 0.5) / factor - 0.5


def sample_e0_grid(
    array: np.ndarray, level: int, window: tuple[int, int, int, int], order: int = 1
) -> np.ndarray:
    """Bilinear (``order=1``) or nearest (``order=0``) sample of an E0-grid array.

    Args:
        array: E0-grid array (may be a memory map).
        level: Level of the target window.
        window: ``(col0, row0, cols, rows)`` in pixels of ``level``.
        order: Interpolation order.

    Returns:
        ``float32`` array ``(rows, cols)``; edges are clamped.
    """
    col0, row0, cols, rows = window
    xs = e0_coordinates(level, col0, cols)
    ys = e0_coordinates(level, row0, rows)
    height, width = array.shape[:2]
    x_lo = max(int(np.floor(xs[0])) - 1, 0)
    x_hi = min(int(np.ceil(xs[-1])) + 2, width)
    y_lo = max(int(np.floor(ys[0])) - 1, 0)
    y_hi = min(int(np.ceil(ys[-1])) + 2, height)
    sub = np.asarray(array[y_lo:y_hi, x_lo:x_hi], dtype=np.float32)
    if order == 0:
        ix = np.clip(np.rint(xs).astype(np.int64) - x_lo, 0, sub.shape[1] - 1)
        iy = np.clip(np.rint(ys).astype(np.int64) - y_lo, 0, sub.shape[0] - 1)
        return sub[np.ix_(iy, ix)]
    # Separable linear interpolation: rows then columns.
    return _interp_axis(_interp_axis(sub, ys - y_lo, 0), xs - x_lo, 1)


def _interp_axis(array: np.ndarray, coords: np.ndarray, axis: int) -> np.ndarray:
    """Linear interpolation of ``array`` along ``axis`` at ``coords`` (clamped)."""
    size = array.shape[axis]
    coords = np.clip(coords, 0.0, size - 1.0)
    lo = np.clip(np.floor(coords).astype(np.int64), 0, max(size - 2, 0))
    hi = np.minimum(lo + 1, size - 1)
    t = (coords - lo).astype(np.float32)
    a = np.take(array, lo, axis=axis)
    b = np.take(array, hi, axis=axis)
    shape = [1, 1]
    shape[axis] = -1
    t = t.reshape(shape)
    return (a * (1.0 - t) + b * t).astype(np.float32)


def sample_level_tiles(
    map_dir: Path, source_level: int, level: int, window: tuple[int, int, int, int]
) -> np.ndarray:
    """Bilinear sample of the cached tiles of ``source_level`` on a finer window.

    NaN where no tile of ``source_level`` exists (``source_level`` >= 1).
    """
    col0, row0, cols, rows = window
    shift = level - source_level
    factor = float(1 << shift)
    xs = (np.arange(col0, col0 + cols) + 0.5) / factor - 0.5
    ys = (np.arange(row0, row0 + rows) + 0.5) / factor - 0.5
    x_lo, x_hi = int(np.floor(xs[0])) - 1, int(np.ceil(xs[-1])) + 2
    y_lo, y_hi = int(np.floor(ys[0])) - 1, int(np.ceil(ys[-1])) + 2
    sub = np.full((y_hi - y_lo, x_hi - x_lo), np.nan, dtype=np.float32)
    for row in range(y_lo // TILE_PX, (y_hi - 1) // TILE_PX + 1):
        for col in range(x_lo // TILE_PX, (x_hi - 1) // TILE_PX + 1):
            path = tile_path(map_dir, TileKey(source_level, col, row))
            if not path.exists():
                continue
            tile = terrain.uint16_to_height(terrain.read_png16(path)).astype(np.float32)
            ty0, tx0 = row * TILE_PX - y_lo, col * TILE_PX - x_lo
            a0, b0 = max(ty0, 0), max(tx0, 0)
            a1, b1 = min(ty0 + TILE_PX, sub.shape[0]), min(tx0 + TILE_PX, sub.shape[1])
            if a1 > a0 and b1 > b0:
                sub[a0:a1, b0:b1] = tile[a0 - ty0 : a1 - ty0, b0 - tx0 : b1 - tx0]
    return _interp_axis(_interp_axis(sub, ys - y_lo, 0), xs - x_lo, 1)


def block_any(mask: np.ndarray, factor: int) -> np.ndarray:
    """``True`` for each ``factor`` x ``factor`` block containing a ``True``."""
    rows, cols = mask.shape[0] // factor, mask.shape[1] // factor
    return mask.reshape(rows, factor, cols, factor).any(axis=(1, 3))


# ------------------------------------------------------------------------------ RLE


def tiles_to_rle(tiles: Iterable[tuple[int, int]]) -> list[dict]:
    """Manifest ``tiles_rle`` of a set of ``(col, row)``: runs of columns per row."""
    by_row: dict[int, list[int]] = {}
    for col, row in set(tiles):
        by_row.setdefault(row, []).append(col)
    result = []
    for row in sorted(by_row):
        cols = sorted(by_row[row])
        runs: list[list[int]] = []
        for col in cols:
            if runs and runs[-1][0] + runs[-1][1] == col:
                runs[-1][1] += 1
            else:
                runs.append([col, 1])
        result.append({"row": row, "runs": runs})
    return result


def rle_to_tiles(rle: list[dict]) -> set[tuple[int, int]]:
    """Inverse of :func:`tiles_to_rle`."""
    return {
        (start + offset, entry["row"])
        for entry in rle
        for start, length in entry["runs"]
        for offset in range(length)
    }


# ------------------------------------------------------------------------- manifest


def _dump_line(value: object) -> str:
    """Compact one-line JSON in the manifest's style (``{ "key": value, ... }``)."""
    text = json.dumps(value, ensure_ascii=False)
    if isinstance(value, dict) and value:
        text = "{ " + text[1:-1] + " }"
    return text


def update_manifest_levels(
    path: Path, entries: dict[int, dict], cache: dict | None = None
) -> None:
    """Rewrite only the ``levels`` lines of ``entries`` (and the ``cache`` line).

    The manifest keeps its top-level keys indented by two spaces and each entry
    of ``levels`` on a single compact JSON line, so that lots baking different
    levels in parallel (ZG1: E1-E4, ZG3: E5-E7) only touch their own lines: every
    other line stays byte-identical. Missing levels are inserted in order;
    ``cache`` is written on one line before the closing brace.

    Args:
        path: The manifest (``data/map/relief_pyramid.json``).
        entries: Full level objects keyed by level number.
        cache: New value of the top-level ``cache`` key, or ``None`` to keep it.
    """
    lines = path.read_text(encoding="utf-8").split("\n")
    start = next(i for i, line in enumerate(lines) if line.strip() == '"levels": [')
    end = next(
        i for i in range(start + 1, len(lines)) if lines[i].strip() in ("]", "],")
    )
    existing: dict[int, int] = {}
    for index in range(start + 1, end):
        stripped = lines[index].strip()
        if stripped:
            existing[json.loads(stripped.rstrip(","))["level"]] = index
    indent = "    "
    if existing:
        sample = lines[next(iter(existing.values()))]
        indent = sample[: len(sample) - len(sample.lstrip())]
    for level, entry in entries.items():
        if level in existing:
            index = existing[level]
            comma = "," if lines[index].rstrip().endswith(",") else ""
            lines[index] = f"{indent}{_dump_line(entry)}{comma}"
    missing = sorted(level for level in entries if level not in existing)
    if missing:
        body = [lines[i].strip().rstrip(",") for i in range(start + 1, end)]
        body = [line for line in body if line]
        levels = {json.loads(line)["level"]: line for line in body}
        for level in missing:
            levels[level] = _dump_line(entries[level])
        ordered = [levels[level] for level in sorted(levels)]
        new_body = [
            f"{indent}{line}{',' if i < len(ordered) - 1 else ''}"
            for i, line in enumerate(ordered)
        ]
        # Keep untouched lines byte-identical except for the trailing comma.
        old = {
            json.loads(lines[i].strip().rstrip(","))["level"]: lines[i]
            for i in existing.values()
        }
        for i, level in enumerate(sorted(levels)):
            if level in old and level not in entries:
                text = old[level].rstrip().rstrip(",")
                new_body[i] = f"{text}{',' if i < len(ordered) - 1 else ''}"
        lines[start + 1 : end] = new_body
    if cache is not None:
        _set_top_level_line(lines, "cache", cache)
    text = "\n".join(lines)
    json.loads(text)
    path.write_text(text, encoding="utf-8")


def _set_top_level_line(lines: list[str], key: str, value: object) -> None:
    """Replace (or append) a top-level key written on one line, in place."""
    prefix = f'  "{key}":'
    for index, line in enumerate(lines):
        if line.startswith(prefix):
            # A multi-line value (hand-written) spans until its closing brace.
            last = index
            if not line.rstrip().rstrip(",").endswith(
                ("}", "]")
            ) or line.rstrip().endswith("{"):
                depth = 0
                for last in range(index, len(lines)):
                    depth += lines[last].count("{") + lines[last].count("[")
                    depth -= lines[last].count("}") + lines[last].count("]")
                    if depth == 0:
                        break
            comma = "," if lines[last].rstrip().endswith(",") else ""
            lines[index : last + 1] = [f"{prefix} {_dump_line(value)}{comma}"]
            return
    closing = max(i for i, line in enumerate(lines) if line.strip() == "}")
    previous = max(i for i in range(closing) if lines[i].strip())
    lines[previous] = lines[previous].rstrip() + ","
    lines.insert(closing, f"{prefix} {_dump_line(value)}")


def scan_level(map_dir: Path, level: int) -> set[tuple[int, int]]:
    """``(col, row)`` of the tiles of ``level`` present in the cache."""
    directory = map_dir / PYRAMID_DIR_NAME / f"E{level}"
    tiles = set()
    if directory.is_dir():
        for path in directory.glob("*.png"):
            col, row = path.stem.split("_")
            tiles.add((int(col), int(row)))
    return tiles


def cache_footprint(map_dir: Path) -> dict:
    """``cache`` block of the manifest: every level present on disk."""
    root = map_dir / PYRAMID_DIR_NAME
    total, count = 0, 0
    if root.is_dir():
        for path in root.glob("E*/*.png"):
            total += path.stat().st_size
            count += 1
    return {
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat(),
        "total_bytes": total,
        "tile_count": count,
    }


def set_manifest_bake_versions(map_dir: Path, versions: dict[str, int]) -> None:
    """Merge ``versions`` into the manifest's top-level ``bake_versions`` line."""
    path = map_dir / MANIFEST
    text = path.read_text(encoding="utf-8")
    current = json.loads(text).get(bake_stamp.MANIFEST_KEY) or {}
    merged = {**current, **versions}
    if merged == current:
        return
    lines = text.split("\n")
    _set_top_level_line(lines, bake_stamp.MANIFEST_KEY, dict(sorted(merged.items())))
    text = "\n".join(lines)
    json.loads(text)
    path.write_text(text, encoding="utf-8")


def refresh_manifest(
    map_dir: Path, levels: Iterable[int], overrides: dict[int, dict] | None = None
) -> None:
    """Rewrite the ``tiles_rle`` of ``levels`` from the cache, plus the cache line."""
    path = map_dir / MANIFEST
    manifest = json.loads(path.read_text(encoding="utf-8"))
    by_level = {entry["level"]: entry for entry in manifest["levels"]}
    entries = {}
    for level in levels:
        entry = dict(by_level.get(level, {}))
        entry.update((overrides or {}).get(level, {}))
        entry["tiles_rle"] = tiles_to_rle(scan_level(map_dir, level))
        entries[level] = entry
    update_manifest_levels(path, entries, cache_footprint(map_dir))


# --------------------------------------------------------------------- shared inputs


def root_origin_tiles(map_dir: Path = MAP_DIR) -> tuple[int, int]:
    """``root_origin_tiles`` of ``relief_pyramid.json`` (``(0, 0)`` if absent)."""
    path = map_dir / MANIFEST
    if not path.exists():
        return (0, 0)
    dx, dy = json.loads(path.read_text(encoding="utf-8")).get(
        "root_origin_tiles", [0, 0]
    )
    return int(dx), int(dy)


def map_bounds(map_dir: Path = MAP_DIR) -> tuple[float, float, float, float]:
    """EPSG:3035 bounds of the pyramid frame (see :data:`E0_SIZE_PX`).

    World frame (``root_origin_tiles`` ``[0, 0]``, ADR 0121): the ``map.json``
    bounds. Legacy frame: ``E0_TILES`` root tiles square, placed at
    ``root_origin_tiles`` inside the world; world units of that frame (CAFV
    points, tile addresses) are the world units minus ``root_origin_tiles × 256``.
    """
    metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    minx, miny, maxx, maxy = (float(v) for v in metadata["bounds_projected"])
    dx, dy = root_origin_tiles(map_dir)
    if (dx, dy) == (0, 0):
        return (minx, miny, maxx, maxy)
    mpp = float(metadata["meters_per_px"])
    unit_m = ROOT_TILE_UNITS * mpp
    x0 = minx + dx * unit_m
    y1 = maxy - dy * unit_m
    side = E0_TILES * unit_m
    return (x0, y1 - side, x0 + side, y1)


def require_world_frame(map_dir: Path, step: str) -> None:
    """Refuse a step whose outputs mix world and pyramid-frame units (OM2).

    ``fine_anchors.json``, ``towns_1340.json`` and the draped roads read world
    positions (settlements, hamlets, roads) and write world units while sampling
    the cache in its frame. Until the pyramid is recooked in the world frame
    (``root_origin_tiles`` = ``[0, 0]``), those files are migrated (+1280 y,
    ``tools/cent_ans_tools/geo/migrate_om2.py``) instead of regenerated.

    Raises:
        RuntimeError: ``root_origin_tiles`` is not ``(0, 0)``.
    """
    origin = root_origin_tiles(map_dir)
    if origin != (0, 0):
        raise RuntimeError(
            f"{step} : la pyramide est cuite dans l'ancien cadre (root_origin_tiles "
            f"{list(origin)}, ADR 0115) ; ses sorties en unités monde sont migrées, "
            "pas régénérées, tant qu'elle n'est pas recuite dans le cadre monde."
        )


def frame_fine_grid(map_dir: Path = MAP_DIR) -> MapGrid:
    """The E0 grid over the pyramid frame (8192² legacy, 14336 x 12288 world)."""
    return level_grid(map_bounds(map_dir), 0)


def e0_heights(map_dir: Path = MAP_DIR) -> np.ndarray:
    """The E0 tiles of the pyramid frame (metres, ``float32``)."""
    dx, dy = root_origin_tiles(map_dir)
    cols, rows_count = frame_tiles(map_bounds(map_dir))
    rows = [
        np.hstack(
            [
                terrain.read_png16(
                    map_dir
                    / relief.TILE_DIR
                    / relief.TILE_PATTERN.format(col=col + dx, row=row + dy)
                )
                for col in range(cols)
            ]
        )
        for row in range(rows_count)
    ]
    return terrain.uint16_to_height(np.vstack(rows)).astype(np.float32)


def boost_base(merged_m: np.ndarray, meters_per_px: float) -> np.ndarray:
    """The blurred altitude (σ 5 km) the ADR 0019 boost is relative to."""
    return ndimage.gaussian_filter(
        merged_m, relief_shade.BOOST_SIGMA_M / meters_per_px
    ).astype(np.float32)


def boost_with_base(height_m: np.ndarray, base_m: np.ndarray) -> np.ndarray:
    """ADR 0019 render boost of ``height_m`` given its blurred base.

    No land test beyond :func:`relief_shade.floor_valleys` (valley floors of
    land above ``MIN_LAND_M`` are not dug, SZ2): the coast is applied after.
    """
    local = np.clip(
        height_m - base_m, -relief_shade.BOOST_LIMIT_M, relief_shade.BOOST_LIMIT_M
    )
    fade_lo, fade_hi = relief_shade.BOOST_FADE_M
    t = np.clip((base_m - fade_lo) / (fade_hi - fade_lo), 0.0, 1.0)
    fade = 1.0 - t * t * (3.0 - 2.0 * t)
    boosted = height_m + relief_shade.BOOST_GAIN * local * fade
    return relief_shade.floor_valleys(boosted, height_m)


def bbox_mask_e0(
    grid0: MapGrid, bbox: tuple[float, float, float, float], step: int = 8
) -> np.ndarray:
    """E0-grid mask of the pixels inside a lon/lat box (sampled every ``step`` px)."""
    coarse_x = grid0.width_px // step
    coarse_y = grid0.height_px // step
    px, py = np.meshgrid(
        (np.arange(coarse_x) + 0.5) * step, (np.arange(coarse_y) + 0.5) * step
    )
    lon, lat = grid0.pixel_to_lonlat(px.ravel(), py.ravel())
    lon_min, lat_min, lon_max, lat_max = bbox
    inside = (
        (np.asarray(lon) >= lon_min)
        & (np.asarray(lon) <= lon_max)
        & (np.asarray(lat) >= lat_min)
        & (np.asarray(lat) <= lat_max)
    ).reshape(coarse_y, coarse_x)
    return np.repeat(np.repeat(inside, step, axis=0), step, axis=1)


def prepare_work(map_dir: Path = MAP_DIR, force: bool = False) -> dict[str, Path]:
    """Cache the E0-grid inputs shared by every worker (memory-mapped ``.npy``).

    - ``e0.npy``: E0 heights (m).
    - ``base.npy``: blurred Copernicus + ETOPO mosaic the E0 boost used.
    - ``coast.npy``: ``uint8`` 0 = water well inside E0's sea, 2 = land well
      inside E0's land, 1 = one-pixel band along E0's shore.
    """
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    paths = {name: WORK_DIR / f"{name}.npy" for name in ("e0", "base", "coast")}
    tiles_dir = map_dir / relief.TILE_DIR
    newest_tile = max(p.stat().st_mtime for p in tiles_dir.glob("*.png"))
    if force or not paths["e0"].exists() or paths["e0"].stat().st_mtime < newest_tile:
        e0 = e0_heights(map_dir)
        np.save(paths["e0"], e0)
        paths["coast"].unlink(missing_ok=True)
    else:
        e0 = np.load(paths["e0"])
    if force or not paths["coast"].exists():
        land = e0 >= relief_shade.MIN_LAND_M * 0.5
        sure_land = ndimage.binary_erosion(land, iterations=1, border_value=1)
        sure_water = ndimage.binary_erosion(~land, iterations=1, border_value=1)
        coast = np.ones(land.shape, dtype=np.uint8)
        coast[sure_land] = 2
        coast[sure_water] = 0
        np.save(paths["coast"], coast)
    if force or not paths["base"].exists():
        grid0 = frame_fine_grid(map_dir)
        names = copernicus.tiles_in_bbox(copernicus.tile_list())
        cop = relief_shade.copernicus_on_grid(
            grid0, names, cache=relief_shade.cache_path(grid0)
        )
        etopo = relief.resample_heights(
            grid0,
            download.etopo_tiles(download.etopo_tiles_for_grid(grid0)),
        )
        merged = copernicus.merge_with_etopo(cop, etopo)
        del cop, etopo
        np.save(paths["base"], boost_base(merged, grid0.meters_per_px))
    return paths


def candidate_tiles(
    map_dir: Path,
    level: int,
    bbox: tuple[float, float, float, float],
    exclude: Iterable[tuple[float, float, float, float]] = (),
) -> set[tuple[int, int]]:
    """Tiles of ``level`` whose footprint holds E0 land inside ``bbox``.

    Land inside one of the ``exclude`` boxes does not count.
    """
    e0 = np.load(WORK_DIR / "e0.npy", mmap_mode="r")
    grid0 = frame_fine_grid(map_dir)
    mask = (np.asarray(e0) >= relief_shade.MIN_LAND_M * 0.5) & bbox_mask_e0(grid0, bbox)
    for box in exclude:
        mask &= ~bbox_mask_e0(grid0, box)
    footprint = TILE_PX >> level
    rows, cols = np.nonzero(block_any(mask, footprint))
    return {(int(c), int(r)) for c, r in zip(cols, rows, strict=True)}


#: Manifest fields of E3-E4 (envelope of the core once reduced).
TIER2_MANIFEST = {
    "source": "Copernicus DEM GLO-30 corrigé (canopée, bâti, retenues) ; cœur réduit à la"
    " France, l'Angleterre et le pays de Galles, le Bénélux et la rive gauche du Rhin",
    "bbox_lonlat": [-6.0, 42.35, 8.0, 56.0],
}


# ------------------------------------------------------------------ worker plumbing

_SHARED: dict[str, object] = {}


def _init_worker(paths: dict[str, str], map_dir: str) -> None:
    """Open the shared inputs once per worker process (memory maps)."""
    for name, path in paths.items():
        _SHARED[name] = np.load(path, mmap_mode="r")
    _SHARED["map_dir"] = Path(map_dir)
    _SHARED["bounds"] = map_bounds(Path(map_dir))


def _write_tile(map_dir: Path, key: TileKey, heights_m: np.ndarray) -> int:
    """Encode and write one tile atomically; returns its size in bytes."""
    path = tile_path(map_dir, key)
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_name(f".{path.stem}.part")
    encoded = np.ascontiguousarray(terrain.height_to_uint16(heights_m))
    Image.fromarray(encoded).save(partial, format="PNG", compress_level=9)
    os.replace(partial, path)
    return path.stat().st_size


def apply_coast(
    height_m: np.ndarray, water: np.ndarray, sea_m: np.ndarray, fade_px: int = 0
) -> np.ndarray:
    """Land above :data:`relief_shade.MIN_LAND_M`, water at ``min(sea_m, 0)``.

    With ``fade_px`` > 0, the ``fade_px`` land pixels nearest the water ramp down
    towards the shore level and the water pixels nearest the land ramp up to 0 m,
    so that the source coast meets the water without a wall.
    """
    land_h = np.maximum(height_m, relief_shade.MIN_LAND_M)
    sea_h = np.minimum(sea_m, 0.0)
    if fade_px > 0 and water.any() and (~water).any():
        span = float(fade_px + 1)
        to_water = ndimage.distance_transform_edt(~water)
        to_land = ndimage.distance_transform_edt(water)
        land_t = np.clip(to_water / span, 0.0, 1.0)
        land_h = relief_shade.MIN_LAND_M + (land_h - relief_shade.MIN_LAND_M) * land_t
        sea_h = sea_h * np.clip(to_land / span, 0.0, 1.0)
    return np.where(water, sea_h, land_h).astype(np.float32)


# ----------------------------------------------------------------------------- tier 1


def tier1_unit(
    key: TileKey, write_levels: tuple[int, ...], e2_tiles: list
) -> list[tuple[int, int]]:
    """Bake one E1 tile and its E2 children from Copernicus GLO-90.

    Args:
        key: The E1 tile.
        write_levels: Subset of ``(1, 2)`` to write.
        e2_tiles: ``(col, row)`` of the E2 children to write (land only).

    Returns:
        ``(level, bytes)`` of every tile written.
    """
    map_dir: Path = _SHARED["map_dir"]  # type: ignore[assignment]
    bounds = _SHARED["bounds"]
    grid2 = level_grid(bounds, 2)  # type: ignore[arg-type]
    window2 = (key.col * 2 * TILE_PX, key.row * 2 * TILE_PX, 2 * TILE_PX, 2 * TILE_PX)
    raw = glo90_on_grid(grid2, window2)
    h2 = tier1_heights(raw, window2)
    written: list[tuple[int, int]] = []
    if 2 in write_levels:
        for col, row in e2_tiles:
            dy, dx = row - 2 * key.row, col - 2 * key.col
            block = h2[
                dy * TILE_PX : (dy + 1) * TILE_PX, dx * TILE_PX : (dx + 1) * TILE_PX
            ]
            written.append((2, _write_tile(map_dir, TileKey(2, col, row), block)))
    if 1 in write_levels:
        h1 = relief_shade.block_mean(h2, 2)
        window1 = tile_window(key)
        e0_bil = sample_e0_grid(_SHARED["e0"], 1, window1)  # type: ignore[arg-type]
        h1 = apply_coast(h1, e0_bil <= 0.0, e0_bil)
        written.append((1, _write_tile(map_dir, key, h1)))
    return written


def glo90_on_grid(grid: MapGrid, window: tuple[int, int, int, int]) -> np.ndarray:
    """Area average of the cached GLO-90 tiles on a grid window (NaN = no tile)."""
    from cent_ans_tools.geo import glo30

    lon_min, lat_min, lon_max, lat_max = glo30.window_lonlat(grid, window)
    paths = []
    for lat in range(int(np.floor(lat_min)), int(np.ceil(lat_max))):
        for lon in range(int(np.floor(lon_min)), int(np.ceil(lon_max))):
            path = copernicus.tile_path(copernicus.tile_name(lon, lat))
            if path.exists():
                paths.append(path)
    return glo30.mosaic_to_grid(paths, grid, window, GLO90_RES_DEG, Resampling.average)


def tier1_heights(
    raw_m: np.ndarray, window: tuple[int, int, int, int], level: int = 2
) -> np.ndarray:
    """Boosted, coast-enforced heights of a tier-1 window from area-averaged GLO-90.

    Where the source has no land value (sea, outside the tiles), the bilinear E0
    surface is kept as is (it is already boosted); the coast is E0's.
    """
    e0_bil = sample_e0_grid(_SHARED["e0"], level, window)  # type: ignore[arg-type]
    base = sample_e0_grid(_SHARED["base"], level, window)  # type: ignore[arg-type]
    valid = ~np.isnan(raw_m) & (np.abs(np.nan_to_num(raw_m)) > 0.01)
    heights = np.where(valid, boost_with_base(np.nan_to_num(raw_m), base), e0_bil)
    return apply_coast(heights, e0_bil <= 0.0, e0_bil)


# ------------------------------------------------------------------------------ build


def _run_units(
    function,
    jobs: list[tuple],
    paths: dict[str, Path],
    map_dir: Path,
    workers: int,
    label: str,
) -> list[tuple[int, int]]:
    """Run ``function(*job)`` in a process pool; returns ``(level, bytes)`` written."""
    written: list[tuple[int, int]] = []
    if not jobs:
        return written
    started = time.perf_counter()
    with ProcessPoolExecutor(
        max_workers=workers,
        initializer=_init_worker,
        initargs=({k: str(v) for k, v in paths.items()}, str(map_dir)),
    ) as pool:
        futures = [pool.submit(function, *job) for job in jobs]
        for done, future in enumerate(as_completed(futures), start=1):
            written.extend(future.result())
            if done % 50 == 0 or done == len(jobs):
                elapsed = time.perf_counter() - started
                print(f"  {label}: {done}/{len(jobs)} ({elapsed:.0f} s)", flush=True)
    return written


def build(
    levels: tuple[int, ...] = (1, 2, 3, 4),
    force: bool = False,
    workers: int | None = None,
    map_dir: Path = MAP_DIR,
    limit: int | None = None,
    stream: bool = True,
) -> PyramidResult:
    """Bake the requested levels (resuming: tiles on disk are kept unless ``force``).

    A whole tier whose cache stamp is not the current :data:`TIER_VERSIONS` is
    rebaked as with ``force``, and an interrupted forced bake resumes: only
    tiles older than its start are rebaked (:mod:`bake_stamp`).

    Args:
        levels: Levels among 1-4.
        force: Rewrite existing tiles.
        workers: Processes (default: every core).
        map_dir: ``data/map``.
        limit: Bake at most this many units per tier (tests, trials).
        stream: Tier 1: download the missing GLO-90 tiles block by block and
            delete them once baked (:func:`_run_tier1_streamed`).
    """
    started = time.perf_counter()
    workers = workers or os.cpu_count() or 1
    paths = prepare_work(map_dir)
    written: list[tuple[int, int]] = []
    skipped = 0
    pyramid_dir = map_dir / PYRAMID_DIR_NAME
    if world_frame.cache_origin(pyramid_dir) is None:
        world_frame.write_frame(pyramid_dir, root_origin_tiles(map_dir))
    tier1 = tuple(level for level in levels if level in TIER1_LEVELS)
    if tier1:
        since = _begin_tier(pyramid_dir, TIER1_LEVELS, tier1, force)
        jobs, skip = _tier1_jobs(map_dir, tier1, since)
        skipped += skip
        done, complete = _run_tier1_streamed(
            jobs[:limit], paths, map_dir, workers, stream
        )
        written += done
        refresh_manifest(map_dir, tier1, {level: TIER1_MANIFEST for level in tier1})
        if complete:
            _finish_tier(map_dir, TIER1_LEVELS, tier1, since, limit, len(jobs))
    tier2 = tuple(level for level in levels if level in TIER2_LEVELS)
    if tier2:
        from cent_ans_tools.geo import surface

        surface.prepare_sources(CORE_BBOX)
        paths = {**paths, **surface.prepare_reservoirs(map_dir, force)}
        since = _begin_tier(pyramid_dir, TIER2_LEVELS, tier2, force)
        jobs, skip = _tier2_jobs(map_dir, tier2, since)
        skipped += skip
        written += _run_units(
            surface.tier2_unit, jobs[:limit], paths, map_dir, workers, "E3-E4"
        )
        refresh_manifest(map_dir, tier2, {level: TIER2_MANIFEST for level in tier2})
        _finish_tier(map_dir, TIER2_LEVELS, tier2, since, limit, len(jobs))
    per_level: dict[int, int] = {}
    for level, _ in written:
        per_level[level] = per_level.get(level, 0) + 1
    return PyramidResult(
        tiles_written=len(written),
        total_bytes=sum(size for _, size in written),
        seconds=time.perf_counter() - started,
        per_level=per_level,
        skipped_units=skipped,
    )


def glo90_names_for(grid2: MapGrid, key: TileKey, available: set[str]) -> set[str]:
    """GLO-90 tiles of the bucket read by :func:`tier1_unit` for E1 tile ``key``."""
    from cent_ans_tools.geo import glo30

    window2 = (key.col * 2 * TILE_PX, key.row * 2 * TILE_PX, 2 * TILE_PX, 2 * TILE_PX)
    lon_min, lat_min, lon_max, lat_max = glo30.window_lonlat(grid2, window2)
    names = set()
    for lat in range(int(np.floor(lat_min)), int(np.ceil(lat_max))):
        for lon in range(int(np.floor(lon_min)), int(np.ceil(lon_max))):
            name = copernicus.tile_name(lon, lat)
            if name in available:
                names.add(name)
    return names


def _in_fine_bbox(name: str) -> bool:
    corner = copernicus.parse_tile_name(name)
    if corner is None:
        return False
    lon, lat = corner
    lon_min, lat_min, lon_max, lat_max = copernicus.FINE_BBOX
    return lon + 1 > lon_min and lon < lon_max and lat + 1 > lat_min and lat < lat_max


def _run_tier1_streamed(
    jobs: list[tuple],
    paths: dict[str, Path],
    map_dir: Path,
    workers: int,
    stream: bool,
) -> tuple[list[tuple[int, int]], bool]:
    """Bake tier-1 jobs block by block, fetching and freeing GLO-90 as it goes.

    Blocks of :data:`STREAM_BLOCK`² E1 tiles, in row order. Before a block: stop
    (resumable) if the disk has less than :data:`STREAM_MIN_FREE_BYTES` free,
    download its missing GLO-90 tiles; after it: delete the raw tiles no later
    block reads (outside ``copernicus.FINE_BBOX`` only).

    Returns:
        ``(level, bytes)`` written, and whether every job ran.
    """
    if not jobs:
        return [], True
    if not stream:
        return _run_units(tier1_unit, jobs, paths, map_dir, workers, "E1-E2"), True
    grid2 = level_grid(map_bounds(map_dir), 2)
    available = set()
    for line in copernicus.tile_list():
        corner = copernicus.parse_tile_name(line)
        if corner is not None:
            available.add(copernicus.tile_name(*corner))
    blocks: dict[tuple[int, int], list[tuple]] = {}
    for job in jobs:
        key = job[0]
        blocks.setdefault(
            (key.row // STREAM_BLOCK, key.col // STREAM_BLOCK), []
        ).append(job)
    order = sorted(blocks)
    needs = {
        block: set().union(
            *(glo90_names_for(grid2, j[0], available) for j in blocks[block])
        )
        for block in order
    }
    last_use: dict[str, int] = {}
    for index, block in enumerate(order):
        for name in needs[block]:
            last_use[name] = index
    written: list[tuple[int, int]] = []
    fetched_bytes = 0
    for index, block in enumerate(order):
        free = shutil.disk_usage(map_dir).free
        if free < STREAM_MIN_FREE_BYTES:
            print(
                f"  E1-E2 : arrêt, {free / 1024**3:.1f} Gio libres "
                f"(< {STREAM_MIN_FREE_BYTES / 1024**3:.0f}) ; relancer pour reprendre",
                flush=True,
            )
            return written, False
        missing = sorted(
            name for name in needs[block] if not copernicus.tile_path(name).exists()
        )
        if missing:
            fetched = copernicus.fetch_tiles(missing)
            fetched_bytes += sum(p.stat().st_size for p in fetched if p.exists())
        label = f"E1-E2 bloc {index + 1}/{len(order)}"
        written += _run_units(tier1_unit, blocks[block], paths, map_dir, workers, label)
        for name in needs[block]:
            path = copernicus.tile_path(name)
            if (
                last_use[name] == index
                and not _in_fine_bbox(name)
                and path.exists()
                and not path.is_symlink()
            ):
                path.unlink()
        print(
            f"  {label} : {len(blocks[block])} unités, {len(missing)} tuiles GLO-90 "
            f"téléchargées ({fetched_bytes / 1e9:.2f} Go en tout), "
            f"{shutil.disk_usage(map_dir).free / 1024**3:.1f} Gio libres",
            flush=True,
        )
    return written, True


def _begin_tier(
    pyramid_dir: Path,
    tier_levels: tuple[int, ...],
    levels: tuple[int, ...],
    force: bool,
) -> float | None:
    """Rebake threshold of a tier (see :func:`bake_stamp.begin`).

    Only a bake of the whole tier (both its levels) is stamped; a partial one
    (``--levels 3``) keeps the plain "missing tiles" rule, or rewrites all with
    ``force``.
    """
    if levels != tier_levels:
        return time.time() if force else None
    name = TIER_NAMES[tier_levels]
    return bake_stamp.begin(pyramid_dir, name, TIER_VERSIONS[name], force)


def _finish_tier(
    map_dir: Path,
    tier_levels: tuple[int, ...],
    levels: tuple[int, ...],
    since: float | None,
    limit: int | None,
    jobs: int,
) -> None:
    """Stamp a whole tier as baked by :data:`TIER_VERSIONS` once every job ran."""
    if levels != tier_levels or (limit is not None and limit < jobs):
        return
    name = TIER_NAMES[tier_levels]
    bake_stamp.finish(map_dir / PYRAMID_DIR_NAME, name, TIER_VERSIONS[name], since)
    set_manifest_bake_versions(map_dir, {name: TIER_VERSIONS[name]})


def _tier1_jobs(
    map_dir: Path, levels: tuple[int, ...], since: float | None
) -> tuple[list, int]:
    """One job per E1 tile holding land of the frame; done ones skipped."""
    e1 = candidate_tiles(map_dir, 1, TIER1_BBOX)
    e2 = candidate_tiles(map_dir, 2, TIER1_BBOX)
    jobs, skipped = [], 0
    for col, row in sorted(e1, key=lambda t: (t[1], t[0])):
        key = TileKey(1, col, row)
        children = [(c.col, c.row) for c in key.children() if (c.col, c.row) in e2]
        todo = []
        if 1 in levels and bake_stamp.needs_rebake(tile_path(map_dir, key), since):
            todo.append(1)
        if 2 in levels and any(
            bake_stamp.needs_rebake(tile_path(map_dir, TileKey(2, c, r)), since)
            for c, r in children
        ):
            todo.append(2)
        if todo:
            jobs.append((key, tuple(todo), children))
        else:
            skipped += 1
    return jobs, skipped


def _tier2_jobs(
    map_dir: Path, levels: tuple[int, ...], since: float | None
) -> tuple[list, int]:
    """One job per E2 footprint holding land of the core bbox; done ones skipped."""
    exclude = [box for _, box in CORE_EXCLUDE]
    e2 = candidate_tiles(map_dir, 2, CORE_BBOX, exclude)
    e3 = candidate_tiles(map_dir, 3, CORE_BBOX, exclude)
    e4 = candidate_tiles(map_dir, 4, CORE_BBOX, exclude)
    jobs, skipped = [], 0
    for col, row in sorted(e2, key=lambda t: (t[1], t[0])):
        key = TileKey(2, col, row)
        e3_children = [(c.col, c.row) for c in key.children() if (c.col, c.row) in e3]
        e4_children = [
            (g.col, g.row)
            for c in key.children()
            for g in c.children()
            if (g.col, g.row) in e4
        ]
        todo = []
        for level, tiles in ((3, e3_children), (4, e4_children)):
            if level in levels and any(
                bake_stamp.needs_rebake(tile_path(map_dir, TileKey(level, c, r)), since)
                for c, r in tiles
            ):
                todo.append(level)
        if todo:
            jobs.append((key, tuple(todo), e3_children, e4_children))
        else:
            skipped += 1
    return jobs, skipped
