"""Campaign ground colour map, "satellite" style (chantier SS, ADR 0141).

``cent-ans geo colormap`` bakes the albedo of the campaign ground as seen from a
satellite (Total War: Attila / Pharaoh): a saturated brown / green / golden
farmland patchwork, dark green forests, light dirt roads, lake shores and town
halos. Output: one RGB texture at ``scale`` x the map grid (14336 x 12288,
~360 m per texel), every mip level precomputed in Godot's layout, BC1 (DXT1) in
zlib parts ``colormap_bc1_<i>.bin`` recorded under ``map.json.colormap.bc1``
(read by ``ReliefLandcover.load_gpu_copy`` like ``relief_shade.bc5``), plus a
JPEG preview ``colormap_preview.jpg`` (1792 px wide) for visual judgement.

Layers, in painting order (palette and parameters: ``colormap_style.yaml``,
validated by ``colormap_style.schema.json``; no colour lives in this module):

1. **Regional base** (map grid): ``splat.png`` weights (grassland, farmland,
   forest, rock / heath) coloured by the palette; dryness (south, steppes,
   arid belt of :func:`landcover.dryness`) swaps grass and fields for their dry
   variants; conifer share (``forest_kind.png``), altitude (heath -> rock, snow),
   ``wetlands.png`` (marsh, ponds, wet meadows), patchy coastal sand (sea only)
   and a shallow -> deep sea; low-frequency brightness noise.
2. **Farmland mosaic** (texture grid): warped Voronoi blocks of ``block_size_m``;
   each block is cultivated with a probability given by the farmland / grassland
   shares and the distance to settlements (``settlements_px.json``,
   ``hamlets.json``); it takes a crop (or pasture, or vine on south-facing
   slopes), is split into open-field strips in the north and hedged where
   grassland dominates (bocage). Never on rock, snow, water, dense forest, marsh.
3. **Town halos** (``towns_1340.json``): trodden ground over the built area,
   gardens around it.
4. **Roads** (``roads.geojson``, map pixels): light dirt, width by ``type``
   (at least ``min_px`` texels), soft edges.
5. **Lakes**: inland water of ``land_mask.png`` (components not touching the map
   edge, below ``max_area_px``; modern reservoirs of ``modern_reservoirs.json``
   are painted as land): shore (sand / pebbles), reed belt, shallow clear water.

Randomness is hash-based on world coordinates (a fixed ``seed``), so the result
does not depend on the band height used to bound memory: same inputs, same bytes.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import block_compress, download

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
STYLE_PATH = MAP_DIR / "colormap_style.yaml"
SCHEMA_PATH = REPO_DIR / "data" / "schemas" / "colormap_style.schema.json"
STEM = "colormap_bc1"
MAP_KEY = "colormap.bc1"
PREVIEW_NAME = "colormap_preview.jpg"
PREVIEW_WIDTH = 1792
PREVIEW_QUALITY = 88
#: Rows of the output texture painted at once (≈ 14336 x 1024 float temporaries).
BAND_ROWS = 1024
#: Rows of the map grid per chunk when evaluating noise over the whole grid.
NOISE_CHUNK_ROWS = 1024
#: Voronoi seed jitter inside its cell (share of the cell).
SEED_JITTER = 0.85
#: Reed belt opacity where the noise lets it grow.
REEDS_OPACITY = 0.85
#: Hedge line half-width (texels) at block edges.
HEDGE_TEXELS = 0.75
#: Texels of margin around a band when rasterising roads (for the soft edge).
ROAD_MARGIN = 4


@dataclass
class ColormapInputs:
    """Rasters (map grid, H x W) and vectors (map pixels) the colour map is painted from.

    ``coast_dist_px`` is the signed distance to the shore in map pixels (positive
    on land), ``conifer`` the conifer share of forests (0-1), ``wetlands`` the
    RGB8 marsh / ponds / wet meadow intensities, ``roads`` ``(type, N x 2)``,
    ``towns`` ``(x, y, built_ha)``, ``reservoirs`` points on modern reservoirs.
    """

    land: np.ndarray
    splat: np.ndarray
    height_m: np.ndarray
    coast_dist_px: np.ndarray
    wetlands: np.ndarray
    conifer: np.ndarray
    lon: np.ndarray
    lat: np.ndarray
    meters_per_px: float
    roads: list[tuple[str, np.ndarray]] = field(default_factory=list)
    settlements: np.ndarray = field(default_factory=lambda: np.zeros((0, 2)))
    towns: list[tuple[float, float, float]] = field(default_factory=list)
    reservoirs: np.ndarray = field(default_factory=lambda: np.zeros((0, 2)))

    @property
    def shape(self) -> tuple[int, int]:
        """Map grid ``(rows, cols)``."""
        return self.land.shape


# ---------------------------------------------------------------------------
# Style
# ---------------------------------------------------------------------------


def load_style(path: Path = STYLE_PATH, schema_path: Path = SCHEMA_PATH) -> dict:
    """Parse ``colormap_style.yaml`` and validate it against its schema."""
    import yaml
    from jsonschema import Draft202012Validator

    style = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    schema = json.loads(Path(schema_path).read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema).iter_errors(style), key=lambda e: list(e.path)
    )
    if errors:
        details = "; ".join(
            f"{'/'.join(map(str, e.path)) or '<racine>'}: {e.message}" for e in errors
        )
        raise ValueError(f"{path}: style invalide ({details})")
    return style


def hex_color(value: str) -> np.ndarray:
    """``#rrggbb`` -> float32 sRGB in 0-1."""
    return np.array(
        [int(value[i : i + 2], 16) / 255.0 for i in (1, 3, 5)], dtype=np.float32
    )


# ---------------------------------------------------------------------------
# Deterministic randomness
# ---------------------------------------------------------------------------

_GOLDEN = np.uint64(0x9E3779B97F4A7C15)


def _mix(value: np.ndarray) -> np.ndarray:
    """splitmix64 finaliser (uint64 in, uint64 out, wrapping arithmetic)."""
    z = np.asarray(value).astype(np.uint64)
    with np.errstate(over="ignore"):
        z = (z ^ (z >> np.uint64(30))) * np.uint64(0xBF58476D1CE4E5B9)
        z = (z ^ (z >> np.uint64(27))) * np.uint64(0x94D049BB133111EB)
    return z ^ (z >> np.uint64(31))


def hash_u64(seed: int, *keys: np.ndarray) -> np.ndarray:
    """Hash of integer arrays (broadcast together) with ``seed``."""
    state = _mix(np.uint64(seed & 0xFFFFFFFFFFFFFFFF))
    with np.errstate(over="ignore"):
        for key in keys:
            as_u64 = np.asarray(key).astype(np.int64).astype(np.uint64)
            state = _mix(state ^ (as_u64 + _GOLDEN))
    return state


def unit(hashes: np.ndarray) -> np.ndarray:
    """Hashes -> uniform floats in [0, 1)."""
    return ((hashes >> np.uint64(40)).astype(np.float32)) * np.float32(1.0 / 2**24)


def value_noise(x: np.ndarray, y: np.ndarray, cell: float, seed: int) -> np.ndarray:
    """Smooth value noise in [0, 1] at world coordinates ``x``, ``y`` (``cell`` in the same unit)."""
    gx = np.asarray(x, dtype=np.float64) / cell
    gy = np.asarray(y, dtype=np.float64) / cell
    ix = np.floor(gx)
    iy = np.floor(gy)
    fx = (gx - ix).astype(np.float32)
    fy = (gy - iy).astype(np.float32)
    fx = fx * fx * (3.0 - 2.0 * fx)
    fy = fy * fy * (3.0 - 2.0 * fy)
    ix = ix.astype(np.int64)
    iy = iy.astype(np.int64)
    v00 = unit(hash_u64(seed, ix, iy))
    v10 = unit(hash_u64(seed, ix + 1, iy))
    v01 = unit(hash_u64(seed, ix, iy + 1))
    v11 = unit(hash_u64(seed, ix + 1, iy + 1))
    top = v00 + (v10 - v00) * fx
    bottom = v01 + (v11 - v01) * fx
    return (top + (bottom - top) * fy).astype(np.float32)


def fbm(
    x: np.ndarray, y: np.ndarray, cell: float, seed: int, octaves: int = 3
) -> np.ndarray:
    """Fractal sum of :func:`value_noise` octaves, normalised to [0, 1]."""
    total = np.zeros(np.broadcast(x, y).shape, dtype=np.float32)
    weight = 1.0
    norm = 0.0
    for octave in range(octaves):
        total += weight * value_noise(x, y, cell / 2**octave, seed + 101 * octave)
        norm += weight
        weight *= 0.5
    return total / np.float32(norm)


def _grid_noise(
    shape: tuple[int, int], mpp: float, cell_m: float, seed: int
) -> np.ndarray:
    """:func:`fbm` over the map grid (pixel centres, metres), evaluated in row chunks."""
    rows, cols = shape
    out = np.empty(shape, dtype=np.float32)
    xs = (np.arange(cols, dtype=np.float64) + 0.5) * mpp
    for top in range(0, rows, NOISE_CHUNK_ROWS):
        bottom = min(rows, top + NOISE_CHUNK_ROWS)
        ys = (np.arange(top, bottom, dtype=np.float64) + 0.5) * mpp
        out[top:bottom] = fbm(xs[None, :], ys[:, None], cell_m, seed)
    return out


# ---------------------------------------------------------------------------
# Small helpers
# ---------------------------------------------------------------------------


def smoothstep(edge0: float, edge1: float, x: np.ndarray) -> np.ndarray:
    """Hermite step from ``edge0`` to ``edge1`` (edges may be reversed)."""
    t = np.clip(
        (np.asarray(x, dtype=np.float32) - np.float32(edge0))
        / np.float32(edge1 - edge0),
        0.0,
        1.0,
    )
    return (t * t * (3.0 - 2.0 * t)).astype(np.float32)


def _lerp(a: np.ndarray, b: np.ndarray, t: np.ndarray) -> np.ndarray:
    return (a + (b - a) * t).astype(np.float32)


def _axis_weights(count_in: int, count_out: int, factor: int, start: int) -> tuple:
    centres = (
        np.arange(start, start + count_out, dtype=np.float64) + 0.5
    ) / factor - 0.5
    low = np.floor(centres)
    frac = (centres - low).astype(np.float32)
    i0 = np.clip(low.astype(np.int64), 0, count_in - 1)
    i1 = np.clip(low.astype(np.int64) + 1, 0, count_in - 1)
    return i0, i1, frac


def upsample_rows(values: np.ndarray, factor: int, top: int, bottom: int) -> np.ndarray:
    """Bilinear upsampling by ``factor`` of ``values`` (H x W [x C]), output rows ``[top, bottom)``."""
    rows, cols = values.shape[:2]
    y0, y1, ty = _axis_weights(rows, bottom - top, factor, top)
    x0, x1, tx = _axis_weights(cols, cols * factor, factor, 0)
    extra = (None,) * (values.ndim - 2)
    ty = ty[(slice(None), None, *extra)]
    tx = tx[(None, slice(None), *extra)]
    a = values[y0].astype(np.float32)
    b = values[y1].astype(np.float32)
    vertical = a + (b - a) * ty
    left = vertical[:, x0]
    right = vertical[:, x1]
    return left + (right - left) * tx


def mip_chain_bytes(width: int, height: int, block_bytes: int) -> int:
    """Bytes of a block-compressed mip chain as Godot sizes it (``Image::_get_dst_image_size``)."""
    total = 0
    while True:
        total += ((width + 3) // 4) * ((height + 3) // 4) * block_bytes
        if width == 1 and height == 1:
            return total
        width, height = max(1, width >> 1), max(1, height >> 1)


def _pick(cumulative: np.ndarray, draws: np.ndarray) -> np.ndarray:
    return np.minimum(
        np.searchsorted(cumulative, draws, side="right"), len(cumulative) - 1
    )


def _weighted(entries: list[dict]) -> tuple[np.ndarray, np.ndarray]:
    colors = np.stack([hex_color(e["color"]) for e in entries])
    weights = np.array([float(e["weight"]) for e in entries], dtype=np.float64)
    cumulative = np.cumsum(weights) / weights.sum()
    return colors, cumulative.astype(np.float32)


# ---------------------------------------------------------------------------
# Global fields (map grid)
# ---------------------------------------------------------------------------


@dataclass
class _Fields:
    """Per-pixel fields on the map grid: upsampled per band, or sampled at block seeds."""

    base: np.ndarray  # H x W x 3 sRGB (regional base)
    land: np.ndarray  # 0/1 (reservoirs count as land)
    lake_sd: np.ndarray  # signed distance to lakes, map px (negative inside)
    mask: np.ndarray  # where the farmland mosaic may be painted (0-1)
    dry: np.ndarray
    bocage: np.ndarray
    presence: np.ndarray  # probability that a block is cultivated
    pasture: np.ndarray  # share of pasture among cultivated blocks
    vine: np.ndarray  # vineyard suitability
    openfield: np.ndarray  # probability of open-field strips


def classify_water(
    land: np.ndarray, reservoirs: np.ndarray, max_area_px: int
) -> tuple[np.ndarray, np.ndarray]:
    """``(lake, reservoir)`` masks: inland water not touching the map edge, below ``max_area_px``.

    Water components holding a point of ``reservoirs`` (map pixels) are modern
    reservoirs (anachronisms), painted as land.
    """
    from scipy import ndimage

    labels, count = ndimage.label(~land)
    if count == 0:
        empty = np.zeros_like(land)
        return empty, empty
    sizes = np.bincount(labels.ravel(), minlength=count + 1)
    edge = np.unique(
        np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    )
    inland = sizes <= max_area_px
    inland[0] = False
    inland[edge] = False
    modern = np.zeros(count + 1, dtype=bool)
    rows, cols = land.shape
    for x, y in np.asarray(reservoirs, dtype=np.float64).reshape(-1, 2):
        col, row = int(x), int(y)
        if 0 <= row < rows and 0 <= col < cols and labels[row, col]:
            modern[labels[row, col]] = True
    return (inland & ~modern)[labels], (inland & modern)[labels]


def _signed_distance(mask: np.ndarray) -> np.ndarray:
    """Signed distance (px) to the edge of ``mask``: negative inside, positive outside."""
    from scipy import ndimage

    if not mask.any():
        return np.full(mask.shape, 1e6, dtype=np.float32)
    outside = ndimage.distance_transform_edt(~mask)
    inside = ndimage.distance_transform_edt(mask)
    return np.where(mask, 0.5 - inside, outside - 0.5).astype(np.float32)


def _settlement_distance_km(inputs: ColormapInputs) -> np.ndarray:
    from scipy import ndimage

    rows, cols = inputs.shape
    seeds = np.zeros((rows, cols), dtype=bool)
    points = np.asarray(inputs.settlements, dtype=np.float64).reshape(-1, 2)
    if len(points):
        c = np.clip(points[:, 0].astype(np.int64), 0, cols - 1)
        r = np.clip(points[:, 1].astype(np.int64), 0, rows - 1)
        seeds[r, c] = True
    if not seeds.any():
        return np.full((rows, cols), 1e6, dtype=np.float32)
    distance = ndimage.distance_transform_edt(~seeds)
    return (distance * inputs.meters_per_px / 1000.0).astype(np.float32)


def _dryness(inputs: ColormapInputs, style: dict, noise: np.ndarray) -> np.ndarray:
    from cent_ans_tools.geo import landcover

    base = style["base"]
    by_lat = smoothstep(base["dry_lat_start"], base["dry_lat_full"], inputs.lat)
    steppe = landcover.dryness(inputs.lon, inputs.lat, inputs.height_m, None)
    dry = np.maximum(by_lat, steppe)
    dry = dry + base["moisture_noise_amp"] * (noise - 0.5) * 2.0
    return np.clip(dry, 0.0, 1.0).astype(np.float32)


def _base_color(
    inputs: ColormapInputs,
    style: dict,
    weights: np.ndarray,
    dry: np.ndarray,
    land: np.ndarray,
    lake: np.ndarray,
    lake_sd: np.ndarray,
    snow: np.ndarray,
) -> np.ndarray:
    palette = {k: hex_color(v) for k, v in style["palette"].items()}
    base_style = style["base"]
    seed = int(style["seed"])
    mpp = float(inputs.meters_per_px)
    shape = inputs.shape
    height = inputs.height_m
    d = dry[..., None]
    rocky = smoothstep(base_style["rock_from_m"], base_style["rock_full_m"], height)
    color = weights[..., 0:1] * _lerp(palette["grassland"], palette["grassland_dry"], d)
    color += weights[..., 1:2] * _lerp(palette["farmland"], palette["farmland_dry"], d)
    color += weights[..., 2:3] * _lerp(
        palette["forest_broadleaf"],
        palette["forest_conifer"],
        inputs.conifer[..., None],
    )
    color += weights[..., 3:4] * _lerp(
        palette["heath"], palette["rock"], rocky[..., None]
    )
    del rocky

    wet = inputs.wetlands
    for channel, name in enumerate(("marsh", "ponds", "wet_meadow")):
        amount = wet[..., channel].astype(np.float32) / 255.0
        amount *= base_style["wetland_strength"][channel]
        color = _lerp(color, palette[name], amount[..., None])

    sand_w = base_style["coast_sand_width_m"] / mpp
    if sand_w > 0:
        noise = _grid_noise(
            shape, mpp, base_style["coast_sand_noise_km"] * 1000.0, seed + 2
        )
        share = base_style["coast_sand_share"]
        sand = smoothstep(share + 0.05, share - 0.05, noise)
        del noise
        coast = inputs.coast_dist_px
        sand *= smoothstep(sand_w, 0.0, coast) * (coast > -0.5)
        top_m = base_style["coast_sand_max_height_m"]
        sand *= smoothstep(top_m + 10.0, top_m, height)
        # Mer seulement : les rives des lacs ont leur grève (couche 5).
        sand *= smoothstep(sand_w + 1.0, sand_w + 3.0, lake_sd)
        color = _lerp(color, palette["coast_sand"], sand[..., None])
        del sand

    color = _lerp(color, palette["snow"], snow[..., None])
    brightness = _grid_noise(
        shape, mpp, base_style["brightness_noise_km"] * 1000.0, seed + 3
    )
    color *= (1.0 + base_style["brightness_noise_amp"] * (brightness - 0.5) * 2.0)[
        ..., None
    ]
    del brightness

    depth = smoothstep(
        0.0, base_style["sea_shallow_width_m"] / mpp, -inputs.coast_dist_px
    )
    sea_c = _lerp(palette["sea_shallow"], palette["sea_deep"], depth[..., None])
    color = np.where((~land & ~lake)[..., None], sea_c, color)
    color = np.where(lake[..., None], palette["lake_shallow"], color)
    return np.clip(color, 0.0, 1.0).astype(np.float32)


def compute_fields(inputs: ColormapInputs, style: dict) -> _Fields:
    """Regional base colour and every per-pixel field of the later layers."""
    from scipy import ndimage

    base_style = style["base"]
    mosaic = style["mosaic"]
    seed = int(style["seed"])
    mpp = float(inputs.meters_per_px)
    height = inputs.height_m.astype(np.float32)

    lake, reservoir = classify_water(
        inputs.land, inputs.reservoirs, int(style["lakes"]["max_area_px"])
    )
    land = inputs.land | reservoir
    lake_sd = _signed_distance(lake)

    weights = inputs.splat.astype(np.float32) / 255.0
    total = weights.sum(axis=-1, keepdims=True)
    # Terres sans poids (réservoirs rendus à la terre, bords) : prairie.
    fallback = np.array([1.0, 0.0, 0.0, 0.0], dtype=np.float32)
    weights = np.where(total > 0.01, weights / np.maximum(total, 1e-6), fallback)
    del total
    grass, farm, forest, rockheath = (weights[..., i] for i in range(4))

    moisture = _grid_noise(
        inputs.shape, mpp, base_style["moisture_noise_km"] * 1000.0, seed + 1
    )
    dry = _dryness(inputs, style, moisture)
    del moisture
    snow = smoothstep(base_style["snow_from_m"], base_style["snow_full_m"], height)
    color = _base_color(inputs, style, weights, dry, land, lake, lake_sd, snow)

    # Mosaïque : où, avec quelle probabilité, quelle forme.
    marsh = inputs.wetlands[..., 0].astype(np.float32) / 255.0
    mask = land.astype(np.float32)
    mask *= smoothstep(mosaic["max_rock"] + 0.1, mosaic["max_rock"] - 0.1, rockheath)
    mask *= smoothstep(mosaic["max_forest"] + 0.1, mosaic["max_forest"] - 0.1, forest)
    mask *= smoothstep(mosaic["max_marsh"] + 0.1, mosaic["max_marsh"] - 0.1, marsh)
    mask *= 1.0 - snow
    del marsh, snow

    near = np.exp(-_settlement_distance_km(inputs) / mosaic["settlement_decay_km"])
    near = mosaic["far_share"] + (1.0 - mosaic["far_share"]) * near
    presence = (
        mosaic["gain_farmland"] * farm + mosaic["gain_grassland"] * grass
    ) * near
    presence = np.where(land, np.clip(presence, 0.0, 1.0), 0.0).astype(np.float32)
    del near
    pasture = (grass / np.maximum(grass + farm, 1e-6)).astype(np.float32)

    vine = mosaic["vine"]
    grad_y, grad_x = np.gradient(ndimage.gaussian_filter(height, 1.0), mpp)
    slope = np.hypot(grad_x, grad_y)
    # Versant exposé au sud : l'altitude décroît vers le sud (y croissant).
    south = np.where(slope > 1e-6, -grad_y / np.maximum(slope, 1e-6), 0.0)
    del grad_x, grad_y
    vine_ok = smoothstep(vine["min_slope"] * 0.6, vine["min_slope"], slope)
    vine_ok *= smoothstep(vine["min_south"] - 0.2, vine["min_south"], south)
    vine_ok *= smoothstep(vine["max_lat"] + 0.4, vine["max_lat"] - 0.4, inputs.lat)
    del slope, south

    openfield = smoothstep(
        mosaic["openfield_lat_from"], mosaic["openfield_lat_full"], inputs.lat
    )
    openfield *= smoothstep(0.25, 0.45, farm)
    bocage = smoothstep(mosaic["hedge_grass_from"], mosaic["hedge_grass_full"], grass)

    return _Fields(
        base=color,
        land=land.astype(np.float32),
        lake_sd=lake_sd,
        mask=mask,
        dry=dry,
        bocage=bocage,
        presence=presence,
        pasture=pasture,
        vine=vine_ok.astype(np.float32),
        openfield=openfield.astype(np.float32),
    )


# ---------------------------------------------------------------------------
# Band painters (texture grid)
# ---------------------------------------------------------------------------


def _sample_nearest(
    values: np.ndarray, x_px: np.ndarray, y_px: np.ndarray
) -> np.ndarray:
    rows, cols = values.shape
    c = np.clip(np.floor(x_px).astype(np.int64), 0, cols - 1)
    r = np.clip(np.floor(y_px).astype(np.int64), 0, rows - 1)
    return values[r, c]


@dataclass
class _Band:
    """One horizontal band of the texture being painted."""

    out: np.ndarray  # rows x cols x 3 float32, painted in place
    top: int  # first texture row
    factor: int
    mpp: float
    land: np.ndarray  # 0/1

    def lerp(self, color: np.ndarray, alpha: np.ndarray) -> None:
        """Blend ``color`` (3 or rows x cols x 3) over the band with ``alpha``."""
        np.copyto(self.out, _lerp(self.out, color, alpha[..., None]))


def _voronoi(u: np.ndarray, v: np.ndarray, seed: int) -> tuple:
    """Nearest jittered-grid seed of every point: cell grid, pick indices, edge distance."""
    cu0 = int(np.floor(u.min())) - 2
    cv0 = int(np.floor(v.min())) - 2
    cu1 = int(np.floor(u.max())) + 3
    cv1 = int(np.floor(v.max())) + 3
    gi = np.arange(cu0, cu1, dtype=np.int64)[None, :]
    gj = np.arange(cv0, cv1, dtype=np.int64)[:, None]
    seed_u = gi + 0.5 + (unit(hash_u64(seed + 3, gi, gj)) - 0.5) * SEED_JITTER
    seed_v = gj + 0.5 + (unit(hash_u64(seed + 4, gi, gj)) - 0.5) * SEED_JITTER

    base_i = np.floor(u).astype(np.int64) - cu0
    base_j = np.floor(v).astype(np.int64) - cv0
    best = np.full(u.shape, np.inf, dtype=np.float32)
    second = np.full(u.shape, np.inf, dtype=np.float32)
    best_i = np.zeros(u.shape, dtype=np.int64)
    best_j = np.zeros(u.shape, dtype=np.int64)
    for dj in (-1, 0, 1):
        for di in (-1, 0, 1):
            ci = base_i + di
            cj = base_j + dj
            du = (u - seed_u[cj, ci]).astype(np.float32)
            dv = (v - seed_v[cj, ci]).astype(np.float32)
            dist = np.sqrt(du * du + dv * dv)
            closer = dist < best
            second = np.where(closer, best, np.minimum(second, dist))
            best = np.where(closer, dist, best)
            best_i = np.where(closer, ci, best_i)
            best_j = np.where(closer, cj, best_j)
    return (gi, gj, cu0, cv0, seed_u, seed_v), (best_j, best_i), second - best


def _paint_mosaic(
    band: _Band,
    fields: _Fields,
    style: dict,
    mask: np.ndarray,
    dry: np.ndarray,
    bocage: np.ndarray,
) -> None:
    mosaic = style["mosaic"]
    seed = int(style["seed"]) + 1000
    rows, cols = band.out.shape[:2]
    cell = mosaic["block_size_m"] / band.mpp  # map px per block cell

    xs = (np.arange(cols, dtype=np.float64) + 0.5) / band.factor / cell
    ys = np.arange(band.top, band.top + rows, dtype=np.float64) + 0.5
    ys = ys / band.factor / cell
    u = np.broadcast_to(xs[None, :], (rows, cols))
    v = np.broadcast_to(ys[:, None], (rows, cols))
    warp = mosaic["warp_share"]
    if warp > 0:
        cells = mosaic["warp_cells"]
        du = (value_noise(u, v, cells, seed + 1) - 0.5) * (2.0 * warp)
        dv = (value_noise(u, v, cells, seed + 2) - 0.5) * (2.0 * warp)
        u = u + du
        v = v + dv
        del du, dv

    grid, pick, gap = _voronoi(u, v, seed)
    gi, gj, cu0, cv0, seed_u, seed_v = grid
    edge_texels = gap * (cell * band.factor * 0.5)
    del gap

    # Attributs des blocs sur la grille des cellules (champs échantillonnés à la graine).
    seed_x, seed_y = seed_u * cell, seed_v * cell
    cultivated = unit(hash_u64(seed + 5, gi, gj)) < _sample_nearest(
        fields.presence, seed_x, seed_y
    )
    is_pasture = unit(hash_u64(seed + 6, gi, gj)) < _sample_nearest(
        fields.pasture, seed_x, seed_y
    )
    is_vine = unit(hash_u64(seed + 7, gi, gj)) < mosaic["vine"][
        "share"
    ] * _sample_nearest(fields.vine, seed_x, seed_y)
    is_strips = unit(hash_u64(seed + 8, gi, gj)) < _sample_nearest(
        fields.openfield, seed_x, seed_y
    )
    angle = unit(hash_u64(seed + 9, gi, gj)) * np.float32(np.pi)

    # Lanières d'openfield : le bloc est découpé en bandes parallèles.
    strip = np.zeros(u.shape, dtype=np.int64)
    strips = is_strips[pick]
    if strips.any():
        theta = angle[pick]
        across = (u - seed_u[pick]) * -np.sin(theta) + (v - seed_v[pick]) * np.cos(
            theta
        )
        strip = np.where(
            strips, np.floor(across * mosaic["strip_ratio"]).astype(np.int64) + 1, 0
        )
        del theta, across
    world_i = pick[1] + cu0
    world_j = pick[0] + cv0
    draw = unit(hash_u64(seed + 10, world_i, world_j, strip))
    jitter = unit(hash_u64(seed + 11, world_i, world_j, strip))
    del world_i, world_j, strip

    crops_c, crops_cum = _weighted(mosaic["crops"])
    past_c, past_cum = _weighted(mosaic["pastures"])
    color = np.where(
        is_pasture[pick][..., None],
        past_c[_pick(past_cum, draw)],
        crops_c[_pick(crops_cum, draw)],
    )
    color = np.where(
        is_vine[pick][..., None], hex_color(mosaic["vine"]["color"]), color
    )
    color = _lerp(
        color,
        hex_color(mosaic["dry_crop_tint"]),
        (mosaic["dry_crop_amount"] * dry)[..., None],
    )
    color *= (1.0 + mosaic["block_jitter"] * (jitter - 0.5) * 2.0)[..., None]

    band.lerp(color, cultivated[pick] * mask * np.float32(mosaic["opacity"]))
    hedge = np.clip(1.0 - edge_texels / HEDGE_TEXELS, 0.0, 1.0)
    hedge *= mosaic["hedge_strength"] * bocage * mask
    band.lerp(hex_color(style["palette"]["hedge"]), hedge)


def _town_discs(
    towns: list[tuple[float, float, float]], style: dict, factor: int, mpp: float
) -> list[tuple[float, float, float, float]]:
    """``(x, y, core, garden)`` in texture pixels."""
    spec = style["towns"]
    texel_m = mpp / factor
    discs = []
    for x, y, built_ha in towns:
        radius = np.sqrt(max(built_ha, 0.0) * 1e4 / np.pi) / texel_m
        core = max(spec["ground_min_px"], float(radius))
        discs.append((x * factor, y * factor, core, core * spec["garden_factor"]))
    return discs


def _paint_towns(band: _Band, discs: list, style: dict) -> None:
    spec = style["towns"]
    seed = int(style["seed"]) + 2000
    ground = hex_color(style["palette"]["town_ground"])
    gardens = hex_color(style["palette"]["town_gardens"])
    rows, cols = band.out.shape[:2]
    top = band.top
    for x, y, core, garden in discs:
        reach = garden + 1.0
        r0 = max(0, int(np.floor(y - reach)) - top)
        r1 = min(rows, int(np.ceil(y + reach)) - top + 1)
        c0 = max(0, int(np.floor(x - reach)))
        c1 = min(cols, int(np.ceil(x + reach)) + 1)
        if r0 >= r1 or c0 >= c1:
            continue
        rr = np.arange(r0, r1, dtype=np.int64) + top
        cc = np.arange(c0, c1, dtype=np.int64)
        dist = np.hypot(cc[None, :] + 0.5 - x, rr[:, None] + 0.5 - y).astype(np.float32)
        patch = band.out[r0:r1, c0:c1]
        land = band.land[r0:r1, c0:c1]
        speckle = unit(hash_u64(seed, cc[None, :], rr[:, None]))
        garden_a = spec["garden_opacity"] * np.clip(garden + 0.5 - dist, 0.0, 1.0)
        garden_a *= (0.6 + 0.4 * speckle) * land
        patch[:] = _lerp(patch, gardens, garden_a[..., None])
        ground_a = spec["ground_opacity"] * np.clip(core + 0.5 - dist, 0.0, 1.0) * land
        patch[:] = _lerp(patch, ground, ground_a[..., None])


def road_texels(
    roads: list[tuple[str, np.ndarray]], factor: int
) -> dict[str, tuple[np.ndarray, np.ndarray]]:
    """Texels crossed by each road type: ``kind -> (rows, cols)`` sorted by row.

    Segments are sampled every quarter texel in texture coordinates, so the
    trace depends only on world coordinates (not on the band being painted).
    """
    by_kind: dict[str, list[np.ndarray]] = {}
    for kind, points in roads:
        points = np.asarray(points, dtype=np.float64).reshape(-1, 2) * factor
        if len(points) < 2:
            continue
        start, stop = points[:-1], points[1:]
        steps = np.maximum(
            1, np.ceil(np.hypot(*(stop - start).T) * 4.0).astype(np.int64)
        )
        seg = np.repeat(np.arange(len(steps)), steps + 1)
        offsets = np.arange(len(seg)) - np.repeat(
            np.cumsum(steps + 1) - (steps + 1), steps + 1
        )
        t = (offsets / np.repeat(steps, steps + 1))[:, None]
        samples = start[seg] + (stop[seg] - start[seg]) * t
        by_kind.setdefault(kind, []).append(np.floor(samples).astype(np.int64))
    texels = {}
    for kind, chunks in by_kind.items():
        cells = np.unique(np.concatenate(chunks)[:, ::-1], axis=0)  # (row, col), sorted
        texels[kind] = (cells[:, 0], cells[:, 1])
    return texels


def _paint_roads(band: _Band, texels: dict, style: dict) -> None:
    from scipy import ndimage

    spec = style["roads"]
    soft = spec["soft_px"]
    rows, cols = band.out.shape[:2]
    texel_m = band.mpp / band.factor
    first = band.top - ROAD_MARGIN
    alpha = np.zeros((rows, cols), dtype=np.float32)
    for kind, kind_spec in spec["types"].items():
        if kind not in texels:
            continue
        road_rows, road_cols = texels[kind]
        lo, hi = np.searchsorted(road_rows, [first, band.top + rows + ROAD_MARGIN])
        r = road_rows[lo:hi] - first
        c = road_cols[lo:hi]
        inside = (c >= 0) & (c < cols)
        if not inside.any():
            continue
        raster = np.zeros((rows + 2 * ROAD_MARGIN, cols), dtype=bool)
        raster[r[inside], c[inside]] = True
        dist = ndimage.distance_transform_edt(~raster)
        dist = dist[ROAD_MARGIN : ROAD_MARGIN + rows]
        half = max(kind_spec["min_px"], kind_spec["width_m"] / texel_m) * 0.5
        alpha = np.maximum(alpha, smoothstep(half + soft, half - soft, dist))
    band.lerp(hex_color(style["palette"]["road"]), alpha * spec["opacity"] * band.land)


def _paint_lakes(band: _Band, style: dict, sd_px: np.ndarray) -> None:
    lakes = style["lakes"]
    palette = style["palette"]
    texel_m = band.mpp / band.factor
    sd = sd_px * np.float32(band.mpp)
    if not (sd < lakes["shore_width_m"] + texel_m).any():
        return
    rows, cols = band.out.shape[:2]
    # Grève sur la rive, côté terre.
    shore = np.clip((lakes["shore_width_m"] - sd) / texel_m + 0.5, 0.0, 1.0)
    shore *= np.clip(sd / texel_m + 0.5, 0.0, 1.0) * band.land
    band.lerp(hex_color(palette["lake_shore"]), shore)
    # Eau : claire au bord, plus sombre au large.
    water = np.clip(-sd / texel_m + 0.5, 0.0, 1.0)
    depth = smoothstep(0.0, lakes["shallow_width_m"], -sd)
    water_c = _lerp(
        hex_color(palette["lake_shallow"]),
        hex_color(palette["lake_deep"]),
        depth[..., None],
    )
    band.lerp(water_c, water)
    # Roselière : bande d'eau peu profonde le long de la rive, interrompue par le bruit.
    if lakes["reeds_width_m"] > 0:
        xs = (np.arange(cols, dtype=np.float64) + 0.5) * texel_m
        ys = (np.arange(band.top, band.top + rows, dtype=np.float64) + 0.5) * texel_m
        noise = value_noise(
            xs[None, :],
            ys[:, None],
            lakes["reeds_noise_km"] * 1000.0,
            int(style["seed"]) + 3000,
        )
        share = lakes["reeds_share"]
        reeds = smoothstep(share + 0.08, share - 0.08, noise) * water
        reeds *= smoothstep(
            lakes["reeds_width_m"] + texel_m, lakes["reeds_width_m"], -sd
        )
        band.lerp(hex_color(palette["reeds"]), reeds * REEDS_OPACITY)


# ---------------------------------------------------------------------------
# Bake
# ---------------------------------------------------------------------------


def bake(inputs: ColormapInputs, style: dict, band_rows: int = BAND_ROWS) -> np.ndarray:
    """RGB colour map (``scale`` x the map grid), uint8, painted in horizontal bands."""
    factor = int(style["scale"])
    mpp = float(inputs.meters_per_px)
    fields = compute_fields(inputs, style)
    rows, cols = inputs.shape
    out_rows = rows * factor
    image = np.empty((out_rows, cols * factor, 3), dtype=np.uint8)
    texels = road_texels(inputs.roads, factor)
    discs = _town_discs(inputs.towns, style, factor, mpp)
    for top in range(0, out_rows, band_rows):
        bottom = min(out_rows, top + band_rows)
        land = (upsample_rows(fields.land, factor, top, bottom) > 0.5).astype(
            np.float32
        )
        band = _Band(
            out=upsample_rows(fields.base, factor, top, bottom),
            top=top,
            factor=factor,
            mpp=mpp,
            land=land,
        )
        _paint_mosaic(
            band,
            fields,
            style,
            upsample_rows(fields.mask, factor, top, bottom),
            upsample_rows(fields.dry, factor, top, bottom),
            upsample_rows(fields.bocage, factor, top, bottom),
        )
        near = [
            d for d in discs if d[1] + d[3] + 1 >= top and d[1] - d[3] - 1 <= bottom
        ]
        _paint_towns(band, near, style)
        _paint_roads(band, texels, style)
        _paint_lakes(band, style, upsample_rows(fields.lake_sd, factor, top, bottom))
        image[top:bottom] = np.clip(np.rint(band.out * 255.0), 0, 255).astype(np.uint8)
    return image


# ---------------------------------------------------------------------------
# Inputs and outputs
# ---------------------------------------------------------------------------


def load_inputs(map_dir: Path = MAP_DIR) -> ColormapInputs:
    """Read the rasters and vectors of ``map_dir`` (map grid of ``map.json``)."""
    from PIL import Image

    from cent_ans_tools.geo import landcover, terrain
    from cent_ans_tools.geo.project import grid_from_metadata

    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    grid = grid_from_metadata(meta)

    def read(name: str, mode: str) -> np.ndarray:
        with Image.open(map_dir / name) as image:
            return np.asarray(image.convert(mode))

    land = read("land_mask.png", "L") > 127
    height = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    coast = (read("coast_dist.png", "L").astype(np.float32) - 128.0) / 2.0
    kind = read("forest_kind.png", "L").astype(np.float32) / 255.0
    factor = land.shape[0] // kind.shape[0]
    conifer = upsample_rows(kind, factor, 0, land.shape[0]) if factor > 1 else kind
    lon, lat = landcover.pixel_lonlat(grid)

    def load_json(name: str) -> dict | list:
        return json.loads((map_dir / name).read_text(encoding="utf-8"))

    roads = [
        (
            str(feature["properties"].get("type", "")),
            np.asarray(feature["geometry"]["coordinates"], dtype=np.float64),
        )
        for feature in load_json("roads.geojson")["features"]
        if (feature.get("geometry") or {}).get("type") == "LineString"
    ]
    positions = load_json("settlements_px.json")
    points = list(positions.values()) + [h["px"] for h in load_json("hamlets.json")]
    towns = [
        (
            float(positions[tid][0]),
            float(positions[tid][1]),
            float(town.get("built_ha", 0)),
        )
        for tid, town in sorted(load_json("towns_1340.json")["towns"].items())
        if tid in positions
    ]
    entries = load_json("modern_reservoirs.json").get("reservoirs", [])
    reservoirs = np.zeros((0, 2))
    if entries:
        rx, ry = grid.lonlat_to_pixel(
            np.array([e["lon"] for e in entries]), np.array([e["lat"] for e in entries])
        )
        reservoirs = np.stack([np.asarray(rx), np.asarray(ry)], axis=-1)
    return ColormapInputs(
        land=land,
        splat=read("splat.png", "RGBA"),
        height_m=height.astype(np.float32),
        coast_dist_px=coast,
        wetlands=read("wetlands.png", "RGB"),
        conifer=conifer.astype(np.float32),
        lon=lon,
        lat=lat,
        meters_per_px=float(grid.meters_per_px),
        roads=roads,
        settlements=np.asarray(points, dtype=np.float64).reshape(-1, 2),
        towns=towns,
        reservoirs=reservoirs,
    )


def encode_chain_bc1(image: np.ndarray) -> bytes:
    """BC1 bytes of every mip level of ``image`` (H x W x 3), Godot's ``FORMAT_DXT1`` layout."""
    return b"".join(
        block_compress.encode_bc1(level) for level in block_compress.mip_chain(image)
    )


def write_colormap(
    image: np.ndarray, map_dir: Path, stem: str = STEM
) -> tuple[list[Path], dict]:
    """BC1 parts with mipmaps and their ``map.json`` metadata (not yet recorded)."""
    height, width = image.shape[:2]
    data = encode_chain_bc1(image)
    expected = mip_chain_bytes(width, height, 8)
    if len(data) != expected:
        raise ValueError(f"BC1 chain is {len(data)} bytes, expected {expected}")
    paths, sizes = block_compress.write_parts(data, map_dir, stem)
    return paths, block_compress._meta("dxt1", width, height, True, data, stem, sizes)


def write_preview(image: np.ndarray, path: Path, width: int = PREVIEW_WIDTH) -> Path:
    """JPEG preview ``width`` px wide (box-filtered halvings, then a final resize)."""
    from PIL import Image

    level = image
    while level.shape[1] >= 2 * width:
        level = block_compress._downsample(level)
    preview = Image.fromarray(level)
    if preview.width != width:
        new_height = max(1, round(preview.height * width / preview.width))
        preview = preview.resize((width, new_height), Image.Resampling.LANCZOS)
    preview.save(path, quality=PREVIEW_QUALITY, optimize=True)
    return path


def build(map_dir: Path = MAP_DIR, style_path: Path = STYLE_PATH) -> list[Path]:
    """``geo colormap``: bake, BC1 parts with mipmaps, ``map.json`` entry, preview."""
    started = time.monotonic()
    style = load_style(style_path)
    inputs = load_inputs(map_dir)
    image = bake(inputs, style)
    del inputs
    baked = time.monotonic()
    preview = write_preview(image, map_dir / PREVIEW_NAME)
    paths, meta = write_colormap(image, map_dir)
    block_compress.update_map_json(map_dir, MAP_KEY, meta)
    print(
        f"colormap : peinture {baked - started:.0f} s, "
        f"BC1 {time.monotonic() - baked:.0f} s, {meta['bytes'] / 1e6:.1f} Mo bruts"
    )
    return [*paths, preview, map_dir / "map.json"]
