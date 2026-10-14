"""Grands fleuves de vallée sèche (Nil, Euphrate, Tigre) ajoutés à ``rivers.geojson`` (lot CC CS2).

La carte composée ne garde que 45 fleuves du réel (``compile.build_rivers``) : ni le Nil ni les
fleuves de Mésopotamie n'y figurent, et l'Égypte n'avait qu'une bande verte sur un désert. Les tracés
d'auteur de ``data/map/valley_courses.json`` (points lon / lat réels, Natural Earth 10 m pour le tracé
dense ; deltas et amonts à la main) sont projetés par le cartogramme, lissés (Chaikin), calés sur la
mer à l'embouchure puis ajoutés comme fleuves ordinaires (``source: "valley"``) : l'eau, le lit
creusé (``geo rivers-render``) et le creusement du relief (``compose-relief-real``) les traitent comme
les autres. Rejouable : les entrées ``valley`` précédentes sont remplacées.

    uv run --project tools cent-ans compose valley-rivers
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image
from shapely.geometry import LineString

from cent_ans_tools.geo import download
from cent_ans_tools.geo.provinces import load_grid
from cent_ans_tools.geo.river_render import chaikin

MAP_DIR = download.TOOLS_DIR.parent / "data" / "map"
COURSES_FILE = "valley_courses.json"
SOURCE = "valley"
CHAIKIN_ROUNDS = 3
SIMPLIFY_PX = 1.6  # efface le crénelage de l'ancienne grille (Natural Earth reprojeté)
EDGE_MARGIN_PX = (
    60.0  # le cartogramme écrase la marge : pas de tracé dans les 60 derniers px
)
SEA_REACH_PX = 14.0  # marche maximale vers la mer au-delà de la dernière pointe


def project_course(course: dict, grid: object) -> np.ndarray:
    """Tracé en pixels carte (points hors de la déformation retirés)."""
    lon = np.array([p[0] for p in course["lonlat"]], dtype=np.float64)
    lat = np.array([p[1] for p in course["lonlat"]], dtype=np.float64)
    px, py = (
        np.asarray(axis, dtype=np.float64) for axis in grid.lonlat_to_pixel(lon, lat)
    )
    keep = np.isfinite(px + py)
    return np.stack([px[keep], py[keep]], axis=1)


def _is_land(land: np.ndarray, point: np.ndarray) -> bool:
    x = int(np.clip(round(point[0]), 0, land.shape[1] - 1))
    y = int(np.clip(round(point[1]), 0, land.shape[0] - 1))
    return bool(land[y, x])


def trim_to_land(
    points: np.ndarray, land: np.ndarray, margin: float = EDGE_MARGIN_PX
) -> np.ndarray:
    """Plus longue suite de points hors de la marge du bord, sans extrémité en mer (un lac au milieu reste)."""
    height, width = land.shape
    inside = np.array(
        [
            margin <= p[0] <= width - 1 - margin
            and margin <= p[1] <= height - 1 - margin
            for p in points
        ]
    )
    best, start = (0, 0), None
    for index, flag in enumerate([*inside, False]):
        if flag and start is None:
            start = index
        elif not flag and start is not None:
            if index - start > best[1] - best[0]:
                best = (start, index)
            start = None
    run = points[best[0] : best[1]]
    on_land = [_is_land(land, p) for p in run]
    if not any(on_land):
        return run[:0]
    first = on_land.index(True)
    last = len(on_land) - 1 - on_land[::-1].index(True)
    return run[first : last + 1]


def reach_sea(
    points: np.ndarray, land: np.ndarray, reach: float = SEA_REACH_PX
) -> np.ndarray:
    """Termine le tracé sur la côte : coupé à la première case de mer, ou prolongé vers elle."""
    for index in range(len(points)):
        if not _is_land(land, points[index]):
            if index == 0:
                return points
            return np.vstack([points[:index], points[index : index + 1]])
    direction = points[-1] - points[-2]
    norm = float(np.hypot(*direction))
    if norm < 1e-6:
        return points
    direction = direction / norm
    end = points[-1]
    for step in range(1, int(reach) + 1):
        candidate = end + direction * step
        if not _is_land(land, candidate):
            return np.vstack([points, candidate])
    return points


def build_features(map_dir: Path = MAP_DIR) -> list[dict]:
    """Fleuves de vallée en pixels composés (liste de features GeoJSON)."""
    grid = load_grid(map_dir)
    document = json.loads((map_dir / COURSES_FILE).read_text(encoding="utf-8"))
    with Image.open(map_dir / "land_mask.png") as image:
        land = np.asarray(image) > 0
    features = []
    for course in document["courses"]:
        points = project_course(course, grid)
        if len(points) < 2:
            continue
        if course.get("clip_land", course["name"] in ("Euphrates", "Tigris")):
            points = trim_to_land(points, land)
            if len(points) < 2:
                continue
        points = np.asarray(LineString(points).simplify(SIMPLIFY_PX).coords)
        points = chaikin(points, CHAIKIN_ROUNDS)
        if course.get("to_sea", course["name"].startswith("Nile (")):
            points = reach_sea(points, land)
        points = np.clip(points, [0, 0], [land.shape[1] - 1, land.shape[0] - 1])
        length_px = float(np.hypot(*np.diff(points, axis=0).T).sum())
        features.append(
            {
                "name": course["name"],
                "scalerank": int(course.get("scalerank", 4)),
                "featurecla": "River",
                "source": SOURCE,
                "width_px": float(course.get("width_px", 2.4)),
                "length_km": round(length_px * float(grid.meters_per_px) / 1000.0, 1),
                "coords": points,
            }
        )
    return features


def _feature_text(feature: dict) -> str:
    props = {k: v for k, v in feature.items() if k != "coords"}
    coords = ", ".join(
        f"[ {x:.1f}, {y:.1f} ]" for x, y in np.asarray(feature["coords"]).tolist()
    )
    return (
        '{ "type": "Feature", "properties": '
        + json.dumps(props, ensure_ascii=False)
        + ', "geometry": { "type": "LineString", "coordinates": [ '
        + coords
        + " ] } }"
    )


def apply(map_dir: Path = MAP_DIR) -> list[dict]:
    """Remplace les entrées ``valley`` de ``rivers.geojson`` ; renvoie les fleuves écrits."""
    path = map_dir / "rivers.geojson"
    document = json.loads(path.read_text(encoding="utf-8"))
    kept = [
        feature
        for feature in document["features"]
        if feature.get("properties", {}).get("source") != SOURCE
    ]
    added = build_features(map_dir)
    lines = [
        "{",
        '"type": "FeatureCollection",',
        f'"name": {json.dumps(document.get("name", "rivers"))},',
        f'"xy_coordinate_resolution": {document.get("xy_coordinate_resolution", 0.1)},',
        '"features": [',
    ]
    texts = [
        _feature_text(
            {
                **feature["properties"],
                "coords": feature["geometry"]["coordinates"],
            }
        )
        for feature in kept
    ] + [_feature_text(feature) for feature in added]
    lines.append(",\n".join(texts))
    lines += ["]", "}"]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return added
