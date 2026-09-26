"""Bake versions of the relief cache tiers (lot SZ2, ADR 0036).

The relief cache (``data/map/pyramid/``, gitignored) carries a stamp file,
``pyramid/bake.json``, saying which bake version produced each tier and whether
that bake finished::

    {"tier1": {"version": 2, "started": 1790000000.0, "complete": true}, ...}

The versioned manifest ``data/map/relief_pyramid.json`` holds the versions the
code expects (top-level ``bake_versions``). A tier whose stamp is missing, older
or unfinished is *stale*: ``geo relief-all --check`` reports it and
``geo relief-all`` rebakes it. A forced bake records its start time first, so
that an interrupted one resumes by rebaking only the tiles older than it.

Only the standard library: :mod:`cent_ans_tools.geo.relief_cache` imports it.
"""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass
from pathlib import Path

STAMP_FILE = "bake.json"
MANIFEST_KEY = "bake_versions"
TIERS = ("tier1", "tier2", "tier3")


@dataclass(frozen=True)
class TierStamp:
    """Stamp of one tier in the cache."""

    version: int
    started: float
    complete: bool


def stamp_path(pyramid_dir: Path) -> Path:
    """``pyramid/bake.json``."""
    return pyramid_dir / STAMP_FILE


def read(pyramid_dir: Path) -> dict[str, TierStamp]:
    """Stamps found in the cache (empty when the file is absent or unreadable)."""
    path = stamp_path(pyramid_dir)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    stamps = {}
    for tier, entry in data.items() if isinstance(data, dict) else ():
        try:
            stamps[tier] = TierStamp(
                int(entry["version"]),
                float(entry.get("started", 0.0)),
                bool(entry.get("complete", False)),
            )
        except (KeyError, TypeError, ValueError):
            continue
    return stamps


def write(pyramid_dir: Path, tier: str, stamp: TierStamp) -> None:
    """Record ``stamp`` for ``tier`` (other tiers kept), atomically."""
    pyramid_dir.mkdir(parents=True, exist_ok=True)
    path = stamp_path(pyramid_dir)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            data = {}
    except (OSError, ValueError):
        data = {}
    data[tier] = {
        "version": stamp.version,
        "started": stamp.started,
        "complete": stamp.complete,
    }
    partial = path.with_name(f".{path.name}.part")
    partial.write_text(json.dumps(data, indent=1, sort_keys=True), encoding="utf-8")
    os.replace(partial, path)


def is_current(stamp: TierStamp | None, version: int) -> bool:
    """The tier was fully baked by ``version``."""
    return stamp is not None and stamp.version == version and stamp.complete


def begin(pyramid_dir: Path, tier: str, version: int, force: bool) -> float | None:
    """Start (or resume) the bake of ``tier``; returns the rebake threshold.

    Returns:
        ``None`` when the cache is current and not forced (only missing tiles
        are baked), otherwise a POSIX time: every tile older than it must be
        rebaked. A new forced or stale bake is stamped as started now; an
        unfinished bake of the same version resumes from its start time.
    """
    stamp = read(pyramid_dir).get(tier)
    if not force and is_current(stamp, version):
        return None
    if not force and stamp is not None and stamp.version == version:
        return stamp.started
    started = time.time()
    write(pyramid_dir, tier, TierStamp(version, started, False))
    return started


def finish(pyramid_dir: Path, tier: str, version: int, started: float | None) -> None:
    """Mark ``tier`` fully baked by ``version``."""
    previous = read(pyramid_dir).get(tier)
    begun = started if started is not None else (previous.started if previous else 0.0)
    write(pyramid_dir, tier, TierStamp(version, begun, True))


def needs_rebake(path: Path, threshold: float | None) -> bool:
    """A tile is missing, or older than the rebake ``threshold``."""
    try:
        mtime = path.stat().st_mtime
    except OSError:
        return True
    return threshold is not None and mtime < threshold


def stale_tiers(pyramid_dir: Path, expected: dict[str, int]) -> list[str]:
    """Tiers whose cache stamp is not the complete bake ``expected`` asks for."""
    stamps = read(pyramid_dir)
    return [
        tier
        for tier in TIERS
        if tier in expected and not is_current(stamps.get(tier), expected[tier])
    ]
