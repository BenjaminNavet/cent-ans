"""Fichiers d'auteur en pixels de carte, ramenés sur le canevas composé (lot CC, C4b).

``cent-ans compose authored-px`` :

* ``coast_types.json`` et ``sea_basins.json`` : leurs polygones ``polygon_px`` étaient dessinés dans
  la grille réelle (EPSG:3035) ; chaque sommet passe par la déformation avant (hors emprise : le
  point est ramené au bord du recadrage, puis le décalage restant est rajouté à l'échelle 1).
  Idempotent : la clé ``px_frame: "composed"`` marque un fichier déjà converti ;
* ``wetland_sites_px.json`` : centres et rayons recalculés depuis ``wetlands.json`` par la grille composée ;
* ``map_landmarks_extra.json`` : ``decor_bake.bake`` (px de chaque site depuis son lonlat).
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"
FRAME_KEY = "px_frame"
FRAME = "composed"


def warp_points(cw: ComposedWarp, points: list[list[float]]) -> list[list[float]]:
    """Sommets en pixels réels -> pixels composés (1 décimale), extrapolés hors recadrage."""
    x0, y0, x1, y1 = cw.crop_box_px
    pts = np.asarray(points, dtype=np.float64)
    cx = np.clip(pts[:, 0], x0, x1 - 1)
    cy = np.clip(pts[:, 1], y0, y1 - 1)
    fx, fy = cw.forward_px(cx, cy)
    out = np.stack([fx + (pts[:, 0] - cx), fy + (pts[:, 1] - cy)], axis=1)
    return [[round(float(x), 1), round(float(y), 1)] for x, y in out]


def warp_polygon_file(path: Path, key: str, cw: ComposedWarp) -> int:
    """Convertit ``polygon_px`` de chaque entrée de ``key`` ; renvoie le nombre de polygones."""
    doc = json.loads(path.read_text("utf-8"))
    if doc.get(FRAME_KEY) == FRAME:
        return 0
    for entry in doc[key]:
        entry["polygon_px"] = warp_points(cw, entry["polygon_px"])
    doc[FRAME_KEY] = FRAME
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return len(doc[key])


def bake_wetland_sites(map_dir: Path = MAP_DIR) -> int:
    """Recalcule ``wetland_sites_px.json`` (centre = moyenne de l'anneau, rayon = distance maximale)."""
    from cent_ans_tools.geo import landcover
    from cent_ans_tools.geo.project import grid_from_metadata

    grid = grid_from_metadata(json.loads((map_dir / "map.json").read_text("utf-8")))
    areas = {
        a["id"]: a
        for a in json.loads((map_dir / "wetlands.json").read_text("utf-8"))["areas"]
    }
    path = map_dir / "wetland_sites_px.json"
    doc = json.loads(path.read_text("utf-8"))
    for site_id, site in doc["sites"].items():
        area = areas.get(site_id)
        if area is None:
            continue
        ring = (
            area["polygon"]
            if "polygon" in area
            else landcover.ellipse_polygon(area["ellipse"])
        )
        xs, ys = grid.lonlat_to_pixel(
            np.array([p[0] for p in ring]), np.array([p[1] for p in ring])
        )
        ok = np.isfinite(xs) & np.isfinite(ys)
        if not ok.any():
            continue
        cx, cy = float(xs[ok].mean()), float(ys[ok].mean())
        site["px"] = [round(cx, 1), round(cy, 1)]
        site["radius_px"] = round(float(np.max(np.hypot(xs[ok] - cx, ys[ok] - cy))), 1)
    path.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return len(doc["sites"])


def build(map_dir: Path = MAP_DIR) -> dict:
    """Enchaîne les quatre conversions ; renvoie un rapport."""
    from cent_ans_tools import decor_bake

    cw = ComposedWarp.load(map_dir / "compose")
    report = {
        "coast_regions": warp_polygon_file(map_dir / "coast_types.json", "regions", cw),
        "sea_basins": warp_polygon_file(map_dir / "sea_basins.json", "basins", cw),
        "wetland_sites": bake_wetland_sites(map_dir),
    }
    decor_bake.bake()
    report["landmark_sites"] = "baked"
    return report


__all__ = ["build", "warp_points"]
