"""Orchestrate the map build: terrain, then provinces, and the documentation previews."""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

import geopandas as gpd
import numpy as np
import pandas as pd
from PIL import Image, ImageDraw

from cent_ans_tools.geo import (
    download,
    hamlets,
    navgrid,
    provinces,
    relief,
    roads,
    settlements,
    terrain,
    vectors,
)
from cent_ans_tools.geo.project import (
    CRS_MAP,
    LAT_MAX,
    LAT_MIN,
    LON_MAX,
    LON_MIN,
    MapGrid,
    default_grid,
)

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
PREVIEW_PATH = REPO_DIR / "docs" / "img" / "map-preview.png"
PREVIEW_SIZE = 1792  # preview width (1792 x 1536, 4 map pixels each)


@dataclass(frozen=True)
class BuildResult:
    """Paths written by :func:`build`."""

    map_json: Path
    heightmap: Path
    land_mask: Path
    rivers: Path
    coastline: Path
    preview: Path
    provinces: provinces.ProvinceResult
    relief: relief.ReliefResult | None = None
    roads: roads.RoadResult | None = None
    settlements: settlements.SettlementResult | None = None
    hamlets: hamlets.HamletResult | None = None
    navgrid: navgrid.NavgridResult | None = None


#: A pixel of the open Atlantic (Bay of Biscay): seed of the ocean for
#: :func:`terrain.lift_inland_depressions`.
OCEAN_SEED_LONLAT = (-8.0, 45.0)


def map_metadata(grid: MapGrid, etopo_tiles: list[str]) -> dict:
    """The ``map.json`` contract plus provenance fields."""
    return {
        "crs": CRS_MAP,
        "bounds_projected": list(grid.bounds),
        "size_px": [grid.width_px, grid.height_px],
        "meters_per_px": grid.meters_per_px,
        "height_min_m": terrain.HEIGHT_MIN_M,
        "height_max_m": terrain.HEIGHT_MAX_M,
        "extent_lonlat": [LON_MIN, LAT_MIN, LON_MAX, LAT_MAX],
        "height_tiles": relief.height_tiles_meta(grid.scaled(relief.FINE_SCALE)),
        "sources": {
            "dem": {"name": "ETOPO 2022 v1 15s surface", "tiles": etopo_tiles},
            "vectors": {
                "name": "Natural Earth 10m physical",
                "layers": list(download.NATURAL_EARTH_LAYERS.values()),
            },
        },
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat(),
    }


def hillshade(
    height_m: np.ndarray,
    meters_per_px: float,
    azimuth_deg: float = 315.0,
    altitude_deg: float = 45.0,
) -> np.ndarray:
    """Classic Lambertian hillshade in ``[0, 1]`` (vertical exaggeration x3)."""
    dy, dx = np.gradient(height_m * 3.0, meters_per_px)
    slope = np.arctan(np.hypot(dx, dy))
    aspect = np.arctan2(-dx, dy)
    azimuth = np.radians(360.0 - azimuth_deg + 90.0)
    altitude = np.radians(altitude_deg)
    shade = np.sin(altitude) * np.cos(slope) + np.cos(altitude) * np.sin(
        slope
    ) * np.cos(azimuth - aspect)
    return np.clip(shade, 0.0, 1.0)


def render_preview(
    height_m: np.ndarray,
    land_mask: np.ndarray,
    rivers: gpd.GeoDataFrame,
    coast: gpd.GeoDataFrame,
    grid: MapGrid,
    path: Path,
    size: int = PREVIEW_SIZE,
) -> None:
    """Render a downscaled hypsometric + hillshade preview with vectors."""
    factor = max(1, grid.width_px // size)
    rows, cols = grid.height_px // factor, grid.width_px // factor
    small_height = (
        height_m[: rows * factor, : cols * factor]
        .reshape(rows, factor, cols, factor)
        .mean(axis=(1, 3))
    )
    small_land = (
        land_mask[: rows * factor, : cols * factor]
        .reshape(rows, factor, cols, factor)
        .max(axis=(1, 3))
        > 0
    )
    shade = hillshade(small_height, grid.meters_per_px * factor)
    relief = np.clip(small_height, 0.0, 2500.0) / 2500.0
    land_rgb = np.stack(
        [
            0.55 + 0.35 * relief,
            0.65 - 0.25 * relief,
            0.40 - 0.20 * relief,
        ],
        axis=-1,
    )
    sea_rgb = np.broadcast_to(np.array([0.45, 0.60, 0.78]), land_rgb.shape)
    rgb = np.where(small_land[..., None], land_rgb, sea_rgb) * (
        0.5 + 0.5 * shade[..., None]
    )
    image = Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8), mode="RGB")
    draw = ImageDraw.Draw(image)
    scale = 1.0 / factor
    for geom in rivers.geometry:
        draw.line(
            [(x * scale, y * scale) for x, y in geom.coords],
            fill=(30, 60, 200),
            width=1,
        )
    for geom in coast.geometry:
        draw.line(
            [(x * scale, y * scale) for x, y in geom.coords], fill=(20, 20, 20), width=1
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)


def build(
    force: bool = False, map_dir: Path = MAP_DIR, preview_path: Path = PREVIEW_PATH
) -> BuildResult:
    """Download (cached), build every terrain output and the preview.

    Args:
        force: Re-download raw data even if cached.
        map_dir: Output directory (``data/map``).
        preview_path: Where the PNG preview is written.
    """
    grid = default_grid()
    map_dir.mkdir(parents=True, exist_ok=True)

    tile_names = download.etopo_tiles_for_grid(grid)
    tile_paths = download.etopo_tiles(tile_names, force)
    shapefiles = {
        layer: download.natural_earth_shapefile(layer, force)
        for layer in download.NATURAL_EARTH_LAYERS
    }

    def read(layer: str) -> gpd.GeoDataFrame:
        return gpd.read_file(shapefiles[layer])

    height_m = terrain.build_heightmap(grid, tile_paths)

    lakes = gpd.GeoDataFrame(
        pd.concat([read("lakes"), read("lakes_europe")], ignore_index=True),
        crs=read("lakes").crs,
    )
    land_mask = terrain.build_land_mask(grid, read("land"), lakes)
    land_mask_path = map_dir / "land_mask.png"
    terrain.write_png8(land_mask, land_mask_path)

    # Caspian depression and Jordan rift: land below 0 m cut off from the ocean.
    ocean_col, ocean_row = grid.lonlat_to_pixel(*OCEAN_SEED_LONLAT)
    height_m, _ = terrain.lift_inland_depressions(
        height_m, land_mask > 0, (int(ocean_row), int(ocean_col))
    )
    heightmap_path = map_dir / "heightmap.png"
    terrain.write_png16(terrain.height_to_uint16(height_m), heightmap_path)

    rivers = vectors.prepare_rivers(read("rivers"), read("rivers_europe"), grid)
    rivers_path = map_dir / "rivers.geojson"
    vectors.write_geojson(rivers, rivers_path)
    coast = vectors.prepare_coastline(read("coastline"), grid)
    coast_path = map_dir / "coastline.geojson"
    vectors.write_geojson(coast, coast_path)

    map_json_path = map_dir / "map.json"
    map_json_path.write_text(
        json.dumps(map_metadata(grid, tile_names), indent=2) + "\n", encoding="utf-8"
    )

    render_preview(height_m, land_mask, rivers, coast, grid, preview_path)
    province_result = provinces.build(map_dir=map_dir)
    # Lot C3: relief tiles, roads, settlement graph (uses roads), hamlets.
    relief_result = relief.build(force=force, map_dir=map_dir)
    road_result = roads.build(force=force, map_dir=map_dir)
    settlement_result = settlements.build(map_dir=map_dir)
    hamlet_result = hamlets.build(force=force, map_dir=map_dir)
    # Lot M1: navigation grid (reads splat.png from `cent-ans geo splat` when present).
    navgrid_result = navgrid.build(map_dir=map_dir)
    return BuildResult(
        map_json_path,
        heightmap_path,
        land_mask_path,
        rivers_path,
        coast_path,
        preview_path,
        province_result,
        relief_result,
        road_result,
        settlement_result,
        hamlet_result,
        navgrid_result,
    )


def info(map_dir: Path = MAP_DIR) -> dict:
    """Summarise an existing ``data/map`` (metadata, file sizes, height range)."""
    metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    files = {}
    for name in (
        "heightmap.png",
        "land_mask.png",
        "rivers.geojson",
        "coastline.geojson",
        "provinces.geojson",
        "province_ids.png",
        "settlement_graph.json",
        "settlement_edge_paths.json",
        "settlements_px.json",
        "roads.geojson",
        "hamlets.json",
        "navgrid.png",
        "crossings.json",
    ):
        path = map_dir / name
        files[name] = path.stat().st_size if path.exists() else None
    summary = {"metadata": metadata, "files": files}
    heightmap = map_dir / "heightmap.png"
    if heightmap.exists():
        encoded = terrain.read_png16(heightmap)
        heights = terrain.uint16_to_height(encoded)
        summary["height_range_m"] = [float(heights.min()), float(heights.max())]
    land_mask = map_dir / "land_mask.png"
    if land_mask.exists():
        with Image.open(land_mask) as image:
            summary["land_fraction"] = float((np.asarray(image) > 0).mean())
    return summary
