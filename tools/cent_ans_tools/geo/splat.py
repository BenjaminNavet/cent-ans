"""Offline rasters for the campaign terrain shader.

Three files are written next to the heightmap in ``data/map/``:

``splat.png``
    RGBA8, 2048², material weights normalised to 255 on land (0 at sea):
    R = grassland, G = farmland, B = forest, A = rock / heath. Built from the
    dominant terrain of each province (``data/provinces/*.json``), altitude,
    slope, distance to the coast and fractal noise.
``province_border_dist.png``
    L8, full resolution: distance (in map pixels, ``value / BORDER_DIST_SCALE``)
    to the nearest land border between two provinces, lightly smoothed so the
    shader can draw anti-aliased borders with ``smoothstep``.
``coast_dist.png``
    L8, full resolution: signed distance to the shore (sea level or lakes),
    ``(value - 128) / COAST_DIST_SCALE`` pixels, positive on land.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.geo import download, terrain
from cent_ans_tools.geo.provinces import decode_ids

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"

SPLAT_SIZE = 2048
BORDER_DIST_SCALE = 8.0  # 1 px = 8 levels, range 0..~32 px
BORDER_SMOOTH_SIGMA = 0.8
COAST_DIST_SCALE = 2.0  # 1 px = 2 levels, range ±64 px
NOISE_SEED = 1337

# Mélange de base (prairie, cultures, forêt, roche/lande) par terrain dominant de province.
TERRAIN_MIX: dict[str, tuple[float, float, float, float]] = {
    "plains": (0.28, 0.62, 0.10, 0.00),
    "bocage": (0.50, 0.25, 0.25, 0.00),
    "forest": (0.22, 0.10, 0.68, 0.00),
    "hills": (0.38, 0.27, 0.32, 0.03),
    "mountains": (0.28, 0.05, 0.42, 0.25),
    "heath": (0.42, 0.05, 0.13, 0.40),
    "marsh": (0.62, 0.10, 0.23, 0.05),
}
DEFAULT_MIX = TERRAIN_MIX["hills"]


@dataclass(frozen=True)
class SplatResult:
    """Paths written by :func:`build`."""

    splat: Path
    border_dist: Path
    coast_dist: Path


def smoothstep(edge0: float, edge1: float, x: np.ndarray) -> np.ndarray:
    """GLSL-style smoothstep (works for ``edge0 > edge1`` too)."""
    t = np.clip((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def fractal_noise(
    shape: tuple[int, int], base_cells: int, octaves: int, rng: np.random.Generator
) -> np.ndarray:
    """Smooth value noise in about ``[-1, 1]``: cubic-upsampled random grids summed.

    Args:
        shape: Output ``(rows, cols)``.
        base_cells: Grid cells along the largest side for the first octave.
        octaves: Number of octaves (each doubles the frequency, halves the amplitude).
        rng: Random generator (deterministic output for a given seed).
    """
    total = np.zeros(shape, dtype=np.float32)
    amplitude = 1.0
    norm = 0.0
    cells = base_cells
    for _ in range(octaves):
        grid = rng.standard_normal((cells + 3, cells + 3)).astype(np.float32)
        zoom = (shape[0] / cells, shape[1] / cells)
        up = ndimage.zoom(grid, zoom, order=3, mode="reflect")
        offset = (int(zoom[0]), int(zoom[1]))
        total += (
            amplitude
            * up[offset[0] : offset[0] + shape[0], offset[1] : offset[1] + shape[1]]
        )
        norm += amplitude
        amplitude *= 0.5
        cells *= 2
    return np.clip(total / norm, -2.0, 2.0) * 0.8


def downsample_mean(array: np.ndarray, factor: int) -> np.ndarray:
    """Block average by an integer ``factor``."""
    if factor == 1:
        return array.astype(np.float32)
    rows, cols = array.shape[0] // factor, array.shape[1] // factor
    trimmed = array[: rows * factor, : cols * factor].astype(np.float32)
    return trimmed.reshape(rows, factor, cols, factor).mean(axis=(1, 3))


def province_terrains(
    geojson_path: Path, provinces_dir: Path = PROVINCES_DIR
) -> dict[int, str]:
    """Raster index → dominant terrain (``data/provinces/<id>.json`` ``terrain`` field)."""
    with geojson_path.open(encoding="utf-8") as handle:
        features = json.load(handle)["features"]
    result: dict[int, str] = {}
    for position, feature in enumerate(features, start=1):
        props = feature.get("properties", {})
        index = int(props.get("index", position))
        path = provinces_dir / f"{props.get('id', '')}.json"
        if path.exists():
            with path.open(encoding="utf-8") as handle:
                result[index] = str(json.load(handle).get("terrain", ""))
    return result


def masked_blur(values: np.ndarray, mask: np.ndarray, sigma: float) -> np.ndarray:
    """Gaussian blur that ignores pixels outside ``mask`` (no bleeding of zeros)."""
    weight = ndimage.gaussian_filter(mask.astype(np.float32), sigma)
    blurred = ndimage.gaussian_filter(values * mask, sigma)
    return np.where(weight > 1e-4, blurred / np.maximum(weight, 1e-4), values)


def compute_splat(
    height_m: np.ndarray,
    land: np.ndarray,
    ids: np.ndarray,
    terrains: dict[int, str],
    meters_per_px: float,
    seed: int = NOISE_SEED,
) -> np.ndarray:
    """Material weights ``(rows, cols, 4)`` in ``[0, 1]``, summing to 1 on land.

    All inputs share the output resolution. ``meters_per_px`` is the size of
    one output pixel (used for slopes).
    """
    shape = height_m.shape
    rng = np.random.default_rng(seed)
    max_index = int(ids.max()) if ids.size else 0
    table = np.tile(np.array(DEFAULT_MIX, dtype=np.float32), (max_index + 1, 1))
    for index, name in terrains.items():
        if 0 <= index <= max_index:
            table[index] = TERRAIN_MIX.get(name, DEFAULT_MIX)
    base = table[ids]
    # Fondu entre provinces (~20 px) : pas de marche de matériau le long des frontières.
    for channel in range(4):
        base[..., channel] = masked_blur(base[..., channel], land, 10.0)

    grad_y, grad_x = np.gradient(ndimage.gaussian_filter(height_m, 1.0), meters_per_px)
    slope = np.hypot(grad_x, grad_y)
    relief = height_m - ndimage.gaussian_filter(height_m, 12.0)
    coast = ndimage.distance_transform_edt(land)
    h = np.maximum(height_m, 0.0)

    grass, farm, forest, rock = (base[..., i] for i in range(4))
    # Cultures : plaines basses, pentes faibles, un peu reculées de la côte.
    farm = farm * smoothstep(850.0, 250.0, h) * smoothstep(0.09, 0.02, slope)
    farm = farm * (0.55 + 0.45 * smoothstep(0.5, 3.0, coast))
    # Forêts : reliefs moyens, crêtes et versants ; limite des arbres vers 1 800 m.
    forest = (
        forest
        * (0.55 + 0.9 * smoothstep(150.0, 700.0, h))
        * smoothstep(2000.0, 1500.0, h)
    )
    forest = forest * (
        1.0 + 0.8 * smoothstep(0.0, 120.0, relief) + 1.2 * smoothstep(0.03, 0.12, slope)
    )
    # Roche et lande : haute montagne, pentes fortes ; alpages entre les deux.
    alpine = smoothstep(1500.0, 2300.0, h)
    rock = rock + 2.5 * alpine + 1.5 * smoothstep(0.18, 0.40, slope)
    grass = grass * (1.0 - 0.5 * alpine) + 0.35 * smoothstep(
        1400.0, 1900.0, h
    ) * smoothstep(2600.0, 2000.0, h)
    grass = grass + 0.25 * smoothstep(3.0, 0.5, coast)

    # Bruit fractal : taches de bois, de landes et de cultures à plusieurs échelles.
    weights = np.stack([grass, farm, forest, rock], axis=-1).astype(np.float32)
    strengths = (1.2, 1.5, 1.8, 1.4)
    for channel, strength in enumerate(strengths):
        noise = fractal_noise(shape, 48, 5, rng)
        weights[..., channel] *= np.exp(strength * noise)
    weights = np.maximum(weights, 0.0) ** 1.6  # contraste : zones plus franches
    total = weights.sum(axis=-1, keepdims=True)
    weights = np.where(total > 1e-6, weights / np.maximum(total, 1e-6), 0.0)
    weights[~land] = 0.0
    return weights


def encode_splat(weights: np.ndarray) -> np.ndarray:
    """Quantise weights to ``uint8`` so that each land pixel sums to 255."""
    scaled = weights * 255.0
    quantised = np.floor(scaled).astype(np.int32)
    remainder = 255 - quantised.sum(axis=-1)
    land = weights.sum(axis=-1) > 0.5
    largest = np.argmax(scaled - quantised, axis=-1)
    rows, cols = np.nonzero(land)
    quantised[rows, cols, largest[rows, cols]] += remainder[rows, cols]
    return np.clip(quantised, 0, 255).astype(np.uint8)


def border_pixels(ids: np.ndarray, land: np.ndarray) -> np.ndarray:
    """Pixels on a land border between two different provinces (both sides marked)."""
    border = np.zeros(ids.shape, dtype=bool)
    for axis in (0, 1):
        a = ids.take(range(0, ids.shape[axis] - 1), axis=axis)
        b = ids.take(range(1, ids.shape[axis]), axis=axis)
        la = land.take(range(0, ids.shape[axis] - 1), axis=axis)
        lb = land.take(range(1, ids.shape[axis]), axis=axis)
        diff = (a != b) & la & lb & ((a > 0) | (b > 0))
        if axis == 0:
            border[:-1, :] |= diff
            border[1:, :] |= diff
        else:
            border[:, :-1] |= diff
            border[:, 1:] |= diff
    return border


def compute_border_dist(ids: np.ndarray, land: np.ndarray) -> np.ndarray:
    """Distance (px) to the nearest province border, 0.5 on border pixels, smoothed."""
    border = border_pixels(ids, land)
    if not border.any():
        return np.full(ids.shape, 255.0 / BORDER_DIST_SCALE, dtype=np.float32)
    dist = ndimage.distance_transform_edt(~border).astype(np.float32) + 0.5
    return ndimage.gaussian_filter(dist, BORDER_SMOOTH_SIGMA)


def encode_border_dist(dist: np.ndarray) -> np.ndarray:
    """``uint8`` encoding of :func:`compute_border_dist` (``BORDER_DIST_SCALE`` levels/px)."""
    return np.clip(np.rint(dist * BORDER_DIST_SCALE), 0, 255).astype(np.uint8)


def compute_coast_dist(water: np.ndarray) -> np.ndarray:
    """Signed distance (px) to the shore: positive on land, negative on water."""
    if water.all() or not water.any():
        return np.where(water, -64.0, 64.0).astype(np.float32)
    inside = ndimage.distance_transform_edt(~water)
    outside = ndimage.distance_transform_edt(water)
    return (inside - outside).astype(np.float32)


def encode_coast_dist(signed: np.ndarray) -> np.ndarray:
    """``uint8`` encoding: 128 = shore, ``COAST_DIST_SCALE`` levels per pixel."""
    return np.clip(np.rint(128.0 + signed * COAST_DIST_SCALE), 0, 255).astype(np.uint8)


def build(map_dir: Path = MAP_DIR, provinces_dir: Path = PROVINCES_DIR) -> SplatResult:
    """Read the map rasters, write ``splat.png``, ``province_border_dist.png``, ``coast_dist.png``."""
    with (map_dir / "map.json").open(encoding="utf-8") as handle:
        meta = json.load(handle)
    height = terrain.uint16_to_height(
        terrain.read_png16(map_dir / "heightmap.png")
    ).astype(np.float32)
    with Image.open(map_dir / "land_mask.png") as image:
        land_full = np.asarray(image.convert("L")) > 127
    with Image.open(map_dir / "province_ids.png") as image:
        ids_full = decode_ids(np.asarray(image.convert("RGB")))
    terrains = province_terrains(map_dir / "provinces.geojson", provinces_dir)

    # Eau = mer (altitude ≤ 0, comme le plan d'eau du jeu) ou lac (masque de terre).
    water = (height <= 0.0) | ~land_full
    coast_path = map_dir / "coast_dist.png"
    terrain.write_png8(encode_coast_dist(compute_coast_dist(water)), coast_path)

    border_path = map_dir / "province_border_dist.png"
    terrain.write_png8(
        encode_border_dist(compute_border_dist(ids_full, land_full)), border_path
    )

    factor = max(1, height.shape[0] // SPLAT_SIZE)
    height_small = downsample_mean(height, factor)
    land_small = downsample_mean(~water, factor) > 0.5
    ids_small = ids_full[::factor, ::factor]
    weights = compute_splat(
        height_small,
        land_small,
        ids_small,
        terrains,
        float(meta.get("meters_per_px", 800.0)) * factor,
    )
    splat_path = map_dir / "splat.png"
    Image.fromarray(encode_splat(weights), mode="RGBA").save(
        splat_path, compress_level=9
    )
    return SplatResult(splat=splat_path, border_dist=border_path, coast_dist=coast_path)
