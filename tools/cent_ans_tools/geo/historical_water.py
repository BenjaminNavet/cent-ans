"""Water of 1340 in the land mask: historical lakes in, modern reservoirs out (lot LR-10).

``land_mask.png`` is Natural Earth land minus Natural Earth lakes (``geo build``).
Two corrections bring it back to 1340 (ADR 0168):

- **Modern reservoirs out.** A Natural Earth lake polygon that holds the point
  of a dam reservoir of ``modern_reservoirs.json`` (ADR 0036: Sainte-Croix, Der,
  Orient, Ebro, Riaño...) is not punched: its pixels stay land.
- **Historical lakes in.** ``historical_lakes.json`` lists lakes of 1340 that
  are drained today (Fucino, Copais, Amouq, Haarlemmermeer, Whittlesey Mere) or
  too small for Natural Earth (Windermere, Paladru, Joux...). Each has a rough
  hand-drawn envelope (polygon, or axis + width); the punched water is the
  envelope where ``heightmap.png`` is at most ``max_height_m`` (the drained
  basin floor), largest connected part only, or the whole envelope when the
  threshold is null.

``cent-ans geo land-mask`` rebuilds ``land_mask.png`` only (byte-identical to
``geo build`` without the corrections), gives every pixel turned back to land
the province of the nearest labelled pixel in ``province_ids.png`` (lakes are
province 0), and recomputes the shore and province-border distance rasters of
the terrain shader (``coast_dist.png``, ``province_border_dist.png``). Then run
``geo lakes`` (water sheets) and ``geo navgrid`` (lakes impassable).
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

import geopandas as gpd
import numpy as np
import pandas as pd
from PIL import Image
from rasterio.features import rasterize
from scipy import ndimage
from shapely.geometry import LineString, Point, Polygon
from shapely.geometry.base import BaseGeometry

from cent_ans_tools.geo import download, terrain
from cent_ans_tools.geo.project import CRS_GEO, CRS_MAP, MapGrid, grid_from_metadata
from cent_ans_tools.geo.provinces import decode_ids, encode_ids

DATA = Path(__file__).resolve().parents[3] / "data"
MAP_DIR = DATA / "map"
HISTORICAL_FILE = MAP_DIR / "historical_lakes.json"
RESERVOIRS_FILE = MAP_DIR / "modern_reservoirs.json"

#: A Natural Earth polygon is dropped as a reservoir only if it is not much
#: larger than the reservoir (a dam point near a big natural lake is kept).
RESERVOIR_AREA_FACTOR = 6.0

Image.MAX_IMAGE_PIXELS = None


@dataclass
class LandMaskResult:
    """Summary of a ``geo land-mask`` run."""

    land_mask: Path
    dropped_reservoirs: list[str] = field(default_factory=list)
    punched: dict[str, int] = field(default_factory=dict)
    to_land: int = 0
    to_water: int = 0
    refilled_provinces: int = 0
    settlements_in_water: list[str] = field(default_factory=list)


def load_historical(path: Path = HISTORICAL_FILE) -> list[dict]:
    """Entries of ``historical_lakes.json`` (empty when the file is missing)."""
    if not path.exists():
        return []
    return json.loads(path.read_text(encoding="utf-8"))["lakes"]


def load_reservoirs(path: Path = RESERVOIRS_FILE) -> list[dict]:
    """Entries of ``modern_reservoirs.json``."""
    return json.loads(path.read_text(encoding="utf-8"))["reservoirs"]


def envelope(entry: dict) -> BaseGeometry:
    """Envelope of a historical lake in map coordinates (EPSG:3035, metres).

    A ``polygon`` is taken as is; an ``axis`` is buffered by half ``width_m``.
    """
    if "polygon" in entry:
        shape: BaseGeometry = Polygon(entry["polygon"])
        return gpd.GeoSeries([shape], crs=CRS_GEO).to_crs(CRS_MAP).iloc[0]
    line = gpd.GeoSeries([LineString(entry["axis"])], crs=CRS_GEO).to_crs(CRS_MAP)
    return line.iloc[0].buffer(entry["width_m"] / 2.0)


def envelope_raster(entry: dict, grid: MapGrid) -> np.ndarray:
    """Pixels touched by the envelope (``all_touched``: narrow lakes stay connected)."""
    return rasterize(
        [(envelope(entry), 1)],
        out_shape=grid.shape,
        transform=grid.transform,
        fill=0,
        dtype=np.uint8,
        all_touched=True,
    ).astype(bool)


def punch_area(entry: dict, grid: MapGrid, height_m: np.ndarray) -> np.ndarray:
    """Water a historical lake adds to the land mask (empty if not ``punch_mask``).

    The envelope where the height is at most ``max_height_m``, largest
    connected part (4-connectivity); the whole envelope when the threshold
    is null.
    """
    area = envelope_raster(entry, grid)
    if not entry.get("punch_mask", False):
        return np.zeros_like(area)
    threshold = entry.get("max_height_m")
    if threshold is None:
        return area
    basin = area & (height_m <= threshold)
    labels, count = ndimage.label(basin)
    if count == 0:
        return basin
    sizes = np.bincount(labels.ravel())
    sizes[0] = 0
    return labels == int(sizes.argmax())


def reservoir_lakes(lakes: gpd.GeoDataFrame, reservoirs: list[dict]) -> dict[int, str]:
    """Rows of the Natural Earth lakes that are a modern reservoir (row -> reservoir id).

    A row is a reservoir when its polygon holds the reservoir point and is at
    most :data:`RESERVOIR_AREA_FACTOR` times the reservoir's area.
    """
    projected = lakes.to_crs(CRS_MAP)
    points = gpd.GeoSeries(
        [Point(r["lon"], r["lat"]) for r in reservoirs], crs=CRS_GEO
    ).to_crs(CRS_MAP)
    found: dict[int, str] = {}
    for reservoir, point in zip(reservoirs, points, strict=True):
        hits = projected.index[projected.geometry.contains(point)]
        for row in hits:
            area_km2 = projected.geometry.loc[row].area / 1e6
            if area_km2 <= RESERVOIR_AREA_FACTOR * max(reservoir["area_km2"], 1.0):
                found.setdefault(int(row), reservoir["id"])
    return found


def corrected_land_mask(
    grid: MapGrid,
    land: gpd.GeoDataFrame,
    lakes: gpd.GeoDataFrame,
    height_m: np.ndarray,
    reservoirs: list[dict],
    historical: list[dict],
) -> tuple[np.ndarray, list[str], dict[str, int]]:
    """``terrain.build_land_mask`` with the 1340 corrections.

    Returns:
        ``(mask, dropped, punched)``: the ``uint8`` mask (255 = land), the
        reservoir ids whose Natural Earth polygon was dropped, and the number
        of pixels each historical lake turned to water.
    """
    dropped = reservoir_lakes(lakes, reservoirs)
    kept = lakes.drop(index=list(dropped))
    mask = terrain.build_land_mask(grid, land, kept)
    punched: dict[str, int] = {}
    for entry in historical:
        area = punch_area(entry, grid, height_m)
        punched[entry["id"]] = int((area & (mask > 0)).sum())
        mask[area] = 0
    return mask, sorted(set(dropped.values())), punched


def refill_provinces(
    ids: np.ndarray, old_land: np.ndarray, new_land: np.ndarray
) -> tuple[np.ndarray, int]:
    """Province ids after a land-mask change.

    Pixels turned to land take the province of the nearest labelled pixel;
    pixels turned to water keep theirs (a lake inside a province still belongs
    to it for the picker; armies are kept off by the navigation grid).

    Returns:
        ``(ids, refilled)``.
    """
    gained = new_land & ~old_land
    if not gained.any():
        return ids, 0
    _, (rows, cols) = ndimage.distance_transform_edt(ids == 0, return_indices=True)
    out = ids.copy()
    out[gained] = ids[rows[gained], cols[gained]]
    return out, int(gained.sum())


def _natural_earth(layer: str) -> gpd.GeoDataFrame:
    return gpd.read_file(download.natural_earth_shapefile(layer))


def _settlements_turned_wet(
    map_dir: Path, old_land: np.ndarray, new_land: np.ndarray
) -> list[str]:
    """Settlements (``settlements_px.json``) whose pixel went from land to water."""
    path = map_dir / "settlements_px.json"
    if not path.exists():
        return []
    positions = json.loads(path.read_text(encoding="utf-8"))
    rows, cols = new_land.shape
    wet = []
    for settlement, (x, y) in positions.items():
        col, row = int(x), int(y)
        inside = 0 <= col < cols and 0 <= row < rows
        if inside and old_land[row, col] and not new_land[row, col]:
            wet.append(settlement)
    return sorted(wet)


def build(map_dir: Path = MAP_DIR) -> LandMaskResult:
    """Rewrite ``land_mask.png`` with the 1340 corrections and patch its dependants."""
    from cent_ans_tools.geo import splat

    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    grid = grid_from_metadata(meta)
    height_m = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    lakes = gpd.GeoDataFrame(
        pd.concat(
            [_natural_earth("lakes"), _natural_earth("lakes_europe")],
            ignore_index=True,
        ),
        crs=_natural_earth("lakes").crs,
    )
    mask, dropped, punched = corrected_land_mask(
        grid,
        _natural_earth("land"),
        lakes,
        height_m,
        load_reservoirs(map_dir / RESERVOIRS_FILE.name),
        load_historical(map_dir / HISTORICAL_FILE.name),
    )
    mask_path = map_dir / "land_mask.png"
    with Image.open(mask_path) as image:
        old_land = np.asarray(image.convert("L")) > 127
    new_land = mask > 127
    terrain.write_png8(mask, mask_path)

    ids_path = map_dir / "province_ids.png"
    with Image.open(ids_path) as image:
        ids = decode_ids(np.asarray(image.convert("RGB")))
    ids, refilled = refill_provinces(ids, old_land, new_land)
    if refilled:
        Image.fromarray(encode_ids(ids), mode="RGB").save(ids_path, compress_level=9)

    # Shader rasters that read the land mask (same code as ``geo splat``).
    water = (height_m <= 0.0) | ~new_land
    terrain.write_png8(
        splat.encode_coast_dist(splat.compute_coast_dist(water)),
        map_dir / "coast_dist.png",
    )
    Image.fromarray(
        splat.encode_border_dist(splat.compute_border_dist(ids, new_land)), mode="RGB"
    ).save(map_dir / "province_border_dist.png", compress_level=9)

    return LandMaskResult(
        land_mask=mask_path,
        dropped_reservoirs=dropped,
        punched=punched,
        to_land=int((new_land & ~old_land).sum()),
        to_water=int((old_land & ~new_land).sum()),
        refilled_provinces=refilled,
        settlements_in_water=_settlements_turned_wet(map_dir, old_land, new_land),
    )
