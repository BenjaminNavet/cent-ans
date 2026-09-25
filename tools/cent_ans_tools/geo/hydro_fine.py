"""Fine river network snapped on the relief pyramid (lot ZG5a, ADR 0036).

``cent-ans geo hydro-fine`` builds, from the national networks of
:mod:`cent_ans_tools.geo.hydro_sources`:

1. link tables per source, with Strahler orders (cached in
   ``tools/geo/raw/hydro/cache/links_<source>.npz``);
2. strokes (river from source to confluence), post-1340 canals removed
   (``data/map/historical_hydro_notes.json``), small streams dropped;
3. each stroke snapped onto the valley floor of the finest cached pyramid level
   (:mod:`cent_ans_tools.geo.valley_snap`), in parallel, one cached result per
   chunk (``cache/snap/``) so that an interrupted run resumes;
4. water level made monotonic downstream across the whole network, confluences
   joined, widths in metres (``data/map/river_widths.json`` anchors, Strahler
   classes, source width classes);
5. tiles ``data/map/pyramid/hydro_fine/E2/{col}_{row}.bin`` (format CAFV, see
   :mod:`cent_ans_tools.geo.fine_tiles`) and the versioned manifest
   ``data/map/rivers_fine.json``.

``rivers_render.json``, ``river_bed.png`` and ``crossings*.json`` are not touched.
"""

from __future__ import annotations

from pathlib import Path

from cent_ans_tools.geo import download, hydro_sources

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
NOTES_FILE = "historical_hydro_notes.json"
WIDTHS_FILE = "river_widths.json"
MANIFEST_FILE = "rivers_fine.json"
TILES_DIR = "pyramid/hydro_fine"


def links_cache(source: str) -> Path:
    """Cache file of the link table of ``source``."""
    return hydro_sources.CACHE_DIR / f"links_{source}.npz"


def prepare_topage(force: bool = False) -> Path:
    """BD TOPAGE links with Strahler orders (slow: ~3 M links, a few minutes)."""
    path = links_cache("topage")
    if path.exists() and not force:
        return path
    table = hydro_sources.read_topage(hydro_sources.ensure_topage())
    table.strahler = hydro_sources.compute_strahler(table)
    table.save(path)
    return path


def prepare_osor(force: bool = False) -> Path:
    """OS Open Rivers links with Strahler orders."""
    path = links_cache("osor")
    if path.exists() and not force:
        return path
    table = hydro_sources.read_osor(hydro_sources.ensure_osor())
    table.strahler = hydro_sources.compute_strahler(table)
    table.save(path)
    return path
