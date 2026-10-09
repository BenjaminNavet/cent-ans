"""Building-region material table: catalogue ``regions`` -> ``materials`` of building_regions.json (TX T4)."""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from cent_ans_tools.texture_factory.generate import load_manifest

REGIONS_FILE = Path(__file__).resolve().parents[3] / "data" / "art" / "building_regions.json"


def region_materials(
    document: dict[str, Any], manifest: dict[str, Any] | None = None
) -> dict[str, dict[str, str]]:
    """``{region: {role: entry id}}``: per role the narrowest entry covering the region, then catalogue order.

    Only entries whose manifest status is ``ok`` (when a manifest is given) are used; a role
    covered by no entry of a region is left out (the game then keeps its default material).
    """
    best: dict[tuple[str, str], tuple[int, int, str]] = {}
    for order, entry in enumerate(document["entries"]):
        if manifest is not None and manifest.get(entry["id"], {}).get("status") != "ok":
            continue
        regions = entry.get("regions", [])
        for region in regions:
            key = (region, entry["role"])
            rank = (len(regions), order, entry["id"])
            if key not in best or rank < best[key]:
                best[key] = rank
    table: dict[str, dict[str, str]] = {}
    for (region, role), rank in sorted(best.items()):
        table.setdefault(region, {})[role] = rank[2]
    return table


_MATERIALS = re.compile(r', "materials": \{[^{}]*\}')


def write_regions(
    table: dict[str, dict[str, str]], path: Path = REGIONS_FILE
) -> list[str]:
    """Insert (or refresh) ``"materials"`` in each region line of ``building_regions.json``.

    The file keeps its hand-written layout (one region per line); returns the catalogue regions
    the file does not know (a mistake in the catalogue, to fix by hand).
    """
    lines = path.read_text(encoding="utf-8").splitlines()
    known = set(json.loads(path.read_text(encoding="utf-8"))["regions"])
    in_regions = False
    for index, line in enumerate(lines):
        if line.strip().startswith('"regions"'):
            in_regions = True
            continue
        match = re.match(r'^(\s*)"([a-z_]+)": \{"framed"', line)
        if not in_regions or not match or match.group(2) not in table:
            continue
        line = _MATERIALS.sub("", line)
        materials = json.dumps(table[match.group(2)], ensure_ascii=False, sort_keys=True)
        lines[index] = re.sub(
            r'("southern": (?:true|false))', rf'\1, "materials": {materials}', line, count=1
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return sorted(set(table) - known)


def refresh(document: dict[str, Any], path: Path = REGIONS_FILE) -> list[str]:
    """Rebuild the table from the catalogue and its manifest, then write it."""
    return write_regions(region_materials(document, load_manifest(document)), path)
