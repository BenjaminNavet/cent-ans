"""Frame of the relief cache (``pyramid/frame.json``) versus the manifest's.

The manifest's ``relief_pyramid.json`` carries ``root_origin_tiles`` (``[0, 0]``
in the world frame). A cache records its own frame in ``pyramid/frame.json``; a
cache without it predates the world frame and is in :data:`LEGACY_ORIGIN`, so
:mod:`relief_cache` can tell it must be rebaked.

Only the standard library: :mod:`relief_cache` and :mod:`pyramid` import it.
"""

from __future__ import annotations

import json
from pathlib import Path

FRAME_FILE = "frame.json"
#: Frame of a cache without ``frame.json`` (every cache baked before the world frame).
LEGACY_ORIGIN = (0, 5)


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
