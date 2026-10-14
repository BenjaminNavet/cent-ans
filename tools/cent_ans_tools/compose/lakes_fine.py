"""Lacs fins de la carte composée (lot CC, C4a).

Le masque terre/mer de C1 ne garde que les plans d'eau intérieurs d'au moins ``min_lake_km2`` (1 800 km²) :
Léman, Constance, Garde, Balaton, Fucino... disparaissent. Ce module les remet dans ``land_mask.png`` :

- les lacs Natural Earth (``lakes``, ``lakes_europe``) d'au moins ``lakes_fine.min_km2`` km² réels, hors retenues
  modernes (``modern_reservoirs.json``), dont le contour réel est tracé à travers la déformation ;
- les lacs de 1340 de ``historical_lakes.json`` marqués ``punch_mask`` (Fucino, Copaïs, Haarlemmermeer...) : le
  bassin est dessiné sur le relief RÉEL (enveloppe sous ``max_height_m``, comme ``geo land-mask``) puis passé à
  travers la déformation (le relief procédural n'a pas de fonds de lac réels) ; gardé s'il couvre au moins
  ``lakes_fine.historical_min_px`` pixels composés.

Appelé à la fin de la compilation du masque (``compile.compile_all``) : le relief (C2) en fait des cuvettes plates.
"""

from __future__ import annotations

import numpy as np
from rasterio import features
from rasterio.features import rasterize
from shapely.geometry import shape as to_shape
from shapely.ops import unary_union

from cent_ans_tools.geo import download, historical_water
from cent_ans_tools.geo.project import CRS_MAP, composed_geometry


def _real_height() -> np.ndarray:
    """Real heightmap (metres) of ``data/map/real``."""
    import json

    from cent_ans_tools.compose.warp import real_path
    from cent_ans_tools.geo import terrain

    meta = json.loads(real_path("map.json").read_text(encoding="utf-8"))
    raw = terrain.read_png16(real_path("heightmap.png")).astype(np.float32)
    low, high = float(meta["height_min_m"]), float(meta["height_max_m"])
    return low + raw * (high - low) / 65535.0


def natural_earth_lakes():  # noqa: ANN201
    """Lacs Natural Earth (monde + Europe) en un seul tableau géographique."""
    import geopandas as gpd
    import pandas as pd

    frames = [
        gpd.read_file(download.natural_earth_shapefile(layer))
        for layer in ("lakes", "lakes_europe")
    ]
    return gpd.GeoDataFrame(pd.concat(frames, ignore_index=True), crs=frames[0].crs)


def fine_lake_shapes(cw, cfg: dict, reservoirs: list[dict], historical: list[dict]):  # noqa: ANN001, ANN201
    """``(natural, historical)`` : formes en pixels composés des lacs à remettre dans le masque."""
    lakes = natural_earth_lakes()
    dropped = historical_water.reservoir_lakes(lakes, reservoirs)
    lakes = lakes.drop(index=list(dropped)).to_crs(CRS_MAP)
    min_km2 = float(cfg["min_km2"])
    natural = []
    for geometry in lakes.geometry:
        if geometry is None or geometry.is_empty or geometry.area / 1e6 < min_km2:
            continue
        shape = composed_geometry(geometry, cw)
        if not shape.is_empty:
            natural.append(shape)
    hist = []
    real = cw.real_map_grid()
    real_height = _real_height()
    for entry in historical:
        if not entry.get("punch_mask", False):
            continue
        # Basin drawn on the REAL relief (drained lakes: envelope under ``max_height_m``), then
        # carried through the warp: the procedural relief has no real lake floors.
        area = historical_water.punch_area(entry, real, real_height)
        if not area.any():
            continue
        polygons = [
            to_shape(geometry)
            for geometry, _ in features.shapes(
                area.astype(np.uint8), mask=area, transform=real.transform
            )
        ]
        placed = composed_geometry(unary_union(polygons), cw)
        if not placed.is_empty:
            hist.append((entry["id"], placed))
    return natural, hist


def punch(mask: np.ndarray, cw, cfg: dict | None, map_dir) -> dict:  # noqa: ANN001
    """Met à 0 (eau) les lacs fins dans ``mask`` (uint8, en place) ; renvoie un rapport."""
    if not cfg:
        return {"natural": 0, "historical": 0}
    reservoirs = historical_water.load_reservoirs(
        map_dir / historical_water.RESERVOIRS_FILE.name
    )
    historical = historical_water.load_historical(
        map_dir / historical_water.HISTORICAL_FILE.name
    )
    natural, hist = fine_lake_shapes(cw, cfg, reservoirs, historical)
    added = 0
    for shape in natural:
        raster = rasterize([(shape, 1)], out_shape=mask.shape, fill=0, dtype=np.uint8)
        # Un grand lac déjà en eau (>= min_lake_km2) n'est pas compté.
        if int(((raster > 0) & (mask > 0)).sum()) > 0:
            added += 1
        mask[raster > 0] = 0
    kept = []
    for lake_id, shape in hist:
        raster = rasterize(
            [(shape, 1)], out_shape=mask.shape, fill=0, dtype=np.uint8, all_touched=True
        )
        if int(raster.sum()) < int(cfg["historical_min_px"]):
            continue
        mask[raster > 0] = 0
        kept.append(lake_id)
    return {"natural": added, "historical": kept}
