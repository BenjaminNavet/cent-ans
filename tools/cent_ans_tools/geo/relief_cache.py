"""Whole relief cache: check and rebuild in order (lot ZG7b, ADR 0036).

The fine relief of the campaign map lives outside git under ``data/map/pyramid/``
(about 2.9 GB): tiles E1-E7 listed by ``data/map/relief_pyramid.json``, fine river
tiles listed by ``data/map/rivers_fine.json`` and draped road tiles listed by
``data/map/fine_anchors.json`` (``roads``). Without it the game falls back to E0
and the close camera stops at about 7 units.

``cent-ans geo relief-all`` rebuilds everything that is missing, in dependency
order, each step resuming from what is already on disk:

1. ``geo pyramid --levels 1,2`` (tier 1, Copernicus GLO-90);
2. ``geo pyramid --levels 3,4`` (tier 2, corrected GLO-30);
3. ``geo detail-dem`` (tier 3, national DTMs on the detail zones; ``--force``
   when step 2 wrote tiles, since detail-dem folds its zones back into E4);
4. ``geo hydro-fine`` (fine rivers snapped on the finest relief);
5. ``geo anchors-fine`` (settlements, bridges and draped roads).

A later step runs whenever an earlier one wrote tiles (it reads them). A tier
baked by an older version of the code (stamp ``pyramid/bake.json`` older than the
manifest's ``bake_versions``, :mod:`bake_stamp`) is stale and rebaked like a
missing one. A cache left in another frame than the manifest's
(``root_origin_tiles``, ADR 0121: caches baked before OMR R7 are in the legacy
``[0, 5]`` frame) is first reframed in place (:mod:`world_frame`: renames, no
rebake). ``--check`` only lists what is missing or stale (exit code 1 if
anything is).
"""

from __future__ import annotations

import json
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from cent_ans_tools.geo import bake_stamp, download, world_frame

MAP_DIR = download.TOOLS_DIR.parent / "data" / "map"
RAW_DIR = download.RAW_DIR
MANIFEST = "relief_pyramid.json"
RIVERS_MANIFEST = "rivers_fine.json"
ANCHORS_MANIFEST = "fine_anchors.json"
MAX_LEVEL = 7
TIER1 = (1, 2)
TIER2 = (3, 4)
TIER3 = (5, 6, 7)
EXAMPLES = 5

#: Steps in dependency order: identifier, CLI command, what it produces.
STEPS: tuple[tuple[str, str, str], ...] = (
    ("tier1", "geo pyramid --levels 1,2", "E1-E2 (Copernicus GLO-90)"),
    ("tier2", "geo pyramid --levels 3,4", "E3-E4 (Copernicus GLO-30 corrigé)"),
    ("tier3", "geo detail-dem", "E5-E7 (MNT nationaux, zones de détail)"),
    ("hydro", "geo hydro-fine", "fleuves fins (pyramid/hydro_fine)"),
    ("anchors", "geo anchors-fine", "ancrages et routes drapées (pyramid/roads_fine)"),
)

#: Raw inputs (``tools/geo/raw/``) each step downloads when absent.
RAW_INPUTS: dict[str, tuple[str, ...]] = {
    "tier1": ("copernicus", "etopo2022"),
    "tier2": ("copernicus30", "worldcover"),
    "tier3": ("detail",),
    "hydro": ("hydro",),
    "anchors": (),
}


@dataclass
class LayerStatus:
    """Presence of one cached layer (a pyramid level or a fine vector layer)."""

    name: str
    expected: int
    present: int
    bytes_on_disk: int = 0
    missing_examples: list[str] = field(default_factory=list)

    @property
    def missing(self) -> int:
        """Listed tiles absent from the disk."""
        return self.expected - self.present

    @property
    def complete(self) -> bool:
        """Every listed tile is there (a layer listing nothing is never complete)."""
        return self.expected > 0 and self.missing == 0


@dataclass
class CacheReport:
    """State of the whole relief cache."""

    map_dir: Path
    pyramid_dir: Path
    levels: dict[int, LayerStatus]
    fine: dict[str, LayerStatus]
    raw_present: dict[str, bool]
    #: Tiers baked by an older bake version (``tier1`` … ``tier3``).
    stale: list[str] = field(default_factory=list)
    #: Root tiles from the manifest's frame to the cache's (``None``: same frame).
    frame_shift: tuple[int, int] | None = None

    @property
    def complete(self) -> bool:
        """Every level and fine layer is complete and no tier is stale."""
        return (
            all(s.complete for s in self.levels.values())
            and all(s.complete for s in self.fine.values())
            and not self.stale
            and self.frame_shift is None
        )

    @property
    def total_bytes(self) -> int:
        """Bytes of the cached tiles found."""
        return sum(s.bytes_on_disk for s in self.levels.values()) + sum(
            s.bytes_on_disk for s in self.fine.values()
        )

    def missing_steps(self) -> list[str]:
        """Steps whose own output is incomplete (before propagating dependencies)."""
        wanted = []
        for step, levels in (("tier1", TIER1), ("tier2", TIER2), ("tier3", TIER3)):
            if step in self.stale or any(
                not self.levels[level].complete for level in levels
            ):
                wanted.append(step)
        if not self.fine["rivers"].complete:
            wanted.append("hydro")
        if not self.fine["roads"].complete:
            wanted.append("anchors")
        return wanted

    def plan(self, force: bool = False) -> list[str]:
        """Steps to run, in order: missing ones and every step after the first."""
        order = [step for step, _, _ in STEPS]
        if force:
            return order
        missing = set(self.missing_steps())
        plan: list[str] = []
        for step in order:
            # detail-dem blends into E4; hydro reads the finest relief, anchors the
            # rivers: rerun them after any upstream step.
            upstream = (step == "tier3" and "tier2" in plan) or (
                bool(plan) and step in ("hydro", "anchors")
            )
            if step in missing or upstream:
                plan.append(step)
        return plan

    def lines(self) -> list[str]:
        """Human-readable report (French, like the rest of the CLI)."""
        out = [f"Cache de relief : {self.pyramid_dir}"]
        if self.frame_shift is not None:
            out.append(
                f"  Cache dans un autre cadre (décalage {list(self.frame_shift)} tuiles "
                "racines, ADR 0121) — « geo relief-all » le recadre sans recuire"
            )
        for level in range(1, MAX_LEVEL + 1):
            out.append(_layer_line(self.levels[level]))
        for layer in ("rivers", "roads"):
            out.append(_layer_line(self.fine[layer]))
        raw = ", ".join(
            f"{name} {'oui' if ok else 'non'}" for name, ok in self.raw_present.items()
        )
        if self.stale:
            names = ", ".join(self.stale)
            out.append(
                f"  Cuisson périmée (version du code plus récente) : {names} — "
                "relancer « geo relief-all »"
            )
        out.append(f"Bruts dans tools/geo/raw : {raw}")
        out.append(f"Total en cache : {self.total_bytes / 1e9:.2f} Go")
        return out


def _layer_line(status: LayerStatus) -> str:
    if status.expected == 0:
        return f"  {status.name:10s} jamais cuit (manifeste vide)"
    state = "complet" if status.complete else f"{status.missing} manquantes"
    line = (
        f"  {status.name:10s} {status.present:6d}/{status.expected:<6d} {state:>16s}"
        f"  {status.bytes_on_disk / 1e6:8.1f} Mo"
    )
    if status.missing_examples:
        line += "  ex. " + ", ".join(status.missing_examples)
    return line


def expand_rle(
    rows: list[dict], cols: int, rows_count: int | None = None
) -> list[tuple[int, int]]:
    """``(col, row)`` of the tiles of one manifest ``tiles_rle`` list."""
    rows_count = cols if rows_count is None else rows_count
    tiles = []
    for row_entry in rows:
        row = int(row_entry.get("row", -1))
        if not 0 <= row < rows_count:
            continue
        for start, length in row_entry.get("runs", []):
            tiles.extend(
                (col, row) for col in range(max(start, 0), min(start + length, cols))
            )
    return tiles


def _scan(name: str, paths: list[Path]) -> LayerStatus:
    status = LayerStatus(name=name, expected=len(paths), present=0)
    for path in paths:
        try:
            size = path.stat().st_size
        except OSError:
            if len(status.missing_examples) < EXAMPLES:
                status.missing_examples.append(path.name)
            continue
        status.present += 1
        status.bytes_on_disk += size
    return status


def _read_json(path: Path) -> dict:
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else {}


def _fine_layer(name: str, index: dict, root: Path) -> LayerStatus:
    directory = root / str(index.get("dir", ""))
    pattern = str(index.get("pattern", "E2/{col}_{row}.bin"))
    paths = [
        directory
        / pattern.replace("{col}", str(t.get("col", 0))).replace(
            "{row}", str(t.get("row", 0))
        )
        for t in index.get("tiles", [])
    ]
    return _scan(name, paths)


def check(map_dir: Path = MAP_DIR, raw_dir: Path = RAW_DIR) -> CacheReport:
    """Stat every tile the manifests list (about 15 000 files, well under a second)."""
    manifest = _read_json(map_dir / MANIFEST)
    pyramid_dir = map_dir / str(manifest.get("dir", "pyramid"))
    pattern = str(manifest.get("pattern", "E{level}/{col}_{row}.png"))
    entries = {int(e.get("level", 0)): e for e in manifest.get("levels", [])}
    frame_cols, frame_rows = _frame_tiles(map_dir, manifest)
    levels = {}
    for level in range(1, MAX_LEVEL + 1):
        tiles = expand_rle(
            entries.get(level, {}).get("tiles_rle", []),
            frame_cols << level,
            frame_rows << level,
        )
        paths = [
            pyramid_dir
            / pattern.replace("{level}", str(level))
            .replace("{col}", str(col))
            .replace("{row}", str(row))
            for col, row in tiles
        ]
        levels[level] = _scan(f"E{level}", paths)
    rivers = _read_json(map_dir / RIVERS_MANIFEST)
    roads = _read_json(map_dir / ANCHORS_MANIFEST).get("roads") or {}
    fine = {
        "rivers": _fine_layer("fleuves", rivers, map_dir),
        "roads": _fine_layer("routes", roads, map_dir),
    }
    raw_present = {
        name: (raw_dir / name).is_dir() and any((raw_dir / name).iterdir())
        for step in RAW_INPUTS.values()
        for name in step
    }
    expected = manifest.get(bake_stamp.MANIFEST_KEY) or {}
    stale = bake_stamp.stale_tiers(
        pyramid_dir, {str(k): int(v) for k, v in expected.items()}
    )
    origin = tuple(int(v) for v in manifest.get("root_origin_tiles", [0, 0]))
    cache_origin = world_frame.cache_origin(pyramid_dir)
    shift = None
    if cache_origin is not None and cache_origin != origin:
        shift = (cache_origin[0] - origin[0], cache_origin[1] - origin[1])
    return CacheReport(map_dir, pyramid_dir, levels, fine, raw_present, stale, shift)


def _frame_tiles(map_dir: Path, manifest: dict) -> tuple[int, int]:
    """Root tiles ``(cols, rows)`` of the cache frame (16 x 16 legacy, else world)."""
    if list(manifest.get("root_origin_tiles", [0, 0])) != [0, 0]:
        return 16, 16
    size = _read_json(map_dir / "map.json").get("size_px") or [4096, 4096]
    return int(size[0]) // 256, int(size[1]) // 256


# ----------------------------------------------------------------------- rebuild

#: A step runner: ``(map_dir, force, workers, log) -> tiles written`` (0 = nothing new).
StepRunner = Callable[[Path, bool, int | None, Callable[[str], None]], int]


def _run_tier1(
    map_dir: Path, force: bool, workers: int | None, log: Callable[[str], None]
) -> int:
    from cent_ans_tools.geo import pyramid

    result = pyramid.build(levels=TIER1, force=force, workers=workers, map_dir=map_dir)
    log(f"E1-E2 : {result.tiles_written} tuiles écrites ({result.seconds:.0f} s)")
    return result.tiles_written


def _run_tier2(
    map_dir: Path, force: bool, workers: int | None, log: Callable[[str], None]
) -> int:
    from cent_ans_tools.geo import pyramid

    result = pyramid.build(levels=TIER2, force=force, workers=workers, map_dir=map_dir)
    log(f"E3-E4 : {result.tiles_written} tuiles écrites ({result.seconds:.0f} s)")
    return result.tiles_written


def _run_tier3(
    map_dir: Path, force: bool, workers: int | None, log: Callable[[str], None]
) -> int:
    from cent_ans_tools.geo import detail_dem

    kwargs = {"workers": workers} if workers else {}
    result = detail_dem.build((), force=force, map_dir=map_dir, log=log, **kwargs)
    written = sum(result.tiles.values())
    log(f"E5-E7 : {written} tuiles ({result.seconds:.0f} s)")
    return written


def _run_hydro(
    map_dir: Path, force: bool, workers: int | None, log: Callable[[str], None]
) -> int:
    from cent_ans_tools.geo import hydro_fine

    result = hydro_fine.build(map_dir=map_dir, workers=workers, log=log)
    log(f"Fleuves fins : {result.tiles} tuiles ({result.seconds:.0f} s)")
    return result.tiles


def _run_anchors(
    map_dir: Path, force: bool, workers: int | None, log: Callable[[str], None]
) -> int:
    from cent_ans_tools.geo import fine_anchors

    result = fine_anchors.build(map_dir=map_dir, workers=workers, log=log)
    log(result.summary())
    return 1


RUNNERS: dict[str, StepRunner] = {
    "tier1": _run_tier1,
    "tier2": _run_tier2,
    "tier3": _run_tier3,
    "hydro": _run_hydro,
    "anchors": _run_anchors,
}


@dataclass
class RebuildResult:
    """Steps run and the cache state afterwards."""

    ran: list[str]
    report: CacheReport


def rebuild(
    map_dir: Path = MAP_DIR,
    force: bool = False,
    workers: int | None = None,
    log: Callable[[str], None] = print,
    runners: dict[str, StepRunner] | None = None,
    raw_dir: Path = RAW_DIR,
) -> RebuildResult:
    """Run the steps the cache needs, in order; resumable (each step skips done work).

    Args:
        map_dir: ``data/map``.
        force: Rebake every step from scratch.
        workers: Processes per step (default: the step's own).
        log: Progress sink.
        runners: Step implementations (tests); default :data:`RUNNERS`.
        raw_dir: ``tools/geo/raw`` (report only).
    """
    runners = runners or RUNNERS
    report = check(map_dir, raw_dir)
    if report.frame_shift is not None:
        log(f"== recadrage du cache (ADR 0121, décalage {list(report.frame_shift)})")
        world_frame.reframe_cache(
            report.pyramid_dir, report.pyramid_dir, report.frame_shift, log=log
        )
        world_frame.write_frame(
            report.pyramid_dir, world_frame.manifest_origin(map_dir)
        )
        report = check(map_dir, raw_dir)
    plan = report.plan(force)
    ran: list[str] = []
    tier2_wrote = False
    labels = {step: (command, what) for step, command, what in STEPS}
    for step in plan:
        command, what = labels[step]
        # detail-dem folds its zones into E4: redo them over freshly baked E3-E4 tiles.
        step_force = force or (step == "tier3" and tier2_wrote)
        suffix = (
            " --force" if step_force and step in ("tier1", "tier2", "tier3") else ""
        )
        log(f"== {command}{suffix} : {what}")
        written = runners[step](map_dir, step_force, workers, log)
        if step == "tier2" and written > 0:
            tier2_wrote = True
        ran.append(step)
    return RebuildResult(ran=ran, report=check(map_dir, raw_dir))
