"""Land cover around AD 1340 for the campaign terrain (lot R1, ADR 0019).

Outputs in ``data/map/`` (same grid as ``province_ids.png``, 4096²):

``splat.png``
    RGBA8, same contract as :mod:`cent_ans_tools.geo.splat` (R grassland,
    G farmland, B forest, A rock / heath; sum 255 on land). The forest channel
    is rebuilt from sourced data instead of per-province mixes and noise:

    * cleared share: KK10 anthropogenic land use averaged over 1330-1349
      (:mod:`cent_ans_tools.geo.kk10`, 5′ grid, bilinear);
    * potential forest: 1 on land, minus the tree line (falling with latitude),
      heath and marsh provinces, dunes and the curated wetlands;
    * target forest share ``F = potential x (1 - cleared)``, raised inside the
      named forests of ``historical_forests.json`` and lowered in its heaths;
    * allocation: forest where a score (fractal noise with uniform marginal +
      terrain preference: slopes, crests, poor high soils; minus the
      surroundings of towns, villages and hamlets and the river flood plains)
      is in the top ``F`` share. The result is binary (massifs with clean
      edges, clearings and assarts around settlements), then anti-aliased.

    The other channels keep the mix of :func:`cent_ans_tools.geo.splat.compute_splat`
    (farmland / grassland / rock), scaled to the non-forest share; named heaths
    feed the A channel.
``forest_kind.png``
    L8, 2048²: share of conifers in the forest (0 broadleaf, 255 conifers):
    montane belt, Scottish pinewoods, Mediterranean pines, named forests.
    For the forest renderer (lot V4); the terrain shader does not need it.
``wetlands.png``
    RGB8, 4096²: R marsh (reed beds, open water), G pond country (medieval
    fish ponds), B wet meadows and bogs, from ``wetlands.json`` (soft, noisy
    edges, cut above ``max_height_m``) plus wet meadows along the rivers on flat
    valley floors.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from rasterio.features import rasterize
from scipy import ndimage
from shapely.geometry import LineString, Polygon

from cent_ans_tools.geo import download, kk10, splat, terrain
from cent_ans_tools.geo.project import MapGrid
from cent_ans_tools.geo.provinces import decode_ids

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"
FORESTS_FILE = "historical_forests.json"
WETLANDS_FILE = "wetlands.json"
FOREST_KIND_SIZE = 2048
SEED = 1340

#: Conifer share by named-forest kind.
CONIFER_BY_KIND = {"broadleaf": 0.05, "mixed": 0.35, "conifer": 0.85, "heath": 0.3}
WETLAND_CHANNEL = {"marsh": 0, "ponds": 1, "wet_meadow": 2}
#: Soft edge half-width and noise amplitude of the curated areas (pixels).
AREA_EDGE_PX = 1.2
AREA_NOISE_PX = 2.5
#: Share of the smaller radius added as edge wobble (named areas must not read as ellipses).
AREA_NOISE_SHARE = 0.35
KK10_SMOOTH_PX = 6.0


@dataclass(frozen=True)
class LandcoverResult:
    """Paths written by :func:`build` and a few statistics."""

    splat: Path
    forest_kind: Path
    wetlands: Path
    forest_share: float
    cleared_share: float


def smoothstep(edge0: float, edge1: float, x: np.ndarray) -> np.ndarray:
    """GLSL-style smoothstep."""
    return splat.smoothstep(edge0, edge1, x)


def uniform_noise(
    shape: tuple[int, int],
    rng: np.random.Generator,
    base_cells: int = 40,
    octaves: int = 6,
) -> np.ndarray:
    """Fractal noise remapped to a uniform ``[0, 1]`` marginal (rank transform)."""
    noise = splat.fractal_noise(shape, base_cells, octaves, rng)
    order = np.argsort(noise, axis=None, kind="stable")
    ranks = np.empty(noise.size, dtype=np.float32)
    ranks[order] = np.linspace(0.0, 1.0, noise.size, dtype=np.float32)
    return ranks.reshape(shape)


def rank_uniform(values: np.ndarray, mask: np.ndarray) -> np.ndarray:
    """Values remapped to their rank in ``[0, 1]`` among ``mask`` pixels (0 elsewhere)."""
    out = np.zeros(values.shape, dtype=np.float32)
    selected = values[mask]
    if selected.size == 0:
        return out
    order = np.argsort(selected, kind="stable")
    ranks = np.empty(selected.size, dtype=np.float32)
    ranks[order] = np.linspace(0.0, 1.0, selected.size, dtype=np.float32)
    out[mask] = ranks
    return out


def kk10_cleared(grid: MapGrid, land_use: kk10.LandUse | None) -> np.ndarray:
    """KK10 cleared share on ``grid`` (bilinear on the 5′ cells, gaps filled by the nearest cell)."""
    size = grid.size_px
    if land_use is None:
        return np.full((size, size), 0.6, dtype=np.float32)
    fraction = land_use.fraction.copy()
    lat = land_use.lat
    if land_use.lat_descending:
        fraction = fraction[::-1]
        lat = lat[::-1]
    missing = np.isnan(fraction)
    if missing.any() and (~missing).any():
        nearest = ndimage.distance_transform_edt(
            missing, return_distances=False, return_indices=True
        )
        fraction = fraction[nearest[0], nearest[1]]
    centres = np.arange(size, dtype=np.float64) + 0.5
    px, py = np.meshgrid(centres, centres)
    lon_px, lat_px = grid.pixel_to_lonlat(px.ravel(), py.ravel())
    lon_step = float(land_use.lon[1] - land_use.lon[0])
    lat_step = float(lat[1] - lat[0])
    cols = (np.asarray(lon_px) - land_use.lon[0]) / lon_step
    rows = (np.asarray(lat_px) - lat[0]) / lat_step
    values = ndimage.map_coordinates(fraction, [rows, cols], order=1, mode="nearest")
    # KK10 répartit l'usage du sol cellule par cellule (bruit poivre et sel) : on n'en garde
    # que la tendance régionale (≈ une cellule de 5′).
    values = ndimage.gaussian_filter(values.reshape(size, size), KK10_SMOOTH_PX)
    return np.clip(values, 0.0, 1.0).astype(np.float32)


def area_signed_distance(
    area: dict, grid: MapGrid
) -> tuple[np.ndarray, tuple[slice, slice]]:
    """Signed distance (pixels, positive inside) of an ellipse or polygon, on its bounding window."""
    size = grid.size_px
    mpp = grid.meters_per_px
    margin = 12
    if "ellipse" in area:
        ellipse = area["ellipse"]
        cx, cy = grid.lonlat_to_pixel(*ellipse["center"])
        cx, cy = float(cx), float(cy)
        a_px = ellipse["radii_km"][0] * 1000.0 / mpp
        b_px = ellipse["radii_km"][1] * 1000.0 / mpp
        reach = max(a_px, b_px) + margin
        r0, r1 = max(int(cy - reach), 0), min(int(cy + reach) + 1, size)
        c0, c1 = max(int(cx - reach), 0), min(int(cx + reach) + 1, size)
        rows, cols = np.mgrid[r0:r1, c0:c1].astype(np.float32) + 0.5
        dx = cols - cx
        dy_north = -(rows - cy)
        angle = np.radians(ellipse.get("angle_deg", 0.0))
        u = dx * np.cos(angle) + dy_north * np.sin(angle)
        v = -dx * np.sin(angle) + dy_north * np.cos(angle)
        r = np.sqrt((u / a_px) ** 2 + (v / b_px) ** 2)
        return ((1.0 - r) * min(a_px, b_px)).astype(np.float32), (
            slice(r0, r1),
            slice(c0, c1),
        )
    lon, lat = zip(*area["polygon"], strict=True)
    px, py = grid.lonlat_to_pixel(np.array(lon), np.array(lat))
    r0 = max(int(np.min(py)) - margin, 0)
    r1 = min(int(np.max(py)) + margin + 1, size)
    c0 = max(int(np.min(px)) - margin, 0)
    c1 = min(int(np.max(px)) + margin + 1, size)
    shape = Polygon(zip(np.asarray(px) - c0, np.asarray(py) - r0, strict=True))
    inside = (
        rasterize([(shape, 1)], out_shape=(r1 - r0, c1 - c0), fill=0, dtype=np.uint8)
        > 0
    )
    dist = ndimage.distance_transform_edt(inside) - ndimage.distance_transform_edt(
        ~inside
    )
    return dist.astype(np.float32), (slice(r0, r1), slice(c0, c1))


def area_mask(
    area: dict, grid: MapGrid, noise: np.ndarray
) -> tuple[np.ndarray, tuple[slice, slice]]:
    """Soft 0-1 mask of an area with a noisy, natural edge (on its bounding window)."""
    dist, window = area_signed_distance(area, grid)
    amplitude = max(AREA_NOISE_PX, AREA_NOISE_SHARE * area_min_radius_px(area, grid))
    wobble = (noise[window] - 0.5) * 2.0 * amplitude
    return smoothstep(-AREA_EDGE_PX, AREA_EDGE_PX, dist + wobble), window


def area_min_radius_px(area: dict, grid: MapGrid) -> float:
    """Smaller radius of an ellipse (pixels); a polygon counts as its inscribed size."""
    if "ellipse" in area:
        return min(area["ellipse"]["radii_km"]) * 1000.0 / grid.meters_per_px
    dist, _ = area_signed_distance(area, grid)
    return float(dist.max())


def load_areas(path: Path) -> list[dict]:
    """``areas`` of a curated file (empty if the file is missing)."""
    if not path.exists():
        return []
    return list(json.loads(path.read_text(encoding="utf-8"))["areas"])


def points_px(map_dir: Path) -> tuple[np.ndarray, np.ndarray]:
    """Map pixel positions of the settlements (towns, abbeys) and of the hamlets."""
    towns = np.zeros((0, 2), dtype=np.float32)
    hamlets = np.zeros((0, 2), dtype=np.float32)
    path = map_dir / "settlements_px.json"
    if path.exists():
        towns = np.array(list(json.loads(path.read_text()).values()), dtype=np.float32)
    path = map_dir / "hamlets.json"
    if path.exists():
        hamlets = np.array(
            [h["px"] for h in json.loads(path.read_text())], dtype=np.float32
        )
    return towns, hamlets


def distance_to_points(points: np.ndarray, size: int) -> np.ndarray:
    """Euclidean distance (pixels) to the nearest point."""
    seeds = np.ones((size, size), dtype=bool)
    if points.size:
        cols = np.clip(np.rint(points[:, 0]).astype(int), 0, size - 1)
        rows = np.clip(np.rint(points[:, 1]).astype(int), 0, size - 1)
        seeds[rows, cols] = False
    else:
        return np.full((size, size), 1e4, dtype=np.float32)
    return ndimage.distance_transform_edt(seeds).astype(np.float32)


def river_distance(map_dir: Path, size: int, min_importance: int = 3) -> np.ndarray:
    """Distance (pixels) to the rivers of ``rivers.geojson`` of at least ``min_importance``."""
    path = map_dir / "rivers.geojson"
    if not path.exists():
        return np.full((size, size), 1e4, dtype=np.float32)
    shapes = []
    for feature in json.loads(path.read_text(encoding="utf-8"))["features"]:
        props = feature.get("properties", {})
        if "strahler" in props:
            importance = int(props["strahler"])
        else:
            importance = 12 - int(props.get("scalerank", 10))
        if importance < min_importance:
            continue
        geometry = feature["geometry"]
        parts = (
            [geometry["coordinates"]]
            if geometry["type"] == "LineString"
            else geometry["coordinates"]
        )
        shapes.extend((LineString(part), 1) for part in parts if len(part) >= 2)
    if not shapes:
        return np.full((size, size), 1e4, dtype=np.float32)
    lines = rasterize(
        shapes, out_shape=(size, size), fill=0, dtype=np.uint8, all_touched=True
    )
    return ndimage.distance_transform_edt(lines == 0).astype(np.float32)


def tree_line_m(lat: np.ndarray) -> np.ndarray:
    """Approximate natural tree line: ≈ 2 000 m in the Alps/Pyrenees, ≈ 600 m in Scotland."""
    return np.clip(2000.0 - (lat - 44.0) * 110.0, 550.0, 2150.0)


def pixel_lonlat(grid: MapGrid) -> tuple[np.ndarray, np.ndarray]:
    """Longitude and latitude of every pixel centre."""
    size = grid.size_px
    centres = np.arange(size, dtype=np.float64) + 0.5
    px, py = np.meshgrid(centres, centres)
    lon, lat = grid.pixel_to_lonlat(px.ravel(), py.ravel())
    return (
        np.asarray(lon, dtype=np.float32).reshape(size, size),
        np.asarray(lat, dtype=np.float32).reshape(size, size),
    )


def build_wetlands(
    grid: MapGrid,
    areas: list[dict],
    height_m: np.ndarray,
    land: np.ndarray,
    slope: np.ndarray,
    river_dist: np.ndarray,
    noise: np.ndarray,
) -> np.ndarray:
    """``(rows, cols, 3)`` float wetland intensities (R marsh, G ponds, B wet meadows)."""
    out = np.zeros((*height_m.shape, 3), dtype=np.float32)
    for area in areas:
        mask, window = area_mask(area, grid, noise)
        mask = mask * float(area["density"])
        if "max_height_m" in area:
            limit = float(area["max_height_m"])
            mask = mask * smoothstep(limit + 15.0, limit, height_m[window])
        channel = WETLAND_CHANNEL[area["kind"]]
        out[window][..., channel] = np.maximum(out[window][..., channel], mask)
    # Prés humides des fonds de vallée : près des rivières, sur le plat.
    valley = np.exp(-((river_dist / 1.6) ** 2)) * smoothstep(0.02, 0.004, slope)
    valley = valley * smoothstep(900.0, 300.0, height_m) * 0.45
    out[..., 2] = np.maximum(out[..., 2], valley)
    out[~land] = 0.0
    return out


def compute_forest(
    grid: MapGrid,
    height_m: np.ndarray,
    land: np.ndarray,
    ids: np.ndarray,
    terrains: dict[int, str],
    cleared: np.ndarray,
    forests: list[dict],
    wet: np.ndarray,
    towns: np.ndarray,
    hamlets: np.ndarray,
    river_dist: np.ndarray,
    rng: np.random.Generator,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Forest cover (0-1, anti-aliased), heath boost (0-1) and conifer share (0-1)."""
    size = grid.size_px
    mpp = grid.meters_per_px
    lon, lat = pixel_lonlat(grid)
    h = np.maximum(height_m, 0.0)
    smooth = ndimage.gaussian_filter(height_m, 1.0)
    grad_y, grad_x = np.gradient(smooth, mpp)
    slope = np.hypot(grad_x, grad_y)
    local = height_m - ndimage.gaussian_filter(height_m, 6.0)
    coast = ndimage.distance_transform_edt(land)

    # Potentiel forestier.
    potential = smoothstep(tree_line_m(lat) + 150.0, tree_line_m(lat) - 150.0, h)
    max_index = int(ids.max()) if ids.size else 0
    province_factor = np.ones(max_index + 1, dtype=np.float32)
    for index, name in terrains.items():
        if 0 <= index <= max_index:
            province_factor[index] = {"heath": 0.55, "marsh": 0.6}.get(name, 1.0)
    potential = potential * splat.masked_blur(province_factor[ids], land, 8.0)
    potential = potential * (
        0.45 + 0.55 * smoothstep(0.5, 2.5, coast)
    )  # dunes, prés salés
    potential = potential * (1.0 - 0.85 * np.maximum(wet[..., 0], wet[..., 1] * 0.5))
    potential = potential * (1.0 - 0.6 * wet[..., 2])
    # Hautes terres océaniques (îles Britanniques) : landes et pâtures d'altitude dès 300 m.
    oceanic = smoothstep(49.8, 50.8, lat) * smoothstep(2.2, 0.8, lon)
    potential = potential * (1.0 - 0.75 * oceanic * smoothstep(220.0, 420.0, h))
    # Garrigue méditerranéenne : forêt claire.
    potential = potential * (
        1.0 - 0.3 * smoothstep(44.5, 43.0, lat) * smoothstep(800.0, 200.0, h)
    )

    target = potential * (1.0 - cleared)
    edge_noise = uniform_noise((size, size), rng, base_cells=220, octaves=4)
    heath = np.zeros((size, size), dtype=np.float32)
    conifer = 0.03 + 0.8 * smoothstep(750.0, 1350.0, h)
    conifer = np.maximum(
        conifer, 0.8 * smoothstep(56.2, 56.8, lat) * smoothstep(100.0, 250.0, h)
    )
    conifer = np.maximum(conifer, 0.45 * smoothstep(44.0, 42.5, lat))
    for area in forests:
        mask, window = area_mask(area, grid, edge_noise)
        density = float(area["density"])
        if area["kind"] == "heath":
            heath[window] = np.maximum(heath[window], mask * density)
            target[window] = target[window] * (1.0 - 0.75 * mask * density)
        else:
            boosted = np.maximum(target[window], density * potential[window] ** 0.5)
            target[window] = target[window] + (boosted - target[window]) * mask
        share = CONIFER_BY_KIND[area["kind"]]
        conifer[window] = (
            conifer[window]
            + (np.maximum(conifer[window], share) - conifer[window]) * mask
        )

    # Préférence de terrain : pentes, crêtes, sols pauvres d'altitude ; défrichés autour des
    # lieux habités et dans les plaines inondables (prés).
    town_d = distance_to_points(towns, size)
    hamlet_d = distance_to_points(hamlets, size)
    near_settle = np.maximum(
        np.exp(-((town_d / 3.0) ** 2)), 0.8 * np.exp(-((hamlet_d / 1.6) ** 2))
    )
    near_river = np.exp(-((river_dist / 1.3) ** 2))
    # Essarts : terroir défriché autour des villes (≈ 3 km) et des villages.
    target = target * (1.0 - 0.9 * np.exp(-((town_d / 4.0) ** 2)))
    target = target * (1.0 - 0.5 * np.exp(-((hamlet_d / 1.5) ** 2)))
    preference = (
        0.2
        + 0.35 * smoothstep(0.015, 0.09, slope)
        + 0.25 * smoothstep(-30.0, 60.0, local)
        + 0.2 * smoothstep(150.0, 700.0, h)
        - 0.55 * near_settle
        - 0.25 * near_river
    )
    # Massifs de 5 à 30 km : bruit assez fin (≈ 25 px) ; la tendance régionale vient de KK10.
    noise = uniform_noise((size, size), rng, base_cells=160, octaves=4)
    score = rank_uniform(0.62 * noise + 0.38 * np.clip(preference, 0.0, 1.0), land)
    forest = (score > 1.0 - np.clip(target, 0.0, 1.0)) & land
    # Bosquets isolés supprimés, petites trouées bouchées : des massifs lisibles.
    forest = ndimage.binary_opening(forest, iterations=1)
    forest = ndimage.binary_closing(forest, iterations=1) & land
    cover = ndimage.gaussian_filter(forest.astype(np.float32), 0.7)
    cover[~land] = 0.0
    return cover, heath, np.clip(conifer, 0.0, 1.0)


def compose_splat(
    base: np.ndarray, forest: np.ndarray, heath: np.ndarray, wet: np.ndarray
) -> np.ndarray:
    """New weights: ``base`` R/G/A scaled to the open share, B = ``forest``; heaths and marshes."""
    grass = base[..., 0] * (1.0 + 1.5 * wet[..., 2] + 1.0 * wet[..., 0])
    farm = base[..., 1] * (1.0 - 0.7 * np.maximum(wet[..., 0], wet[..., 2]))
    rock = base[..., 3] + 1.2 * heath
    grass = grass * (1.0 - 0.5 * heath)
    farm = farm * (1.0 - 0.8 * heath)
    open_mix = np.stack([grass, farm, rock], axis=-1)
    total = open_mix.sum(axis=-1, keepdims=True)
    open_mix = np.where(
        total > 1e-6, open_mix / np.maximum(total, 1e-6), [1.0, 0.0, 0.0]
    )
    open_share = (1.0 - forest)[..., None]
    weights = np.empty(base.shape, dtype=np.float32)
    weights[..., 0] = open_mix[..., 0] * open_share[..., 0]
    weights[..., 1] = open_mix[..., 1] * open_share[..., 0]
    weights[..., 2] = forest
    weights[..., 3] = open_mix[..., 2] * open_share[..., 0]
    land = base.sum(axis=-1) > 0.5
    weights[~land] = 0.0
    return weights


def build(
    map_dir: Path = MAP_DIR, provinces_dir: Path = PROVINCES_DIR
) -> LandcoverResult:
    """Write ``splat.png``, ``forest_kind.png`` and ``wetlands.png``."""
    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    grid = MapGrid(tuple(meta["bounds_projected"]), int(meta["size_px"][0]))
    height = terrain.uint16_to_height(
        terrain.read_png16(map_dir / "heightmap.png")
    ).astype(np.float32)
    with Image.open(map_dir / "land_mask.png") as image:
        land_mask = np.asarray(image.convert("L")) > 127
    with Image.open(map_dir / "province_ids.png") as image:
        ids = decode_ids(np.asarray(image.convert("RGB")))
    terrains = splat.province_terrains(map_dir / "provinces.geojson", provinces_dir)
    land = land_mask & (height > 0.0)
    rng = np.random.default_rng(SEED)
    size = grid.size_px
    mpp = grid.meters_per_px

    grad_y, grad_x = np.gradient(ndimage.gaussian_filter(height, 1.0), mpp)
    slope = np.hypot(grad_x, grad_y)
    river_d = river_distance(map_dir, size)
    area_noise = uniform_noise((size, size), rng, base_cells=220, octaves=4)
    wet = build_wetlands(
        grid,
        load_areas(map_dir / WETLANDS_FILE),
        height,
        land,
        slope,
        river_d,
        area_noise,
    )
    cleared = kk10_cleared(grid, kk10.load())
    towns, hamlets = points_px(map_dir)
    forest, heath, conifer = compute_forest(
        grid,
        height,
        land,
        ids,
        terrains,
        cleared,
        load_areas(map_dir / FORESTS_FILE),
        wet,
        towns,
        hamlets,
        river_d,
        rng,
    )
    base = splat.compute_splat(height, land, ids, terrains, mpp)
    weights = compose_splat(base, forest, heath, wet)

    splat_path = map_dir / "splat.png"
    Image.fromarray(splat.encode_splat(weights), mode="RGBA").save(
        splat_path, compress_level=9
    )
    wet_path = map_dir / "wetlands.png"
    Image.fromarray(
        np.clip(np.rint(wet * 255.0), 0, 255).astype(np.uint8), mode="RGB"
    ).save(wet_path, compress_level=9)
    factor = max(1, size // FOREST_KIND_SIZE)
    kind = splat.downsample_mean(np.where(land, conifer, 0.0), factor)
    kind_path = map_dir / "forest_kind.png"
    terrain.write_png8(
        np.clip(np.rint(kind * 255.0), 0, 255).astype(np.uint8), kind_path
    )
    return LandcoverResult(
        splat=splat_path,
        forest_kind=kind_path,
        wetlands=wet_path,
        forest_share=float(forest[land].mean()) if land.any() else 0.0,
        cleared_share=float(cleared[land].mean()) if land.any() else 0.0,
    )
