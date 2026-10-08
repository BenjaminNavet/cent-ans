"""Eaux douces de la carte (lot DN-ME4) : sites en pixels pour `FreshwaterLayer`.

Entrées : ``map_freshwater.json`` (réglages et sites historiques en lon/lat), ``wetlands.json``,
``rivers_render.json`` et ``heightmap.png``. Sortie : ``freshwater_px.json`` (zones humides,
salins, cascades, gués en pixels carte ; tronçons de torrent détectés par la pente du lit).
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import terrain
from cent_ans_tools.geo.project import grid_from_metadata

MAP_DIR = Path(__file__).resolve().parents[3] / "data" / "map"
CONFIG_FILE = "map_freshwater.json"
OUTPUT_FILE = "freshwater_px.json"


def _ellipse_px(grid, center_lonlat, radii_km, angle_deg) -> dict:
    cx, cy = grid.lonlat_to_pixel(*center_lonlat)
    mpp = grid.meters_per_px
    return {
        "center": [round(float(cx), 2), round(float(cy), 2)],
        "radii_px": [
            round(radii_km[0] * 1000.0 / mpp, 3),
            round(radii_km[1] * 1000.0 / mpp, 3),
        ],
        "angle_deg": angle_deg,
    }


def torrent_runs(rivers: list[dict], height_m: np.ndarray, config: dict, mpp: float):
    """Tronçons de fleuve dont la pente lissée dépasse ``min_slope``."""
    rows, cols = height_m.shape
    runs = []
    for river in rivers:
        pts = np.asarray(river["points"], dtype=np.float64)
        if len(pts) < 4:
            continue
        xs = np.clip(pts[:, 0].astype(int), 0, cols - 1)
        ys = np.clip(pts[:, 1].astype(int), 0, rows - 1)
        z = height_m[ys, xs]
        step = np.hypot(*np.diff(pts, axis=0).T) * mpp
        slope = -np.diff(z) / np.maximum(step, 1.0)
        slope = np.convolve(slope, np.ones(3) / 3.0, mode="same")
        steep = slope >= config["min_slope"]
        i = 0
        while i < len(steep):
            if not steep[i]:
                i += 1
                continue
            j = i
            while j < len(steep) and steep[j]:
                j += 1
            if j - i >= config["min_points"] and z[i] > 40.0:
                runs.append(
                    {
                        "river": river.get("name", ""),
                        "slope": round(float(slope[i:j].mean()), 4),
                        "points": [
                            [round(float(p[0]), 2), round(float(p[1]), 2)]
                            for p in pts[i : j + 1]
                        ],
                        "altitude_m": int(z[i]),
                    }
                )
            i = j
    runs.sort(key=lambda r: -r["slope"] * len(r["points"]))
    return runs[: config["max_runs"]]


def build(map_dir: Path = MAP_DIR) -> dict:
    """Écrit ``freshwater_px.json`` et renvoie son contenu."""
    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    grid = grid_from_metadata(meta)
    config = json.loads((map_dir / CONFIG_FILE).read_text(encoding="utf-8"))
    wet = json.loads((map_dir / "wetlands.json").read_text(encoding="utf-8"))
    lagoons = set(config["reeds"]["lagoon_ids"])
    wetlands = []
    for area in wet["areas"]:
        if "ellipse" not in area:
            continue
        e = area["ellipse"]
        entry = {
            "id": area["id"],
            "kind": area["kind"],
            "density": area.get("density", 0.5),
            "lagoon": area["id"] in lagoons,
        }
        entry.update(_ellipse_px(grid, e["center"], e["radii_km"], e.get("angle_deg", 0.0)))
        wetlands.append(entry)
    salt_pans = []
    for pan in config["salt_pans"]:
        entry = {"id": pan["id"], "name": pan["name"], "tint": pan["tint"]}
        entry.update(
            _ellipse_px(
                grid,
                pan["center"],
                [pan["size_km"][0] / 2.0, pan["size_km"][1] / 2.0],
                pan["angle_deg"],
            )
        )
        salt_pans.append(entry)

    def points(key: str) -> list[dict]:
        out = []
        for item in config[key]:
            x, y = grid.lonlat_to_pixel(*item["lonlat"])
            entry = {k: v for k, v in item.items() if k != "lonlat"}
            entry["px"] = [round(float(x), 2), round(float(y), 2)]
            out.append(entry)
        return out

    height = terrain.uint16_to_height(
        terrain.read_png16(map_dir / "heightmap.png")
    ).astype(np.float32)
    rivers = json.loads((map_dir / "rivers_render.json").read_text(encoding="utf-8"))
    result = {
        "description": "Généré par `cent-ans geo freshwater-sites` (lot DN-ME4) depuis map_freshwater.json : ne pas modifier à la main.",
        "wetlands": wetlands,
        "salt_pans": salt_pans,
        "waterfalls": points("waterfalls"),
        "fords": points("fords"),
        "torrents": torrent_runs(
            rivers["rivers"], height, config["torrent"], grid.meters_per_px
        ),
    }
    (map_dir / OUTPUT_FILE).write_text(
        json.dumps(result, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    return result

