"""Bake pixel coordinates into ``data/map/map_landmarks_extra.json`` (lot DN ME6/ME7/ME9).

Sites and route waypoints are authored in longitude/latitude; the game reads map pixels
(EPSG:3035 grid of ``data/map/map.json``). Route waypoints are snapped to the nearest
settlement of ``settlements_px.json`` (``nodes``), whose road graph the planner follows.
"""

from __future__ import annotations

import json
import math
from pathlib import Path

from pyproj import Transformer

REPO = Path(__file__).resolve().parents[2]
DECOR_PATH = REPO / "data" / "map" / "map_landmarks_extra.json"
MAP_PATH = REPO / "data" / "map" / "map.json"
SETTLEMENTS_PATH = REPO / "data" / "map" / "settlements_px.json"


def lonlat_to_px(
    lon: float, lat: float, map_doc: dict, transformer: Transformer
) -> list[float]:
    """Project a longitude/latitude to map pixels (0.1 px precision)."""
    x, y = transformer.transform(lon, lat)
    west, _south, _east, north = map_doc["bounds_projected"]
    mpp = map_doc["meters_per_px"]
    return [round((x - west) / mpp, 1), round((north - y) / mpp, 1)]


def nearest_settlement(px: list[float], settlements: dict) -> str:
    """Id of the settlement whose pixel position is closest to ``px``."""
    return min(settlements, key=lambda k: math.dist(px, settlements[k]))


def bake(path: Path = DECOR_PATH) -> dict:
    """Fill ``px`` of every site and ``nodes`` of every route rule, rewrite the file."""
    document = json.loads(path.read_text(encoding="utf-8"))
    map_doc = json.loads(MAP_PATH.read_text(encoding="utf-8"))
    settlements = json.loads(SETTLEMENTS_PATH.read_text(encoding="utf-8"))
    transformer = Transformer.from_crs("EPSG:4326", map_doc["crs"], always_xy=True)
    for site in document.get("sites", []):
        if "lonlat" in site:
            site["px"] = lonlat_to_px(*site["lonlat"], map_doc, transformer)
    for rule in document.get("rules", []):
        if rule.get("mode") == "route" and "waypoints" in rule:
            nodes: list[str] = []
            for lon, lat in rule["waypoints"]:
                node = nearest_settlement(
                    lonlat_to_px(lon, lat, map_doc, transformer), settlements
                )
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
