"""Game data staged into an exported build (``tools/export_macos.sh``, lot ZG7b).

The Windows export (``tools/export_windows.sh``, ADR 0087) uses the same staging
with the folder of ``Cent Ans.exe`` as ``resources_dir`` and ``external_parent``.

``data/`` is read at runtime from absolute paths (``MapPaths``), never from the
Godot ``.pck``: the export copies it next to the executable. The fine relief
cache ``data/map/pyramid/`` (about 2.9 GB, ~15 000 files, ADR 0036) is placed
according to ``relief``:

- ``bundle`` (default): inside the app, ``Contents/Resources/data/map/pyramid``;
  one self-contained download, survives macOS app translocation.
- ``external``: in a ``Cent Ans relief/pyramid`` folder next to the app (split
  download; ``MapPaths.relief_root_for`` finds it there).
- ``none``: left out (light build; the game shows its "relief rapproché limité"
  notice and stops the close camera at about 7 units).

Symbolic links (agent worktrees link ``data/map/pyramid`` to the main checkout)
are followed. On APFS the copy uses ``cp -c`` clones: instantaneous and no extra
disk space until a file changes.
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from cent_ans_tools.geo import relief_cache
from cent_ans_tools.paths import REPO_DIR

RELIEF_MODES = ("bundle", "external", "none")
RELIEF_SIBLING_DIR = "Cent Ans relief"
EXCLUDED_TOP = ("schemas",)
PYRAMID = Path("map") / "pyramid"


@dataclass
class StageResult:
    """Where the data and the relief went, and the relief cache state."""

    data_dir: Path
    relief_dir: Path | None
    data_bytes: int
    relief_bytes: int
    report: relief_cache.CacheReport


def unused_relief_shade_pngs(map_dir: Path) -> set[str]:
    """Names of the ``relief_shade_<band>.png`` files the exported game never reads.

    The terrain loads the BC5 GPU copy (``relief_shade_bc5_<part>.bin``, ADR 0118) first and
    falls back on the PNG bands only when it is missing (``relief_landcover.gd``): when every
    BC5 part listed in ``map.json`` is present, the bands (about 130 MB) are dead weight.
    Returns an empty set when the BC5 copy is absent or incomplete (the PNGs then stay).
    """
    try:
        shade = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))[
            "relief_shade"
        ]
        bc5 = shade["bc5"]
        pattern = bc5["pattern"]
        parts = len(bc5["part_bytes"])
    except (OSError, ValueError, KeyError, TypeError):
        return set()
    if not parts or not all(
        (map_dir / pattern.format(part=part)).is_file() for part in range(parts)
    ):
        return set()
    band_pattern = re.compile(r"relief_shade(_\d+)?\.png")
    return {p.name for p in map_dir.iterdir() if band_pattern.fullmatch(p.name)}


def _tree_bytes(path: Path) -> int:
    return sum(p.stat().st_size for p in path.rglob("*") if p.is_file())


def copy_tree(source: Path, target: Path) -> None:
    """Copy ``source`` to ``target`` following links; APFS clones when possible."""
    source = source.resolve()
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        shutil.rmtree(target)
    if sys.platform == "darwin":
        done = subprocess.run(
            ["cp", "-RLc", str(source), str(target)], capture_output=True, check=False
        )
        if done.returncode == 0:
            return
        shutil.rmtree(target, ignore_errors=True)
    shutil.copytree(source, target, symlinks=False)


def stage(
    resources_dir: Path,
    relief: str = "bundle",
    repo_dir: Path = REPO_DIR,
    external_parent: Path | None = None,
) -> StageResult:
    """Copy ``data/`` (without schemas) and the relief cache into an export.

    Args:
        resources_dir: ``Cent Ans.app/Contents/Resources`` (macOS) or the folder
            of ``Cent Ans.exe`` (Windows); receives ``data/``.
        relief: One of :data:`RELIEF_MODES`.
        repo_dir: Repository root (tests use a fake one).
        external_parent: Folder receiving ``Cent Ans relief/`` in ``external`` mode
            (default: the folder holding the ``.app``).
    """
    if relief not in RELIEF_MODES:
        raise ValueError(f"relief : {relief!r} (attendu : {', '.join(RELIEF_MODES)})")
    source = repo_dir / "data"
    target = resources_dir / "data"
    if target.exists():
        shutil.rmtree(target)
    target.mkdir(parents=True)
    for entry in sorted(source.iterdir()):
        if entry.name in EXCLUDED_TOP:
            continue
        if entry.is_dir():
            if entry.name == "map":
                _copy_map(entry, target / "map")
            else:
                copy_tree(entry, target / entry.name)
        else:
            shutil.copy2(entry, target / entry.name)
    data_bytes = _tree_bytes(target)
    report = relief_cache.check(source / "map")
    relief_dir: Path | None = None
    pyramid = source / PYRAMID
    if relief != "none" and pyramid.exists():
        if relief == "bundle":
            relief_dir = target / PYRAMID
        else:
            parent = external_parent or resources_dir.parents[1].parent
            relief_dir = parent / RELIEF_SIBLING_DIR / "pyramid"
        copy_tree(pyramid, relief_dir)
    relief_bytes = _tree_bytes(relief_dir) if relief_dir else 0
    return StageResult(target, relief_dir, data_bytes, relief_bytes, report)


def _copy_map(source: Path, target: Path) -> None:
    """``data/map`` without ``pyramid/`` (placed separately by :func:`stage`)."""
    target.mkdir(parents=True)
    skipped = unused_relief_shade_pngs(source)
    for entry in sorted(source.iterdir()):
        if entry.name == PYRAMID.name or entry.name in skipped:
            continue
        if entry.is_dir():
            copy_tree(entry, target / entry.name)
        else:
            shutil.copy2(entry, target / entry.name)
