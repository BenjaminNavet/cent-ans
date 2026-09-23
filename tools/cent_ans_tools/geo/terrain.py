"""Heightmap and land mask rasters on the map grid.

Height encoding: 16-bit unsigned PNG, linear from :data:`HEIGHT_MIN_M`
(value 0) to :data:`HEIGHT_MAX_M` (value 65535); heights outside the range
are clamped. The land mask is an 8-bit PNG where 255 = land and 0 = sea or
lake (lakes are punched out of the Natural Earth land polygons).
"""

from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path

import geopandas as gpd
import numpy as np
import rasterio
from PIL import Image
from rasterio.enums import Resampling
from rasterio.features import rasterize
from rasterio.merge import merge
from rasterio.warp import reproject

from cent_ans_tools.geo.project import CRS_GEO, CRS_MAP, MapGrid

HEIGHT_MIN_M = -200.0
HEIGHT_MAX_M = 4800.0
UINT16_MAX = 65535


def height_to_uint16(height_m: np.ndarray | float) -> np.ndarray:
    """Encode metres into the 16-bit heightmap scale (clamped, rounded).

    Args:
        height_m: Elevation(s) in metres.

    Returns:
        ``uint16`` array where 0 = -200 m and 65535 = 4800 m.
    """
    scale = UINT16_MAX / (HEIGHT_MAX_M - HEIGHT_MIN_M)
    encoded = np.rint((np.asarray(height_m, dtype=np.float64) - HEIGHT_MIN_M) * scale)
    return np.clip(encoded, 0, UINT16_MAX).astype(np.uint16)


def uint16_to_height(value: np.ndarray | int) -> np.ndarray:
    """Decode 16-bit heightmap values back into metres."""
    scale = (HEIGHT_MAX_M - HEIGHT_MIN_M) / UINT16_MAX
    return np.asarray(value, dtype=np.float64) * scale + HEIGHT_MIN_M


def mosaic_dem(
    tile_paths: Iterable[Path], geo_extent: tuple[float, float, float, float]
) -> tuple[np.ndarray, object]:
    """Merge ETOPO tiles into one WGS84 array covering ``geo_extent``.

    Args:
        tile_paths: GeoTIFF tiles (EPSG:4326, same resolution).
        geo_extent: ``(lon_min, lon_max, lat_min, lat_max)`` clip box.

    Returns:
        ``(array, transform)`` of the merged single band (float32).
    """
    lon_min, lon_max, lat_min, lat_max = geo_extent
    datasets = [rasterio.open(path) for path in tile_paths]
    try:
        mosaic, transform = merge(
            datasets, bounds=(lon_min, lat_min, lon_max, lat_max), nodata=np.nan
        )
    finally:
        for dataset in datasets:
            dataset.close()
    return mosaic[0].astype(np.float32), transform


def build_heightmap(grid: MapGrid, tile_paths: Iterable[Path]) -> np.ndarray:
    """Resample the DEM onto the map grid and return elevations in metres.

    Uses area-averaging resampling (the source is finer than the grid).
    Pixels without data are set to sea level.
    """
    mosaic, src_transform = mosaic_dem(tile_paths, grid.geographic_extent())
    destination = np.full((grid.size_px, grid.size_px), np.nan, dtype=np.float32)
    reproject(
        source=mosaic,
        destination=destination,
        src_transform=src_transform,
        src_crs=CRS_GEO,
        src_nodata=np.nan,
        dst_transform=grid.transform,
        dst_crs=CRS_MAP,
        dst_nodata=np.nan,
        resampling=Resampling.average,
    )
    return np.nan_to_num(destination, nan=0.0)


def build_land_mask(
    grid: MapGrid, land: gpd.GeoDataFrame, lakes: gpd.GeoDataFrame
) -> np.ndarray:
    """Rasterise land polygons (255) and punch lakes out (0).

    Args:
        grid: Target grid.
        land: Land polygons in any CRS.
        lakes: Lake polygons in any CRS.

    Returns:
        ``uint8`` array, 255 = land.
    """
    shape = (grid.size_px, grid.size_px)
    land_shapes = (
        (geom, 255) for geom in land.to_crs(CRS_MAP).geometry if not geom.is_empty
    )
    mask = rasterize(
        land_shapes, out_shape=shape, transform=grid.transform, fill=0, dtype=np.uint8
    )
    lake_shapes = [
        (geom, 1) for geom in lakes.to_crs(CRS_MAP).geometry if not geom.is_empty
    ]
    if lake_shapes:
        lake_raster = rasterize(
            lake_shapes,
            out_shape=shape,
            transform=grid.transform,
            fill=0,
            dtype=np.uint8,
        )
        mask[lake_raster == 1] = 0
    return mask


def write_png16(array: np.ndarray, path: Path) -> None:
    """Write a ``uint16`` array as a 16-bit greyscale PNG (max compression)."""
    Image.fromarray(np.ascontiguousarray(array, dtype=np.uint16), mode="I;16").save(
        path, compress_level=9
    )


def write_png8(array: np.ndarray, path: Path) -> None:
    """Write a ``uint8`` array as an 8-bit greyscale PNG (max compression)."""
    Image.fromarray(np.ascontiguousarray(array, dtype=np.uint8), mode="L").save(
        path, compress_level=9
    )


def read_png16(path: Path) -> np.ndarray:
    """Read a 16-bit greyscale PNG back into a ``uint16`` array."""
    with Image.open(path) as image:
        return np.asarray(image, dtype=np.uint16)
