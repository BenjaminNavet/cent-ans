"""Tiled binary polylines for the zoomable campaign map (lot ZG5a, ADR 0036).

Fine rivers and draped roads are cut along the tiles of a pyramid level (E2 by
default: 64 world units, ~46 km) so that the quadtree of the renderer (lot ZG5b)
loads only what it shows. One file per tile, little-endian, layout ``CAFV`` v1:

====================  =====  =============================================
field                 type   meaning
====================  =====  =============================================
magic                 4 B    ``b"CAFV"``
version               u16    1
layer                 u16    1 = rivers, 2 = roads
level                 u16    pyramid level of the tiling (2)
reserved              u16    0
col, row              u32 ×2 tile address at ``level``
line_count            u32    number of polylines
point_count           u32    number of vertices
lines                 u32 ×4 per line: ``feature`` (index in the manifest),
                             ``first`` vertex, ``count``, ``flags``
x, y                  f32    world units (map pixels 4096), ``point_count`` each
z                     f32    metres (water level / road surface, before the
                             vertical exaggeration of the renderer)
w                     f32    width in metres
====================  =====  =============================================

Arrays are stored one after the other (structure of arrays), so that Godot reads
each with ``FileAccess.get_buffer(4 * n).to_float32_array()``. A polyline crossing a
tile border is split there; both pieces share the border vertex.
"""

from __future__ import annotations

import hashlib
import struct
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

MAGIC = b"CAFV"
VERSION = 1
LAYER_RIVERS = 1
LAYER_ROADS = 2
TILE_LEVEL = 2
ROOT_TILE_UNITS = 256.0
HEADER = struct.Struct("<4sHHHHIIII")

#: Line flags (bit field), shared by rivers and roads.
FLAG_DIVAGATING = 1  # river: unembanked, braided or wandering course in 1340
FLAG_TIDAL = 2  # river: estuary / tidal reach
FLAG_INTERMITTENT = 4  # river: seasonal stream
FLAG_RECTIFIED = 8  # river: modern course of a reach rectified after 1340
FLAG_COARSE = 16  # geometry from Natural Earth (coarse source, snapped)
FLAG_WETLAND = 32  # river: crosses an undrained marsh in 1340; road: causeway
FLAG_MAIN_ROAD = 64  # road: main (Itiner-e "main") road
FLAG_COMPUTED_ROAD = 128  # road: computed fallback road (no historical trace)


def tile_units(level: int = TILE_LEVEL) -> float:
    """Side of a tile of ``level`` in world units."""
    return ROOT_TILE_UNITS / (1 << level)


@dataclass
class TileLines:
    """Polylines of one tile before encoding."""

    features: list[int] = field(default_factory=list)
    flags: list[int] = field(default_factory=list)
    xy: list[np.ndarray] = field(default_factory=list)
    z: list[np.ndarray] = field(default_factory=list)
    w: list[np.ndarray] = field(default_factory=list)

    def add(
        self, feature: int, flags: int, xy: np.ndarray, z: np.ndarray, w: np.ndarray
    ) -> None:
        """Append one polyline."""
        self.features.append(int(feature))
        self.flags.append(int(flags))
        self.xy.append(np.asarray(xy, dtype=np.float32))
        self.z.append(np.asarray(z, dtype=np.float32))
        self.w.append(np.asarray(w, dtype=np.float32))

    @property
    def point_count(self) -> int:
        """Total vertices."""
        return int(sum(len(a) for a in self.xy))


def split_by_tiles(
    xy: np.ndarray, attrs: np.ndarray, side: float
) -> list[tuple[int, int, np.ndarray, np.ndarray]]:
    """Cut a polyline along a square grid of ``side`` world units.

    Args:
        xy: ``(n, 2)`` world-unit vertices.
        attrs: ``(n, k)`` per-vertex attributes, interpolated at the cuts.
        side: Grid step.

    Returns:
        ``(col, row, xy, attrs)`` pieces in order; consecutive pieces share their
        border vertex.
    """
    xy = np.asarray(xy, dtype=np.float64)
    attrs = np.asarray(attrs, dtype=np.float64).reshape(len(xy), -1)
    if len(xy) < 2:
        return []
    out_xy = [xy[0]]
    out_attr = [attrs[0]]
    for i in range(len(xy) - 1):
        a, b = xy[i], xy[i + 1]
        ts = []
        for axis in (0, 1):
            lo, hi = sorted((a[axis], b[axis]))
            k0 = int(np.floor(lo / side)) + 1
            k1 = int(np.ceil(hi / side)) - 1
            span = b[axis] - a[axis]
            if span == 0.0:
                continue
            for k in range(k0, k1 + 1):
                t = (k * side - a[axis]) / span
                if 0.0 < t < 1.0:
                    ts.append(t)
        for t in sorted(set(ts)):
            out_xy.append(a + (b - a) * t)
            out_attr.append(attrs[i] + (attrs[i + 1] - attrs[i]) * t)
        out_xy.append(b)
        out_attr.append(attrs[i + 1])
    pts = np.asarray(out_xy)
    att = np.asarray(out_attr)
    mids = 0.5 * (pts[:-1] + pts[1:])
    cols = np.floor(mids[:, 0] / side).astype(np.int64)
    rows = np.floor(mids[:, 1] / side).astype(np.int64)
    pieces = []
    start = 0
    for seg in range(1, len(mids) + 1):
        if seg == len(mids) or cols[seg] != cols[start] or rows[seg] != rows[start]:
            pieces.append(
                (
                    int(cols[start]),
                    int(rows[start]),
                    pts[start : seg + 1],
                    att[start : seg + 1],
                )
            )
            start = seg
    return pieces


def encode(layer: int, level: int, col: int, row: int, lines: TileLines) -> bytes:
    """Bytes of one tile file (layout in the module docstring)."""
    count = lines.point_count
    table = np.zeros((len(lines.features), 4), dtype="<u4")
    first = 0
    for i, pts in enumerate(lines.xy):
        table[i] = (lines.features[i], first, len(pts), lines.flags[i])
        first += len(pts)
    xy = np.concatenate(lines.xy) if lines.xy else np.zeros((0, 2), np.float32)
    z = np.concatenate(lines.z) if lines.z else np.zeros(0, np.float32)
    w = np.concatenate(lines.w) if lines.w else np.zeros(0, np.float32)
    header = HEADER.pack(
        MAGIC, VERSION, layer, level, 0, col, row, len(lines.features), count
    )
    return b"".join(
        [
            header,
            table.tobytes(),
            xy[:, 0].astype("<f4").tobytes(),
            xy[:, 1].astype("<f4").tobytes(),
            z.astype("<f4").tobytes(),
            w.astype("<f4").tobytes(),
        ]
    )


def decode(data: bytes) -> dict:
    """Inverse of :func:`encode` (tests, previews)."""
    magic, version, layer, level, _, col, row, n_lines, n_points = HEADER.unpack_from(
        data
    )
    if magic != MAGIC or version != VERSION:
        raise ValueError("not a CAFV v1 tile")
    offset = HEADER.size
    table = np.frombuffer(data, dtype="<u4", count=4 * n_lines, offset=offset).reshape(
        -1, 4
    )
    offset += 16 * n_lines
    arrays = []
    for _ in range(4):
        arrays.append(np.frombuffer(data, dtype="<f4", count=n_points, offset=offset))
        offset += 4 * n_points
    if offset != len(data):
        raise ValueError("trailing bytes in CAFV tile")
    x, y, z, w = arrays
    lines = []
    for feature, first, count, flags in table:
        sl = slice(int(first), int(first + count))
        lines.append(
            {
                "feature": int(feature),
                "flags": int(flags),
                "xy": np.column_stack([x[sl], y[sl]]),
                "z": z[sl],
                "w": w[sl],
            }
        )
    return {"layer": layer, "level": level, "col": col, "row": row, "lines": lines}


def write_tiles(
    directory: Path, layer: int, level: int, tiles: dict[tuple[int, int], TileLines]
) -> list[dict]:
    """Write every tile (and remove stale ones); returns the manifest index."""
    directory.mkdir(parents=True, exist_ok=True)
    wanted = {f"{c}_{r}.bin" for c, r in tiles}
    for stale in directory.glob("*.bin"):
        if stale.name not in wanted:
            stale.unlink()
    index = []
    for (col, row), lines in sorted(tiles.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        data = encode(layer, level, col, row, lines)
        path = directory / f"{col}_{row}.bin"
        if not path.exists() or path.read_bytes() != data:
            tmp = path.with_suffix(".tmp")
            tmp.write_bytes(data)
            tmp.replace(path)
        index.append(
            {
                "col": col,
                "row": row,
                "lines": len(lines.features),
                "points": lines.point_count,
                "bytes": len(data),
                "sha1": hashlib.sha1(data).hexdigest()[:12],
            }
        )
    return index
