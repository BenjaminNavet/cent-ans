"""Bake pixel coordinates into ``data/map/map_landmarks_extra.json`` (lot DN ME6/ME7/ME9).

Sites and route waypoints are authored in longitude/latitude; the game reads map pixels
(campaign grid of ``data/map/map.json``: EPSG:3035 or composed, through ``MapGrid``). Route waypoints are snapped to the nearest
settlement of ``settlements_px.json`` (``nodes``), whose road graph the planner follows.
"""

from __future__ import annotations

import json
import math
from pathlib import Path

from cent_ans_tools.geo.project import MapGrid, grid_from_metadata
from cent_ans_tools.paths import REPO_DIR as REPO

DECOR_PATH = REPO / "data" / "map" / "map_landmarks_extra.json"
MAP_PATH = REPO / "data" / "map" / "map.json"
SETTLEMENTS_PATH = REPO / "data" / "map" / "settlements_px.json"


def lonlat_to_px(lon: float, lat: float, grid: MapGrid) -> list[float]:
    """Project a longitude/latitude to campaign-map pixels (0.1 px precision)."""
    x, y = grid.lonlat_to_pixel(lon, lat)
    return [round(float(x), 1), round(float(y), 1)]


def nearest_settlement(px: list[float], settlements: dict) -> str:
    """Id of the settlement whose pixel position is closest to ``px``."""
    return min(settlements, key=lambda k: math.dist(px, settlements[k]))


def bake(path: Path = DECOR_PATH) -> dict:
    """Fill ``px`` of every site and ``nodes`` of every route rule, rewrite the file."""
    document = json.loads(path.read_text(encoding="utf-8"))
    map_doc = json.loads(MAP_PATH.read_text(encoding="utf-8"))
    settlements = json.loads(SETTLEMENTS_PATH.read_text(encoding="utf-8"))
    grid = grid_from_metadata(map_doc)
    for site in document.get("sites", []):
        if "lonlat" in site:
            site["px"] = lonlat_to_px(*site["lonlat"], grid)
    for rule in document.get("rules", []):
        if rule.get("mode") == "route" and "waypoints" in rule:
            nodes: list[str] = []
            for lon, lat in rule["waypoints"]:
                node = nearest_settlement(lonlat_to_px(lon, lat, grid), settlements)
                if not nodes or nodes[-1] != node:
                    nodes.append(node)
            rule["nodes"] = nodes
    path.write_text(
        json.dumps(document, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return document


if __name__ == "__main__":
    result = bake()
    print(
        f"{len(result.get('sites', []))} sites, {len(result.get('rules', []))} rules baked"
    )
