"""Game data staged into an exported build (``tools/export_macos.sh``, lot ZG7b).

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

import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from cent_ans_tools.geo import relief_cache

REPO_DIR = Path(__file__).resolve().parents[2]
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
        resources_dir: ``Cent Ans.app/Contents/Resources`` (receives ``data/``).
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
    for entry in sorted(source.iterdir()):
        if entry.name == PYRAMID.name:
            continue
        if entry.is_dir():
            copy_tree(entry, target / entry.name)
        else:
            shutil.copy2(entry, target / entry.name)
