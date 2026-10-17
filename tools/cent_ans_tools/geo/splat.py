"""Offline rasters for the campaign terrain shader.

Three files are written next to the heightmap in ``data/map/``:

``splat.png``
    RGBA8, half the map grid (one texel = 2 x 2 map pixels), material weights normalised to 255 on land (0 at sea):
    R = grassland, G = farmland, B = forest, A = rock / heath. Built from the
    dominant terrain of each province (``data/provinces/*.json``), altitude,
    slope, distance to the coast and fractal noise.
``province_border_dist.png``
    RGB8, full resolution: three signed distance fields (map pixels,
    ``(value - 128) / BORDER_DIST_SCALE``) to the nearest land border between two
    provinces; ``min(|R|, |G|, |B|)`` after bilinear filtering is a sub-pixel
    distance to the border line, so the shader draws anti-aliased borders with
    ``smoothstep`` (see :func:`compute_border_dist`).
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
from scipy import ndimage, special

from cent_ans_tools.geo import download, terrain
from cent_ans_tools.geo.provinces import decode_ids

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"

SPLAT_FACTOR = 2  # map pixels per splat texel (per side)
BORDER_DIST_SCALE = 6.0  # 1 px = 6 levels, signed range ±21 px
BORDER_RANGE_PX = 127.0 / BORDER_DIST_SCALE
BORDER_CHANNELS = 3
BORDER_SMOOTH_SIGMA = 0.7
COAST_DIST_SCALE = 2.0  # 1 px = 2 levels, range ±64 px
#: Gaussian (px) of the sub-pixel shore of ``compute_coast_dist``; the contour of a binary mask is a
#: staircase of whole pixels (719 m), which the shore bands of the terrain and the sea reproduce.
COAST_SMOOTH_SIGMA = 1.3
#: Band (px from the shore) over which the smooth shore field fades into the exact distance.
COAST_SMOOTH_BAND = (2.0, 4.0)
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
    # Bruit fractal : taches de bois, de landes et de cultures à plusieurs échelles. Tiré dans
    # l'ordre d'origine (un champ par canal) ; le reste est local et se calcule par bandes.
    noises = [fractal_noise(shape, 48, 5, rng) for _ in range(4)]
    weights = np.empty((*shape, 4), dtype=np.float32)
    # Lot CC CS : par bandes avec halo (flou de frontières 41 px, flou de relief 49 px).
    halo = SPLAT_HALO
    for top in range(0, shape[0], BAND_ROWS):
        bottom = min(shape[0], top + BAND_ROWS)
        lo, hi = max(0, top - halo), min(shape[0], bottom + halo)
        slab = _splat_slab(
            height_m[lo:hi],
            land[lo:hi],
            ids[lo:hi],
            table,
            meters_per_px,
            [noise[lo:hi] for noise in noises],
        )
        weights[top:bottom] = slab[top - lo : bottom - lo]
    return weights


#: Rows of context of a splat band: the widest kernel is the relief filter (σ 12: 49 px) and
#: the province blend (σ 10: 41 px).
SPLAT_HALO = 56


def _splat_slab(
    height_m: np.ndarray,
    land: np.ndarray,
    ids: np.ndarray,
    table: np.ndarray,
    meters_per_px: float,
    noises: list[np.ndarray],
) -> np.ndarray:
    """Material weights of one slab (see :func:`compute_splat`); same arithmetic as the whole map."""
    # Fondu entre provinces (~20 px) : pas de marche de matériau le long des frontières.
    base = [
        masked_blur(np.ascontiguousarray(table[:, channel][ids]), land, 10.0)
        for channel in range(4)
    ]
    grad_y, grad_x = np.gradient(ndimage.gaussian_filter(height_m, 1.0), meters_per_px)
    slope = np.hypot(grad_x, grad_y)
    del grad_x, grad_y
    relief = height_m - ndimage.gaussian_filter(height_m, 12.0)
    coast = edt_capped(land, 4.0)  # seuils lissés à ≤ 3 px : plafonné
    h = np.maximum(height_m, 0.0)

    grass, farm, forest, rock = base
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

    strengths = (1.2, 1.5, 1.8, 1.4)
    channels = []
    for weight, strength, noise in zip(
        (grass, farm, forest, rock), strengths, noises, strict=True
    ):
        weight = (weight.astype(np.float32) * np.exp(strength * noise)).astype(
            np.float32
        )
        channels.append(
            np.maximum(weight, 0.0) ** 1.6
        )  # contraste : zones plus franches
    total = channels[0] + channels[1]
    total = total + channels[2]
    total = total + channels[3]
    out = np.empty((*height_m.shape, 4), dtype=np.float32)
    ok = total > 1e-6
    denominator = np.maximum(total, 1e-6)
    for index, channel in enumerate(channels):
        out[..., index] = np.where(ok, channel / denominator, 0.0)
    out[~land] = 0.0
    return out


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


def _neighbour_pairs(labels: np.ndarray):
    """Yield ``(a, b, slice_a, slice_b)`` for horizontal and vertical 4-neighbours."""
    yield (
        labels[:-1, :],
        labels[1:, :],
        (slice(None, -1), slice(None)),
        (slice(1, None), slice(None)),
    )
    yield (
        labels[:, :-1],
        labels[:, 1:],
        (slice(None), slice(None, -1)),
        (slice(None), slice(1, None)),
    )


def region_labels(ids: np.ndarray, land: np.ndarray) -> np.ndarray:
    """Land regions: province index, ``max + 1`` for land outside every province, -1 at sea."""
    labels = ids.astype(np.int32)
    labels[(labels == 0) & land] = int(ids.max()) + 1
    labels[~land] = -1
    return labels


def border_pixels(labels: np.ndarray) -> np.ndarray:
    """Pixels on a land border between two different regions (both sides marked)."""
    border = np.zeros(labels.shape, dtype=bool)
    for a, b, slice_a, slice_b in _neighbour_pairs(labels):
        diff = (a != b) & (a >= 0) & (b >= 0)
        border[slice_a] |= diff
        border[slice_b] |= diff
    return border


def colour_regions(
    labels: np.ndarray, channels: int = BORDER_CHANNELS
) -> dict[int, int]:
    """Greedy colouring of the region adjacency graph with at most ``2**channels`` colours.

    Adjacent regions get different colours, hence differ in at least one bit, so at
    least one channel of the signed field changes sign across every border.
    """
    neighbours: dict[int, set[int]] = {}
    for a, b, _, _ in _neighbour_pairs(labels):
        mask = (a != b) & (a >= 0) & (b >= 0)
        for x, y in set(zip(a[mask].tolist(), b[mask].tolist(), strict=True)):
            neighbours.setdefault(x, set()).add(y)
            neighbours.setdefault(y, set()).add(x)
    for region in np.unique(labels[labels >= 0]).tolist():
        neighbours.setdefault(region, set())
    colours: dict[int, int] = {}
    for region in sorted(neighbours, key=lambda r: (-len(neighbours[r]), r)):
        used = {colours[n] for n in neighbours[region] if n in colours}
        colour = next(c for c in range(2**channels) if c not in used)
        colours[region] = colour
    return colours


def compute_border_dist(ids: np.ndarray, land: np.ndarray) -> np.ndarray:
    """Signed distances to province borders, ``(rows, cols, BORDER_CHANNELS)`` in px.

    Every channel holds the distance to the nearest land border (0.5 on border
    pixels); its sign is one bit of the region colour (:func:`colour_regions`).
    With bilinear filtering, ``min(|channel|)`` is then a sub-pixel distance to
    the border line (zero exactly between two provinces). Sea pixels take the
    sign of the nearest land region so coasts never produce a sign change.
    """
    labels = region_labels(ids, land)
    border = border_pixels(labels)
    if not border.any():
        return np.full((*ids.shape, BORDER_CHANNELS), BORDER_RANGE_PX, dtype=np.float32)
    dist = ndimage.distance_transform_edt(~border).astype(np.float32) + 0.5
    if (~land).any() and land.any():
        nearest = ndimage.distance_transform_edt(
            ~land, return_distances=False, return_indices=True
        )
        labels = labels[nearest[0], nearest[1]]
    colours = colour_regions(labels)
    lut = np.zeros(int(labels.max()) + 1, dtype=np.int32)
    for region, colour in colours.items():
        lut[region] = colour
    colour_map = lut[np.maximum(labels, 0)]
    out = np.empty((*ids.shape, BORDER_CHANNELS), dtype=np.float32)
    for channel in range(BORDER_CHANNELS):
        sign = np.where((colour_map >> channel) & 1, -1.0, 1.0).astype(np.float32)
        out[..., channel] = ndimage.gaussian_filter(sign * dist, BORDER_SMOOTH_SIGMA)
    return out


def encode_border_dist(dist: np.ndarray) -> np.ndarray:
    """``uint8`` encoding: 128 + ``BORDER_DIST_SCALE`` levels per px (±``BORDER_RANGE_PX``)."""
    return np.clip(np.rint(128.0 + dist * BORDER_DIST_SCALE), 0, 255).astype(np.uint8)


def decode_border_dist(encoded: np.ndarray) -> np.ndarray:
    """Unsigned distance to the nearest border (min over channels), as the shader does."""
    signed = (encoded.astype(np.float32) - 128.0) / BORDER_DIST_SCALE
    return np.abs(signed).min(axis=-1)


def compute_coast_dist(water: np.ndarray) -> np.ndarray:
    """Signed distance (px) to the shore: positive on land, negative on water.

    Far from the shore the exact distance to the nearest pixel of the other side (minus the half
    pixel to the pixel edge). Within ``COAST_SMOOTH_BAND`` px of it, the distance to the smooth
    contour of the blurred mask (``sigma * sqrt(2) * erfinv(2 v - 1)`` for a mask blurred to ``v``):
    a straight coast stays straight at any angle instead of a staircase of pixels.
    """
    if water.all() or not water.any():
        return np.where(water, -64.0, 64.0).astype(np.float32)
    inside = ndimage.distance_transform_edt(~water)
    outside = ndimage.distance_transform_edt(water)
    exact = inside - outside
    exact = exact - 0.5 * np.sign(exact)
    blurred = ndimage.gaussian_filter((~water).astype(np.float32), COAST_SMOOTH_SIGMA)
    blurred = np.clip(blurred, 1e-4, 1.0 - 1e-4)
    smooth = COAST_SMOOTH_SIGMA * np.sqrt(2.0) * special.erfinv(2.0 * blurred - 1.0)
    lo, hi = COAST_SMOOTH_BAND
    t = np.clip((np.abs(exact) - lo) / (hi - lo), 0.0, 1.0)
    keep_exact = t * t * (3.0 - 2.0 * t)
    return (smooth * (1.0 - keep_exact) + exact * keep_exact).astype(np.float32)


def encode_coast_dist(signed: np.ndarray) -> np.ndarray:
    """``uint8`` encoding: 128 = shore, ``COAST_DIST_SCALE`` levels per pixel."""
    return np.clip(np.rint(128.0 + signed * COAST_DIST_SCALE), 0, 255).astype(np.uint8)


#: Rows per band of the banded distance fields (lot CC CS : memory ceiling of ``geo splat``).
BAND_ROWS = 512
#: Halo (rows) around a band: larger than the distance range kept by the 8-bit encodings
#: (coast ±64 px, border ±21 px + smoothing), so banded results equal the whole-map ones.
COAST_HALO = 80
BORDER_HALO = 32


def edt_capped(
    mask: np.ndarray, cap_px: float, band_rows: int | None = None
) -> np.ndarray:
    """``ndimage.distance_transform_edt(mask)`` clipped to ``cap_px`` (float32), in horizontal bands.

    Distance (px) of every nonzero pixel to the nearest zero pixel. Each band is transformed with a
    halo of ``cap_px + 2`` rows, which makes every value up to ``cap_px`` exact; larger ones are
    clipped to ``cap_px`` whatever the slab sees. The whole-map transform needs ≈ 2 GB of
    temporaries on the 7080 x 6104 map; a band needs ≈ 150 MB (lot CC CS). Callers pass a cap
    beyond which their consumer saturates (smoothstep edges, ``exp(-(d / k) ** 2)`` tails).
    """
    rows, cols = mask.shape
    band = band_rows or BAND_ROWS
    halo = int(np.ceil(cap_px)) + 2
    out = np.empty((rows, cols), dtype=np.float32)
    for top in range(0, rows, band):
        bottom = min(rows, top + band)
        lo, hi = max(0, top - halo), min(rows, bottom + halo)
        slab = mask[lo:hi]
        if slab.all():
            out[top:bottom] = np.float32(cap_px)
            continue
        dist = ndimage.distance_transform_edt(slab).astype(np.float32)
        out[top:bottom] = np.minimum(dist[top - lo : bottom - lo], np.float32(cap_px))
    return out


def gaussian_banded(
    values: np.ndarray,
    sigma: float,
    band_rows: int | None = None,
    truncate: float = 4.0,
) -> np.ndarray:
    """``ndimage.gaussian_filter(values, sigma)`` evaluated in horizontal bands (same values).

    A halo of ``int(truncate * sigma + 0.5)`` rows (the kernel radius) makes the interior of every
    slab exact; at the map's own top and bottom the slab edge is the map edge, so the reflection
    is the same. Peak memory: one slab instead of several whole-map temporaries (lot CC CS).
    """
    rows = values.shape[0]
    band = band_rows or BAND_ROWS
    halo = int(truncate * sigma + 0.5) + 1
    out = np.empty(values.shape, dtype=ndimage.gaussian_filter(values[:2], 0.5).dtype)
    for top in range(0, rows, band):
        bottom = min(rows, top + band)
        lo, hi = max(0, top - halo), min(rows, bottom + halo)
        out[top:bottom] = ndimage.gaussian_filter(values[lo:hi], sigma)[
            top - lo : bottom - lo
        ]
    return out


def slope_banded(
    height_m: np.ndarray,
    meters_per_px: float,
    sigma: float = 1.0,
    band_rows: int | None = None,
) -> np.ndarray:
    """``hypot(gradient(gaussian_filter(h, sigma), mpp))`` in bands (same values)."""
    rows = height_m.shape[0]
    band = band_rows or BAND_ROWS
    halo = int(4.0 * sigma + 0.5) + 2
    out = np.empty(height_m.shape, dtype=np.float32)
    for top in range(0, rows, band):
        bottom = min(rows, top + band)
        lo, hi = max(0, top - halo), min(rows, bottom + halo)
        grad_y, grad_x = np.gradient(
            ndimage.gaussian_filter(height_m[lo:hi], sigma), meters_per_px
        )
        out[top:bottom] = np.hypot(grad_x, grad_y)[top - lo : bottom - lo]
    return out


def _bands(rows: int, band_rows: int = BAND_ROWS):
    """Yield ``(top, bottom)`` row ranges covering ``rows``."""
    for top in range(0, rows, band_rows):
        yield top, min(rows, top + band_rows)


def coast_dist_banded(water: np.ndarray, band_rows: int = BAND_ROWS) -> np.ndarray:
    """:func:`encode_coast_dist` of :func:`compute_coast_dist`, computed in bands (same bytes).

    A slab ``COAST_HALO`` rows taller than the band is enough: a pixel whose shore is further
    than the encoded range (64 px) saturates whatever the slab sees.
    """
    rows, cols = water.shape
    out = np.empty((rows, cols), dtype=np.uint8)
    for top, bottom in _bands(rows, band_rows):
        lo, hi = max(0, top - COAST_HALO), min(rows, bottom + COAST_HALO)
        slab = water[lo:hi]
        if slab.all() or not slab.any():
            signed = np.where(slab, -64.0, 64.0).astype(np.float32)
        else:
            signed = compute_coast_dist(slab)
        out[top:bottom] = encode_coast_dist(signed[top - lo : bottom - lo])
    return out


def border_dist_banded(
    ids: np.ndarray, land: np.ndarray, band_rows: int = BAND_ROWS
) -> np.ndarray:
    """:func:`encode_border_dist` of :func:`compute_border_dist`, computed in bands (same bytes).

    Region labels, the border mask and the region colours are whole-map (cheap, 8-bit); only the
    distance transforms and the smoothing run on slabs ``BORDER_HALO`` rows taller than the band
    (the encoding saturates at ``BORDER_RANGE_PX`` = 21 px).
    """
    rows, cols = ids.shape
    labels = region_labels(ids, land)
    border = border_pixels(labels)
    if not border.any():
        return encode_border_dist(
            np.full((*ids.shape, BORDER_CHANNELS), BORDER_RANGE_PX, dtype=np.float32)
        )
    if (~land).any() and land.any():
        nearest = ndimage.distance_transform_edt(
            ~land, return_distances=False, return_indices=True
        )
        labels = labels[nearest[0], nearest[1]]
        del nearest
    colours = colour_regions(labels)
    lut = np.zeros(int(labels.max()) + 1, dtype=np.int32)
    for region, colour in colours.items():
        lut[region] = colour
    colour_map = lut[np.maximum(labels, 0)].astype(np.uint8)
    del labels
    out = np.empty((rows, cols, BORDER_CHANNELS), dtype=np.uint8)
    for top, bottom in _bands(rows, band_rows):
        lo, hi = max(0, top - BORDER_HALO), min(rows, bottom + BORDER_HALO)
        slab_border = border[lo:hi]
        if slab_border.any():
            dist = ndimage.distance_transform_edt(~slab_border).astype(np.float32) + 0.5
        else:
            dist = np.full(slab_border.shape, 1e4, dtype=np.float32)
        slab_colour = colour_map[lo:hi]
        for channel in range(BORDER_CHANNELS):
            sign = np.where((slab_colour >> channel) & 1, -1.0, 1.0).astype(np.float32)
            smooth = ndimage.gaussian_filter(sign * dist, BORDER_SMOOTH_SIGMA)
            out[top:bottom, :, channel] = encode_border_dist(
                smooth[top - lo : bottom - lo]
            )
    return out


def build_coast_dist(map_dir: Path = MAP_DIR) -> Path:
    """``coast_dist.png`` alone (needs neither provinces nor settlements; lot CC C4a)."""
    height = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    with Image.open(map_dir / "land_mask.png") as image:
        land_full = np.asarray(image.convert("L")) > 127
    water = (height <= 0.0) | ~land_full
    coast_path = map_dir / "coast_dist.png"
    terrain.write_png8(coast_dist_banded(water), coast_path)
    return coast_path


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
    terrain.write_png8(coast_dist_banded(water), coast_path)

    # Lot CC CS : distances par bandes (pic mémoire < 3 Go au lieu de ~ 14 Go), mêmes octets.
    border_path = map_dir / "province_border_dist.png"
    Image.fromarray(border_dist_banded(ids_full, land_full), mode="RGB").save(
        border_path, compress_level=9
    )

    factor = SPLAT_FACTOR
    height_small = downsample_mean(height, factor)
    land_small = downsample_mean(~water, factor) > 0.5
    ids_small = np.ascontiguousarray(ids_full[::factor, ::factor])
    del height, water, ids_full, land_full
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
