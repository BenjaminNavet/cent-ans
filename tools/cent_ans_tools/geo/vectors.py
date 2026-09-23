"""Rivers and coastline clipped to the grid and expressed in pixel coordinates.

Output GeoJSON files carry no CRS: coordinates are map pixels (X east, Y
south, origin at the north-west corner), rounded to one decimal.
"""

from __future__ import annotations

import warnings
from pathlib import Path

import geopandas as gpd
import numpy as np
import pandas as pd
import shapely
from shapely.geometry import box

from cent_ans_tools.geo.project import CRS_MAP, MapGrid

RIVER_COLUMNS = ["name", "scalerank", "featurecla"]
COAST_COLUMNS = ["scalerank", "featurecla"]


def clip_to_grid(frame: gpd.GeoDataFrame, grid: MapGrid) -> gpd.GeoDataFrame:
    """Project to EPSG:3035 and clip to the grid bounds."""
    projected = frame.to_crs(CRS_MAP)
    clipped = gpd.clip(projected, box(*grid.bounds))
    clipped = clipped[~clipped.geometry.is_empty]
    return clipped.explode(index_parts=False).reset_index(drop=True)


def to_pixel_frame(
    frame: gpd.GeoDataFrame, grid: MapGrid, decimals: int = 1
) -> gpd.GeoDataFrame:
    """Rewrite EPSG:3035 geometries in pixel coordinates (CRS dropped)."""

    def convert(coords: np.ndarray) -> np.ndarray:
        px, py = grid.projected_to_pixel(coords[:, 0], coords[:, 1])
        return np.round(np.column_stack([px, py]), decimals)

    geometry = shapely.transform(frame.geometry.values, convert)
    return gpd.GeoDataFrame(frame.drop(columns="geometry"), geometry=geometry, crs=None)


def _select_columns(frame: gpd.GeoDataFrame, columns: list[str]) -> gpd.GeoDataFrame:
    kept = [column for column in columns if column in frame.columns]
    result = frame[[*kept, "geometry"]].copy()
    for column in columns:
        if column not in result.columns:
            result[column] = None
    return result


def prepare_rivers(
    base: gpd.GeoDataFrame, europe: gpd.GeoDataFrame | None, grid: MapGrid
) -> gpd.GeoDataFrame:
    """Merge the global rivers with the Europe supplement and pixelise.

    Args:
        base: ``ne_10m_rivers_lake_centerlines``.
        europe: ``ne_10m_rivers_europe`` (may be ``None``).
        grid: Target grid.

    Returns:
        LineStrings in pixel coordinates with ``name``, ``scalerank``,
        ``featurecla`` and ``source`` properties.
    """
    frames = [
        _select_columns(base, RIVER_COLUMNS).assign(
            source="ne_10m_rivers_lake_centerlines"
        )
    ]
    if europe is not None:
        frames.append(
            _select_columns(europe, RIVER_COLUMNS).assign(source="ne_10m_rivers_europe")
        )
    merged = gpd.GeoDataFrame(pd.concat(frames, ignore_index=True), crs=base.crs)
    merged["scalerank"] = merged["scalerank"].astype("Int64")
    clipped = clip_to_grid(merged, grid)
    clipped = clipped[clipped.geometry.geom_type == "LineString"]
    return to_pixel_frame(clipped, grid)


def prepare_coastline(coast: gpd.GeoDataFrame, grid: MapGrid) -> gpd.GeoDataFrame:
    """Clip ``ne_10m_coastline`` to the grid and pixelise."""
    clipped = clip_to_grid(_select_columns(coast, COAST_COLUMNS), grid)
    clipped = clipped[clipped.geometry.geom_type == "LineString"]
    return to_pixel_frame(clipped, grid)


def write_geojson(frame: gpd.GeoDataFrame, path: Path) -> None:
    """Write a CRS-less GeoDataFrame as GeoJSON with one-decimal coordinates."""
    with warnings.catch_warnings():
        warnings.filterwarnings("ignore", message="'crs' was not provided")
        frame.to_file(path, driver="GeoJSON", COORDINATE_PRECISION=1, RFC7946="NO")
