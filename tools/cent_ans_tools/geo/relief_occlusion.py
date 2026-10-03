"""Valley occlusion (signed sky openness) of the campaign map, lot RV-D.

The campaign relief used to read like an IGN hillshade: thin directional shadows
and no sense of the large landforms. This step bakes a low-frequency, sun
independent occlusion map so the terrain shader can darken and cool valley
floors and basins and lift crests and plateaus.

Method (horizon-based ambient occlusion, multi-scale):

* the display heightmap (``heightmap_render.png``, else ``heightmap.png``) is
  block-averaged to half resolution, sea clamped to 0 m;
* for each scale band (radius ``R`` of a few km to ~20 km) the heights are
  smoothed with a Gaussian of ``R / 6`` (no fine detail), then for ``N``
  directions the horizon tangent is the maximum, over geometric sample
  distances in ``[R / 4, R]``, of ``Δh · exaggeration / distance``;
* for each direction the highest and lowest sight lines over those samples
  are averaged (``(sin max + sin min) / 2``): a uniform slope cancels out, so
  rugged massifs carry no overall bias; the band value is the mean over
  directions: positive in a valley (the horizon rises around it), negative on
  a crest or a summit (the horizon falls away), 0 on a plain or a regular
  slope; bands are averaged, smoothed, then compressed by ``|x| ** 0.75``.

Output ``data/map/relief_occlusion.png``: L8, half the map grid (one texel =
2 x 2 map pixels, like ``splat.png``), ``value = 128 + 127 · clip(o · gain)``
where ``gain`` maps the 99.5th percentile of ``|o|`` on land to 1. Above 128 =
occluded (valley), below = open (crest); 128 at sea.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.geo import download, terrain

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
OUTPUT_NAME = "relief_occlusion.png"

OCCLUSION_FACTOR = 2  # map pixels per occlusion texel (per side)
DEFAULT_RADII_M = (6_000.0, 12_000.0, 25_000.0)  # scale bands (horizon search radius)
DEFAULT_DIRECTIONS = 16
DEFAULT_EXAGGERATION = (
    3.0  # between the strategic (x4.3) and close (x1.5) display scales
)
SAMPLES_PER_BAND = 4
NORMALISE_PERCENTILE = 99.5
FINAL_SMOOTH_SIGMA = 1.5  # texels
RESPONSE_GAMMA = 0.75  # < 1 lifts hill country against the Alps


@dataclass(frozen=True)
class OcclusionResult:
    """Paths and statistics of a baked occlusion map."""

    path: Path
    size: tuple[int, int]
    gain: float
    bytes: int


def _shifted(
    padded: np.ndarray, pad: int, dy: int, dx: int, shape: tuple[int, int]
) -> np.ndarray:
    """View of ``padded`` (edge padded by ``pad``) offset by ``(dy, dx)`` pixels."""
    rows, cols = shape
    return padded[pad + dy : pad + dy + rows, pad + dx : pad + dx + cols]


def band_openness(
    height_m: np.ndarray,
    meters_per_px: float,
    radius_m: float,
    directions: int = DEFAULT_DIRECTIONS,
    exaggeration: float = DEFAULT_EXAGGERATION,
    smooth: bool = True,
) -> np.ndarray:
    """Signed horizon openness of one scale band.

    Args:
        height_m: Elevation grid in metres.
        meters_per_px: Ground size of one grid pixel.
        radius_m: Horizon search radius of the band.
        directions: Number of azimuths sampled.
        exaggeration: Vertical exaggeration applied to height differences.
        smooth: Pre-smooth the heights with a Gaussian of ``radius / 6``.

    Returns:
        ``float32`` grid: mean over azimuths of the mean of the highest and
        lowest sight line sines, > 0 in
        valleys, < 0 on crests, 0 on a plain.
    """
    radius_px = max(radius_m / meters_per_px, 1.0)
    heights = np.asarray(height_m, dtype=np.float32)
    if smooth and radius_px / 6.0 > 0.3:
        heights = ndimage.gaussian_filter(heights, radius_px / 6.0, mode="nearest")
    near = max(radius_px / 4.0, 1.0)
    distances = np.unique(np.geomspace(near, radius_px, SAMPLES_PER_BAND))
    pad = int(np.ceil(radius_px)) + 1
    padded = np.pad(heights, pad, mode="edge")
    total = np.zeros(heights.shape, dtype=np.float32)
    for k in range(directions):
        azimuth = 2.0 * np.pi * k / directions
        ux, uy = np.cos(azimuth), np.sin(azimuth)
        best = np.full(heights.shape, -np.inf, dtype=np.float32)
        worst = np.full(heights.shape, np.inf, dtype=np.float32)
        for distance in distances:
            dx = int(round(ux * distance))
            dy = int(round(uy * distance))
            if dx == 0 and dy == 0:
                continue
            ground = float(np.hypot(dx, dy)) * meters_per_px
            rise = _shifted(padded, pad, dy, dx, heights.shape) - heights
            tangent = rise * np.float32(exaggeration / ground)
            np.maximum(best, tangent, out=best)
            np.minimum(worst, tangent, out=worst)
        best[~np.isfinite(best)] = 0.0
        worst[~np.isfinite(worst)] = 0.0
        # Highest and lowest sight line, sin(arctan(t)): a uniform slope cancels out,
        # so rugged massifs carry no overall bias (valleys > 0, crests < 0).
        total += 0.5 * (
            best / np.sqrt(1.0 + best * best) + worst / np.sqrt(1.0 + worst * worst)
        )
    return total / np.float32(directions)


def compute_occlusion(
    height_m: np.ndarray,
    meters_per_px: float,
    radii_m: tuple[float, ...] = DEFAULT_RADII_M,
    directions: int = DEFAULT_DIRECTIONS,
    exaggeration: float = DEFAULT_EXAGGERATION,
) -> np.ndarray:
    """Multi-scale signed occlusion (mean of :func:`band_openness` over bands).

    Args:
        height_m: Elevation grid in metres (sea already clamped by the caller).
        meters_per_px: Ground size of one grid pixel.
        radii_m: Horizon search radius of each band.
        directions: Number of azimuths sampled per band.
        exaggeration: Vertical exaggeration applied to height differences.

    Returns:
        ``float32`` grid, > 0 occluded (valley), < 0 open (crest).
    """
    bands = [
        band_openness(height_m, meters_per_px, radius, directions, exaggeration)
        for radius in radii_m
    ]
    return np.mean(bands, axis=0).astype(np.float32)


def encode_occlusion(
    occlusion: np.ndarray, land: np.ndarray, gain: float | None = None
) -> tuple[np.ndarray, float]:
    """Encode signed occlusion to L8 (128 neutral), sea forced to 128.

    Args:
        occlusion: Signed occlusion from :func:`compute_occlusion`.
        land: Boolean land mask of the same shape.
        gain: Scale factor; ``None`` maps the land 99.5th percentile of ``|o|`` to 1.

    Returns:
        ``(uint8 array, gain used)``.
    """
    if gain is None:
        sample = np.abs(occlusion[land]) if land.any() else np.abs(occlusion)
        top = float(np.percentile(sample, NORMALISE_PERCENTILE)) if sample.size else 0.0
        gain = 1.0 / top if top > 1e-6 else 1.0
    scaled = np.clip(occlusion * gain, -1.0, 1.0)
    scaled = np.sign(scaled) * np.abs(scaled) ** RESPONSE_GAMMA
    encoded = np.rint(128.0 + 127.0 * scaled)
    encoded[~land] = 128.0
    return encoded.astype(np.uint8), gain


def _read_display_height(map_dir: Path) -> np.ndarray:
    """Display heightmap in metres (``heightmap_render.png`` when present)."""
    path = map_dir / "heightmap_render.png"
    if not path.exists():
        path = map_dir / "heightmap.png"
    with Image.open(path) as image:
        raw = np.asarray(image)
    return terrain.uint16_to_height(raw).astype(np.float32)


def _block_mean(array: np.ndarray, factor: int) -> np.ndarray:
    rows = array.shape[0] // factor * factor
    cols = array.shape[1] // factor * factor
    view = array[:rows, :cols].reshape(rows // factor, factor, cols // factor, factor)
    return view.mean(axis=(1, 3))


def build(
    map_dir: Path = MAP_DIR,
    meters_per_px: float | None = None,
    radii_m: tuple[float, ...] = DEFAULT_RADII_M,
    directions: int = DEFAULT_DIRECTIONS,
    exaggeration: float = DEFAULT_EXAGGERATION,
) -> OcclusionResult:
    """Bake ``relief_occlusion.png`` next to the heightmap.

    Args:
        map_dir: Directory holding ``map.json`` and the heightmaps.
        meters_per_px: Map pixel size; read from ``map.json`` when ``None``.
        radii_m: Horizon search radius of each band.
        directions: Number of azimuths per band.
        exaggeration: Vertical exaggeration applied to height differences.

    Returns:
        The written file and its statistics.
    """
    if meters_per_px is None:
        import json

        meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
        meters_per_px = float(meta["meters_per_px"])
    height = _read_display_height(map_dir)
    land_full = height > 0.0
    height = np.maximum(height, 0.0)
    half = _block_mean(height, OCCLUSION_FACTOR).astype(np.float32)
    land = _block_mean(land_full.astype(np.float32), OCCLUSION_FACTOR) > 0.5
    occlusion = compute_occlusion(
        half, meters_per_px * OCCLUSION_FACTOR, radii_m, directions, exaggeration
    )
    occlusion = ndimage.gaussian_filter(occlusion, FINAL_SMOOTH_SIGMA, mode="nearest")
    encoded, gain = encode_occlusion(occlusion, land)
    path = map_dir / OUTPUT_NAME
    Image.fromarray(encoded, mode="L").save(path, optimize=True)
    return OcclusionResult(
        path=path,
        size=(encoded.shape[1], encoded.shape[0]),
        gain=gain,
        bytes=path.stat().st_size,
    )
