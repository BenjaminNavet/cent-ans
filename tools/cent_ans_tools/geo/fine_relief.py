"""Height sampler on the finest cached level of the relief pyramid (lot ZG5a).

Reads the tiles of ``data/map/pyramid/E{k}/`` (ADR 0036) and the E0 tiles of
``data/map/height/`` as a fallback, never writes them. Coordinates are EPSG:3035
metres; each point is sampled bilinearly on the finest level having a tile under
it (the pyramid is sparse: E5-E7 only in the detail zones, E3-E4 on the core).
Heights are the render heights of the pyramid (ADR 0019 boost included), i.e. the
relief the player sees: rivers and roads must sit on it.
"""

from __future__ import annotations

import json
import warnings
from collections import OrderedDict
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import terrain

TILE_PX = 512
E0_TILES = 16
E0_DIR = "height"
PYRAMID_DIR = "pyramid"
MAX_LEVEL = 7


class FineRelief:
    """Bilinear height sampler over the pyramid, finest level first.

    Args:
        map_dir: ``data/map``.
        bounds: ``bounds_projected`` of ``map.json``.
        max_level: Finest level to use (``7`` = everything).
        min_level: Coarsest level to use (``0`` = E0 fallback).
        cache_tiles: Decoded tiles kept in memory (LRU, 1 MB each).
    """

    def __init__(
        self,
        map_dir: Path,
        bounds: tuple[float, float, float, float],
        max_level: int = MAX_LEVEL,
        min_level: int = 0,
        cache_tiles: int = 192,
    ) -> None:
        """Scan the cache for the tiles present at each level."""
        self.map_dir = Path(map_dir)
        # E0 lives in the world grid (height/), the pyramid in its frame (OM2).
        self.origin = _root_origin_tiles(self.map_dir)
        self.bounds = tuple(float(v) for v in bounds)
        self.width_m = self.bounds[2] - self.bounds[0]
        self.max_level = max_level
        self.min_level = min_level
        self.cache_tiles = cache_tiles
        # Root tiles of the frame: 16 x 16 legacy, 28 x 24 world (ADR 0119).
        from cent_ans_tools.geo import pyramid  # noqa: PLC0415 - import cycle

        self.frame_cols, self.frame_rows = pyramid.frame_tiles(self.bounds)
        self._cache: OrderedDict[tuple[int, int, int], np.ndarray] = OrderedDict()
        self.present: dict[int, np.ndarray] = {}
        for level in range(min_level, max_level + 1):
            cols, rows = self.frame_cols << level, self.frame_rows << level
            grid = np.zeros((rows, cols), dtype=bool)
            for col, row in self._scan(level):
                if 0 <= col < cols and 0 <= row < rows:
                    grid[row, col] = True
            self.present[level] = grid

    # ------------------------------------------------------------------ tiles

    def _tile_path(self, level: int, col: int, row: int) -> Path:
        if level == 0:
            dx, dy = self.origin
            return self.map_dir / E0_DIR / f"h_{col + dx}_{row + dy}.png"
        return self.map_dir / PYRAMID_DIR / f"E{level}" / f"{col}_{row}.png"

    def _scan(self, level: int) -> list[tuple[int, int]]:
        if level == 0:
            directory, prefix = self.map_dir / E0_DIR, "h_"
        else:
            directory, prefix = self.map_dir / PYRAMID_DIR / f"E{level}", ""
        tiles = []
        if directory.is_dir():
            for path in directory.glob(f"{prefix}*.png"):
                parts = path.stem.removeprefix(prefix).split("_")
                if len(parts) == 2 and all(p.isdigit() for p in parts):
                    col, row = int(parts[0]), int(parts[1])
                    if level == 0:
                        col, row = col - self.origin[0], row - self.origin[1]
                    tiles.append((col, row))
        return tiles

    def tile(self, level: int, col: int, row: int) -> np.ndarray:
        """Decoded tile in metres (``float32`` 512²), LRU-cached."""
        key = (level, col, row)
        cached = self._cache.get(key)
        if cached is not None:
            self._cache.move_to_end(key)
            return cached
        encoded = terrain.read_png16(self._tile_path(level, col, row))
        heights = terrain.uint16_to_height(encoded).astype(np.float32)
        self._cache[key] = heights
        while len(self._cache) > self.cache_tiles:
            self._cache.popitem(last=False)
        return heights

    # --------------------------------------------------------------- geometry

    def pixel_m(self, level: int) -> float:
        """Pixel size of ``level`` in metres."""
        return self.width_m / ((self.frame_cols * TILE_PX) << level)

    def level_coords(
        self, x: np.ndarray, y: np.ndarray, level: int
    ) -> tuple[np.ndarray, np.ndarray]:
        """Continuous pixel coordinates of ``level`` (pixel ``i`` centred on ``i``)."""
        size = self.pixel_m(level)
        u = (np.asarray(x, dtype=np.float64) - self.bounds[0]) / size - 0.5
        v = (self.bounds[3] - np.asarray(y, dtype=np.float64)) / size - 0.5
        return u, v

    def finest_level(self, x: np.ndarray, y: np.ndarray) -> np.ndarray:
        """Finest level with a tile under each point (``-1`` if none)."""
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        result = np.full(x.shape, -1, dtype=np.int8)
        for level in range(self.min_level, self.max_level + 1):
            grid = self.present[level]
            if not grid.any():
                continue
            u, v = self.level_coords(x, y, level)
            col = np.floor((u + 0.5) / TILE_PX).astype(np.int64)
            row = np.floor((v + 0.5) / TILE_PX).astype(np.int64)
            rows, cols = grid.shape
            inside = (col >= 0) & (col < cols) & (row >= 0) & (row < rows)
            hit = np.zeros(x.shape, dtype=bool)
            hit[inside] = grid[row[inside], col[inside]]
            result[hit] = level
        return result

    # --------------------------------------------------------------- sampling

    def _gather(self, level: int, px: np.ndarray, py: np.ndarray) -> np.ndarray:
        """Pixel values at integer coordinates of ``level`` (NaN without a tile)."""
        out = np.full(px.shape, np.nan, dtype=np.float32)
        rows, cols = self.present[level].shape
        px = np.clip(px, 0, cols * TILE_PX - 1)
        py = np.clip(py, 0, rows * TILE_PX - 1)
        col, row = px // TILE_PX, py // TILE_PX
        key = row * cols + col
        for k in np.unique(key):
            r, c = int(k // cols), int(k % cols)
            if not self.present[level][r, c]:
                continue
            mask = key == k
            data = self.tile(level, c, r)
            out[mask] = data[py[mask] - r * TILE_PX, px[mask] - c * TILE_PX]
        return out

    def sample_level(self, x: np.ndarray, y: np.ndarray, level: int) -> np.ndarray:
        """Bilinear heights on one level (a missing neighbour tile is clamped)."""
        u, v = self.level_coords(x, y, level)
        u0 = np.floor(u).astype(np.int64)
        v0 = np.floor(v).astype(np.int64)
        tu = (u - u0).astype(np.float32)
        tv = (v - v0).astype(np.float32)
        h00 = self._gather(level, u0, v0)
        h10 = self._gather(level, u0 + 1, v0)
        h01 = self._gather(level, u0, v0 + 1)
        h11 = self._gather(level, u0 + 1, v0 + 1)
        # A neighbour outside the sparse level: reuse the available corners.
        corners = np.stack([h00, h10, h01, h11])
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", RuntimeWarning)  # all four corners missing
            fallback = np.nanmean(corners, axis=0)
        h00, h10, h01, h11 = (np.where(np.isfinite(h), h, fallback) for h in corners)
        top = h00 * (1 - tu) + h10 * tu
        bottom = h01 * (1 - tu) + h11 * tu
        return top * (1 - tv) + bottom * tv

    def sample(self, x: np.ndarray, y: np.ndarray) -> np.ndarray:
        """Heights in metres on the finest level under each point (NaN off the map)."""
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        shape = x.shape
        x, y = x.ravel(), y.ravel()
        levels = self.finest_level(x, y)
        out = np.full(x.shape, np.nan, dtype=np.float64)
        for level in np.unique(levels):
            if level < 0:
                continue
            mask = levels == level
            out[mask] = self.sample_level(x[mask], y[mask], int(level))
        return out.reshape(shape)


def _root_origin_tiles(map_dir: Path) -> tuple[int, int]:
    """``root_origin_tiles`` of ``relief_pyramid.json`` (``(0, 0)`` if absent, OM2)."""
    path = Path(map_dir) / "relief_pyramid.json"
    if not path.exists():
        return (0, 0)
    dx, dy = json.loads(path.read_text(encoding="utf-8")).get(
        "root_origin_tiles", [0, 0]
    )
    return int(dx), int(dy)
