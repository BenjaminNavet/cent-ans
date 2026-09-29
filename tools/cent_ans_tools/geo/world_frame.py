"""Move the relief cache into the world frame (lot OMR R7, ADR 0119).

Before OMR R7 the relief pyramid was baked in the 4096² map of the West and
kept in that frame after the world grew to 7168 x 6144 units (ADR 0115): the
manifest carried ``root_origin_tiles`` ``[0, 5]`` and the cache tile
``(k, col, row)`` was the world tile ``(col, row + 5·2^k)``. The East and South
could not be added to that square frame (their tiles would sit at negative rows
or beyond its 16 root tiles), so R7 moves the cache to the world frame
(``root_origin_tiles`` ``[0, 0]``):

- relief tiles ``E{k}/{col}_{row}.png`` are renamed (or hard-linked into another
  cache) to their world address, their content is unchanged;
- fine river and road tiles (CAFV, :mod:`fine_tiles`) are renamed and their
  vertices shifted by ``root_origin_tiles × 256`` world units;
- the versioned manifests (``relief_pyramid.json`` ``tiles_rle``,
  ``rivers_fine.json`` and ``fine_anchors.json`` tile indexes) are shifted the
  same way (:func:`shift_manifests`), once, in the repository;
- ``pyramid/frame.json`` records the frame of the cache, so that
  ``geo relief-all`` recognises a cache left in the old frame and reframes it in
  place instead of rebaking it (:func:`cache_origin`).

Only the standard library and numpy: :mod:`relief_cache` imports it.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import struct
from collections.abc import Callable
from pathlib import Path

import numpy as np

FRAME_FILE = "frame.json"
#: Frame of a cache without ``frame.json`` (every cache baked before R7).
LEGACY_ORIGIN = (0, 5)
ROOT_TILE_UNITS = 256
FINE_LAYERS = ("hydro_fine", "roads_fine")
FINE_LEVEL = 2
_HEADER = struct.Struct("<4sHHHHIIII")


# ----------------------------------------------------------------------- frame


def manifest_origin(map_dir: Path) -> tuple[int, int]:
    """``root_origin_tiles`` of ``relief_pyramid.json`` (``(0, 0)`` if absent)."""
    path = Path(map_dir) / "relief_pyramid.json"
    if not path.exists():
        return (0, 0)
    dx, dy = json.loads(path.read_text(encoding="utf-8")).get(
        "root_origin_tiles", [0, 0]
    )
    return int(dx), int(dy)


def _has_tiles(pyramid_dir: Path) -> bool:
    for level_dir in pyramid_dir.glob("E*"):
        if level_dir.is_dir() and any(level_dir.glob("*.png")):
            return True
    return False


def cache_origin(pyramid_dir: Path) -> tuple[int, int] | None:
    """Frame of a cache: ``frame.json``, :data:`LEGACY_ORIGIN` without it.

    ``None`` for an empty cache (nothing to reframe).
    """
    path = Path(pyramid_dir) / FRAME_FILE
    if path.exists():
        try:
            dx, dy = json.loads(path.read_text(encoding="utf-8"))["root_origin_tiles"]
            return int(dx), int(dy)
        except (OSError, ValueError, KeyError, TypeError):
            pass
    if not Path(pyramid_dir).is_dir() or not _has_tiles(Path(pyramid_dir)):
        return None
    return LEGACY_ORIGIN


def write_frame(pyramid_dir: Path, origin: tuple[int, int]) -> None:
    """Record the frame of a cache in ``frame.json``."""
    Path(pyramid_dir).mkdir(parents=True, exist_ok=True)
    (Path(pyramid_dir) / FRAME_FILE).write_text(
        json.dumps({"root_origin_tiles": list(origin)}) + "\n", encoding="utf-8"
    )


# ----------------------------------------------------------------------- tiles


def shifted_address(
    level: int, col: int, row: int, shift: tuple[int, int]
) -> tuple[int, int]:
    """Address at ``level`` of a tile moved by ``shift`` root tiles."""
    return col + (shift[0] << level), row + (shift[1] << level)


def shift_cafv(data: bytes, shift: tuple[int, int]) -> bytes:
    """A CAFV tile moved by ``shift`` root tiles (header address and vertices)."""
    magic, version, layer, level, reserved, col, row, n_lines, n_points = (
        _HEADER.unpack_from(data)
    )
    if magic != b"CAFV":
        raise ValueError("not a CAFV tile")
    col, row = shifted_address(level, col, row, shift)
    header = _HEADER.pack(
        magic, version, layer, level, reserved, col, row, n_lines, n_points
    )
    offset = _HEADER.size + 16 * n_lines
    xs = np.frombuffer(data, dtype="<f4", count=n_points, offset=offset).astype(
        np.float64
    )
    ys = np.frombuffer(
        data, dtype="<f4", count=n_points, offset=offset + 4 * n_points
    ).astype(np.float64)
    xs = (xs + shift[0] * ROOT_TILE_UNITS).astype("<f4")
    ys = (ys + shift[1] * ROOT_TILE_UNITS).astype("<f4")
    return b"".join(
        [
            header,
            data[_HEADER.size : offset],
            xs.tobytes(),
            ys.tobytes(),
            data[offset + 8 * n_points :],
        ]
    )


def _parse_stem(stem: str) -> tuple[int, int] | None:
    parts = stem.split("_")
    if len(parts) != 2 or not all(p.lstrip("-").isdigit() for p in parts):
        return None
    return int(parts[0]), int(parts[1])


def reframe_cache(
    src: Path,
    dst: Path,
    shift: tuple[int, int],
    link: bool = True,
    log: Callable[[str], None] = print,
) -> dict[str, int]:
    """Copy (hard links) or move a cache into a frame ``shift`` root tiles away.

    Args:
        src: Cache in the old frame (``data/map/pyramid``).
        dst: Destination cache; may be ``src`` (renamed in place).
        shift: Old origin minus new origin, in root tiles (``(0, 5)`` for R7).
        link: With ``dst != src``, hard-link relief tiles (no extra disk) instead
            of copying them.
        log: Progress sink.

    Returns:
        Files handled per kind.
    """
    src, dst = Path(src), Path(dst)
    in_place = src.resolve() == dst.resolve()
    counts = {"relief": 0, "cafv": 0}
    for level_dir in sorted(src.glob("E*")):
        if not level_dir.is_dir() or not level_dir.name[1:].isdigit():
            continue
        level = int(level_dir.name[1:])
        out_dir = dst / level_dir.name
        out_dir.mkdir(parents=True, exist_ok=True)
        tiles = []
        for path in level_dir.glob("*.png"):
            address = _parse_stem(path.stem)
            if address is not None:
                tiles.append((address, path))
        # In place, move the rows farthest along the shift first (no overwrite).
        tiles.sort(key=lambda t: (t[0][1], t[0][0]), reverse=shift[1] > 0)
        for (col, row), path in tiles:
            new_col, new_row = shifted_address(level, col, row, shift)
            target = out_dir / f"{new_col}_{new_row}.png"
            if in_place:
                os.replace(path, target)
            elif link:
                target.unlink(missing_ok=True)
                os.link(path, target)
            else:
                shutil.copy2(path, target)
            counts["relief"] += 1
        log(f"  {level_dir.name} : {len(tiles)} tuiles")
    for layer in FINE_LAYERS:
        layer_src = src / layer
        if not layer_src.is_dir():
            continue
        layer_dst = dst / layer
        tiles_dst = layer_dst / f"E{FINE_LEVEL}"
        tiles_dst.mkdir(parents=True, exist_ok=True)
        for extra in layer_src.glob("*.json"):
            if not in_place:
                shutil.copy2(extra, layer_dst / extra.name)
        tiles = []
        for path in (layer_src / f"E{FINE_LEVEL}").glob("*.bin"):
            address = _parse_stem(path.stem)
            if address is not None:
                tiles.append((address, path))
        tiles.sort(key=lambda t: (t[0][1], t[0][0]), reverse=shift[1] > 0)
        for (col, row), path in tiles:
            new_col, new_row = shifted_address(FINE_LEVEL, col, row, shift)
            data = shift_cafv(path.read_bytes(), shift)
            target = tiles_dst / f"{new_col}_{new_row}.bin"
            partial = target.with_name(f".{target.stem}.part")
            partial.write_bytes(data)
            if in_place:
                path.unlink()
            os.replace(partial, target)
            counts["cafv"] += 1
        log(f"  {layer} : {len(tiles)} tuiles")
    stamp = src / "bake.json"
    if stamp.exists() and not in_place:
        shutil.copy2(stamp, dst / "bake.json")
    return counts


# ------------------------------------------------------------------- manifests


def _shift_rle(rle: list[dict], level: int, shift: tuple[int, int]) -> list[dict]:
    dx, dy = shift[0] << level, shift[1] << level
    return [
        {"row": entry["row"] + dy, "runs": [[s + dx, n] for s, n in entry["runs"]]}
        for entry in rle
    ]


def shift_manifests(
    map_dir: Path, shift: tuple[int, int], pyramid_dir: Path | None = None
) -> None:
    """Shift the versioned manifests by ``shift`` root tiles (run once).

    ``relief_pyramid.json``: ``tiles_rle`` of every level and
    ``root_origin_tiles`` (minus ``shift``), rewritten line by line like
    :func:`pyramid.update_manifest_levels`. ``rivers_fine.json`` and
    ``fine_anchors.json`` (``roads``): tile addresses, and the SHA-1 prints from
    the reframed tiles of ``pyramid_dir`` when given.
    """
    map_dir = Path(map_dir)
    path = map_dir / "relief_pyramid.json"
    lines = path.read_text(encoding="utf-8").split("\n")
    origin = manifest_origin(map_dir)
    new_origin = [origin[0] - shift[0], origin[1] - shift[1]]
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith('"root_origin_tiles"'):
            comma = "," if stripped.endswith(",") else ""
            indent = line[: len(line) - len(line.lstrip())]
            lines[index] = (
                f'{indent}"root_origin_tiles": {json.dumps(new_origin)}{comma}'
            )
        elif stripped.startswith("{") and '"tiles_rle"' in stripped:
            comma = "," if stripped.endswith(",") else ""
            entry = json.loads(stripped.rstrip(","))
            entry["tiles_rle"] = _shift_rle(entry["tiles_rle"], entry["level"], shift)
            text = json.dumps(entry, ensure_ascii=False)
            indent = line[: len(line) - len(line.lstrip())]
            lines[index] = f"{indent}{{ {text[1:-1]} }}{comma}"
    text = "\n".join(lines)
    json.loads(text)
    path.write_text(text, encoding="utf-8")
    for name, key, layer in (
        ("rivers_fine.json", None, "hydro_fine"),
        ("fine_anchors.json", "roads", "roads_fine"),
    ):
        manifest_path = map_dir / name
        if not manifest_path.exists():
            continue
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
        index = data if key is None else data.get(key) or {}
        for tile in index.get("tiles", []):
            tile["col"], tile["row"] = shifted_address(
                FINE_LEVEL, tile["col"], tile["row"], shift
            )
            if pyramid_dir is not None:
                bin_path = (
                    Path(pyramid_dir)
                    / layer
                    / f"E{FINE_LEVEL}"
                    / f"{tile['col']}_{tile['row']}.bin"
                )
                if bin_path.exists():
                    tile["sha1"] = hashlib.sha1(bin_path.read_bytes()).hexdigest()[
                        : len(tile.get("sha1", "")) or 12
                    ]
        _write_like(manifest_path, data)


def _write_like(path: Path, data: dict) -> None:
    """Rewrite a generated JSON manifest with the layout it already has.

    ``hydro-fine`` writes ``indent=1``; ``anchors-fine`` writes compact JSON.
    """
    text = path.read_text(encoding="utf-8")
    if text.startswith("{\n"):
        out = json.dumps(data, ensure_ascii=False, indent=1)
    else:
        out = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    path.write_text(out + ("\n" if text.endswith("\n") else ""), encoding="utf-8")
