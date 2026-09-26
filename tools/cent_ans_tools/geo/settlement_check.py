"""Quick report on settlement files for the densification lots (DC2, ADR 0082).

Run from the repository root::

    uv run --project tools python -m cent_ans_tools.geo.settlement_check prov_anjou prov_maine

Without arguments every province is checked. The report lists, per province,
the number of settlements by kind, the entries whose ``lonlat`` falls outside
their province on ``province_ids.png`` (the game snaps them inside, but a
real place should not need it), pairs closer than ``MIN_DISTANCE_KM`` and the
entries missing ``sources`` or ``description``. It writes nothing.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from itertools import combinations

import numpy as np

from cent_ans_tools.geo import provinces as provinces_step
from cent_ans_tools.geo.settlements import (
    PROVINCES_DIR,
    SETTLEMENTS_DIR,
    load_labels,
    load_province_geometry,
    load_provinces,
)

MIN_DISTANCE_KM = 3.0
"""Two settlements closer than this read as one place on the map."""


def main(argv: list[str]) -> int:
    """Print the report; return 1 when a blocking problem is found."""
    grid = provinces_step.load_grid()
    labels = load_labels()
    provinces = load_provinces(PROVINCES_DIR)
    index_of = {pid: p["index"] for pid, p in load_province_geometry().items()}
    km_per_px = grid.meters_per_px / 1000.0
    wanted = set(argv) or set(provinces)
    seen_ids: Counter[str] = Counter()
    entries_by_province: dict[str, list[dict]] = {}
    for path in sorted(SETTLEMENTS_DIR.glob("prov_*.json")):
        entries = json.loads(path.read_text(encoding="utf-8"))
        seen_ids.update(entry["id"] for entry in entries)
        entries_by_province[path.stem] = entries
    blocking = False
    duplicates = [sid for sid, count in seen_ids.items() if count > 1]
    if duplicates:
        blocking = True
        print(f"IDS EN DOUBLE : {', '.join(sorted(duplicates))}")
    total = 0
    for province_id in sorted(wanted):
        entries = entries_by_province.get(province_id, [])
        total += len(entries)
        kinds = Counter(entry["kind"] for entry in entries)
        print(f"{province_id} : {len(entries)} ({dict(sorted(kinds.items()))})")
        if kinds["city"] != 1:
            blocking = True
            print("  ! il faut exactement une city")
        if not entries:
            continue
        lon = np.array([entry["lonlat"][0] for entry in entries])
        lat = np.array([entry["lonlat"][1] for entry in entries])
        xs, ys = grid.lonlat_to_pixel(lon, lat)
        height, width = labels.shape
        for entry, x, y in zip(entries, xs, ys, strict=True):
            row, col = int(y), int(x)
            inside = 0 <= row < height and 0 <= col < width
            if not inside or labels[row, col] != index_of.get(province_id):
                print(f"  hors province (sera recalée) : {entry['id']}")
            if not entry.get("sources") or not entry.get("description"):
                blocking = True
                print(f"  ! sources ou description manquantes : {entry['id']}")
        for (a, xa, ya), (b, xb, yb) in combinations(
            zip(entries, xs, ys, strict=True), 2
        ):
            distance_km = float(np.hypot(xa - xb, ya - yb)) * km_per_px
            if distance_km < MIN_DISTANCE_KM:
                print(f"  trop proches ({distance_km:.1f} km) : {a['id']} / {b['id']}")
    print(f"Total : {total} colonies dans {len(wanted)} provinces")
    return 1 if blocking else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
