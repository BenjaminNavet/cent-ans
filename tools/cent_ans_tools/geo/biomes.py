"""Campaign biome map (chantier HB, ADR 0143): ``cent-ans geo biomes``.

Bakes ``data/map/biomes.png``: one 8-bit index per map pixel (7168 x 6144, the
grid of ``map.json``, EPSG:3035, ~719 m per pixel). Indices are frozen (read by
the ground colour map, the ground materials and the tree species):
0 sea / off map, 1 oceanic, 2 continental, 3 mediterranean, 4 steppe,
5 boreal, 6 mountain / alpine, 7 semi-arid. Lakes take the biome of their shores.

Source: the 1 km Köppen-Geiger map of Beck et al. (2023, CC BY 4.0), period
1901-1930 (the oldest, least warmed one), downloaded into ``tools/geo/raw/koppen``
and reprojected (nearest) onto the map grid. Köppen classes map to biomes
(``koppen`` in ``biomes.yaml``), then rules adjust them (``rules``):

* oceanic (Cfb, Cfc) only within ``oceanic_max_km`` of the Atlantic seas
  (Channel, North Sea, Bay of Biscay): the Cfb of the Paris basin or Germany is
  continental open field;
* cold Dxb north of ``boreal_min_lat`` is boreal (southern Finland);
* dry classes (BS, BW) are steppe north of ``steppe_min_lat`` (Pontic-Caspian,
  western Kazakhstan), semi-arid south of it, except BS near the Mediterranean
  shore (Attica), which stays mediterranean;
* continental or mediterranean pixels drier than ``dry_min`` on
  :func:`landcover.dryness` (the historical Pontic steppe edge, the Anatolian
  plateau) become steppe (north) or semi-arid (south);
* above a latitude-dependent altitude (higher for dry climates) everything is
  mountain; lowland tundra (ET) is boreal.

Without the Köppen source (download failure) a documented deterministic rule set
(latitude, altitude, distance to the Atlantic, dryness) is used instead
(:func:`classify_fallback`). The classes are then blended with a Gaussian
(``smoothing.sigma_km``, argmax) and islands smaller than ``min_area_km2`` are
merged into their surroundings, so borders are smooth and no pixel is isolated.
Every threshold lives in ``biomes.yaml`` (schema ``biomes.schema.json``).
"""

from __future__ import annotations

import json
import time
import zipfile
from dataclasses import dataclass
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import download

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
LEGEND_PATH = MAP_DIR / "biomes.yaml"
SCHEMA_PATH = REPO_DIR / "data" / "schemas" / "biomes.schema.json"
BIOMES_NAME = "biomes.png"
MAP_KEY = "biomes"
RAW_KOPPEN = download.RAW_DIR / "koppen"
#: Downsampling of the grid for the distance to the Atlantic (150 km scale).
ATLANTIC_STEP = 4

SEA = 0


@dataclass
class BiomeInputs:
    """Map-grid rasters (H x W) the biomes are derived from.

    ``koppen`` holds the Köppen code of every pixel (1-based index into
    ``source.codes``, 0 = no data) or ``None`` for the rule-based fallback;
    ``water`` is open sea (lakes excluded), ``coast_km`` the signed distance to
    the shore (positive on land, clamped), ``dryness`` :func:`landcover.dryness`.
    """

    land: np.ndarray
    water: np.ndarray
    height_m: np.ndarray
    lon: np.ndarray
    lat: np.ndarray
    coast_km: np.ndarray
    dryness: np.ndarray
    meters_per_px: float
    koppen: np.ndarray | None = None

    @property
    def shape(self) -> tuple[int, int]:
        """Map grid ``(rows, cols)``."""
        return self.land.shape


# ---------------------------------------------------------------------------
# Legend
# ---------------------------------------------------------------------------


def load_legend(path: Path = LEGEND_PATH, schema_path: Path = SCHEMA_PATH) -> dict:
    """Parse ``biomes.yaml`` and validate it against its schema."""
    import yaml
    from jsonschema import Draft202012Validator

    legend = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
    schema = json.loads(Path(schema_path).read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema).iter_errors(legend), key=lambda e: list(e.path)
    )
    if errors:
        details = "; ".join(
            f"{'/'.join(map(str, e.path)) or '<racine>'}: {e.message}" for e in errors
        )
        raise ValueError(f"{path}: légende invalide ({details})")
    return legend


def indices(legend: dict) -> dict[str, int]:
    """Biome name -> frozen index."""
    return {entry["name"]: int(entry["index"]) for entry in legend["classes"]}


def koppen_lut(legend: dict) -> np.ndarray:
    """uint8 table: Köppen raster code (0..255) -> biome index (0 for no data)."""
    ids = indices(legend)
    codes = legend["source"]["codes"]
    lut = np.zeros(256, dtype=np.uint8)
    for biome, classes in legend["koppen"].items():
        for name in classes:
            lut[codes.index(name) + 1] = ids[biome]
    return lut


def _code_mask(koppen: np.ndarray, legend: dict, names: list[str]) -> np.ndarray:
    codes = legend["source"]["codes"]
    wanted = np.zeros(256, dtype=bool)
    for name in names:
        wanted[codes.index(name) + 1] = True
    return wanted[koppen]


# ---------------------------------------------------------------------------
# Fields
# ---------------------------------------------------------------------------


def atlantic_distance_km(inputs: BiomeInputs, boxes: list[list[float]]) -> np.ndarray:
    """Distance (km) to open sea lying in one of the lon/lat ``boxes``."""
    from scipy import ndimage

    step = ATLANTIC_STEP
    rows, cols = inputs.shape
    atlantic = np.zeros(inputs.shape, dtype=bool)
    for lon0, lat0, lon1, lat1 in boxes:
        atlantic |= (
            (inputs.lon >= lon0)
            & (inputs.lon <= lon1)
            & (inputs.lat >= lat0)
            & (inputs.lat <= lat1)
        )
    atlantic &= inputs.water
    small = atlantic[::step, ::step]
    if not small.any():
        return np.full(inputs.shape, 1e6, dtype=np.float32)
    distance = ndimage.distance_transform_edt(~small).astype(np.float32)
    distance *= step * inputs.meters_per_px / 1000.0
    full = np.repeat(np.repeat(distance, step, axis=0), step, axis=1)
    return full[:rows, :cols]


def mountain_threshold_m(lat: np.ndarray, legend: dict) -> np.ndarray:
    """Altitude above which a humid climate is mountain (lower to the north)."""
    rule = legend["rules"]["mountain"]
    return (rule["base_m"] - rule["per_deg_m"] * (lat - rule["ref_lat"])).astype(
        np.float32
    )


# ---------------------------------------------------------------------------
# Classification
# ---------------------------------------------------------------------------


def _apply_dry_and_mountain(
    biome: np.ndarray, inputs: BiomeInputs, legend: dict
) -> np.ndarray:
    """Shared final rules: dry lands to steppe / semi-arid, heights to mountain."""
    ids = indices(legend)
    rules = legend["rules"]
    north = inputs.lat >= rules["steppe_min_lat"]
    humid = (biome == ids["continental"]) | (biome == ids["mediterranean"])
    dry = humid & (inputs.dryness >= rules["dry_min"])
    biome = np.where(dry & north, ids["steppe"], biome)
    biome = np.where(dry & ~north, ids["semi_arid"], biome)

    threshold = mountain_threshold_m(inputs.lat, legend)
    arid = (biome == ids["steppe"]) | (biome == ids["semi_arid"])
    threshold = threshold + np.where(arid, rules["mountain"]["dry_extra_m"], 0.0)
    high = inputs.height_m > threshold
    tundra = biome == ids["mountain"]
    biome = np.where(tundra & ~high, ids[rules["mountain"]["tundra_lowland"]], biome)
    return np.where(high, ids["mountain"], biome).astype(np.uint8)


def classify_koppen(inputs: BiomeInputs, legend: dict) -> np.ndarray:
    """Raw biome of every pixel from the Köppen codes (0 where no data)."""
    ids = indices(legend)
    rules = legend["rules"]
    koppen = inputs.koppen
    biome = koppen_lut(legend)[koppen]

    atlantic = atlantic_distance_km(inputs, rules["atlantic_boxes"])
    far = atlantic > rules["oceanic_max_km"]
    biome = np.where((biome == ids["oceanic"]) & far, ids["continental"], biome)
    del atlantic, far

    cold_b = _code_mask(koppen, legend, ["Dfb", "Dsb", "Dwb"])
    boreal = cold_b & (inputs.lat >= rules["boreal_min_lat"])
    biome = np.where(boreal, ids["boreal"], biome)
    del cold_b, boreal

    dry = biome == ids["semi_arid"]
    north = inputs.lat >= rules["steppe_min_lat"]
    biome = np.where(dry & north, ids["steppe"], biome)
    steppe_codes = _code_mask(koppen, legend, ["BSh", "BSk"])
    coastal = (
        dry
        & ~north
        & steppe_codes
        & (inputs.coast_km <= rules["coastal_semi_arid_km"])
        & (inputs.lon <= rules["coastal_semi_arid_max_lon"])
    )
    biome = np.where(coastal, ids["mediterranean"], biome)
    del dry, north, steppe_codes, coastal
    return _apply_dry_and_mountain(biome, inputs, legend)


def classify_fallback(inputs: BiomeInputs, legend: dict) -> np.ndarray:
    """Rule-based biomes when the Köppen source is unavailable.

    Boreal north of ``boreal_min_lat``; mediterranean south of
    ``fallback.mediterranean_max_lat``, semi-arid there when dry
    (half of ``dry_min``), south of ``semi_arid_max_lat`` and farther than
    ``semi_arid_coast_km`` from the shore; oceanic within ``oceanic_max_km`` of
    the Atlantic; continental elsewhere; then the shared dry / mountain rules.
    """
    ids = indices(legend)
    rules = legend["rules"]
    fallback = legend["fallback"]
    atlantic = atlantic_distance_km(inputs, rules["atlantic_boxes"])
    biome = np.full(inputs.shape, ids["continental"], dtype=np.uint8)
    biome = np.where(atlantic <= rules["oceanic_max_km"], ids["oceanic"], biome)
    south = inputs.lat < fallback["mediterranean_max_lat"]
    biome = np.where(south, ids["mediterranean"], biome)
    semi = (
        south
        & (inputs.lat < fallback["semi_arid_max_lat"])
        & (inputs.coast_km > fallback["semi_arid_coast_km"])
        & (inputs.dryness >= 0.5 * rules["dry_min"])
    )
    biome = np.where(semi, ids["semi_arid"], biome)
    biome = np.where(inputs.lat >= rules["boreal_min_lat"], ids["boreal"], biome)
    return _apply_dry_and_mountain(biome.astype(np.uint8), inputs, legend)


def _fill_nearest(values: np.ndarray, known: np.ndarray) -> np.ndarray:
    """Copy into every unknown pixel the value of the nearest known one."""
    from scipy import ndimage

    if known.all() or not known.any():
        return values
    index = ndimage.distance_transform_edt(
        ~known, return_distances=False, return_indices=True
    )
    return values[index[0], index[1]]


def smooth(biome: np.ndarray, count: int, sigma_px: float, min_px: int) -> np.ndarray:
    """Gaussian blend of the classes 1..count (argmax), then no island below ``min_px``.

    ``biome`` must be filled everywhere (no 0). Deterministic.
    """
    from scipy import ndimage

    out = biome
    if sigma_px > 0:
        best = np.full(biome.shape, -1.0, dtype=np.float32)
        out = np.zeros(biome.shape, dtype=np.uint8)
        for index in range(1, count + 1):
            present = biome == index
            if not present.any():
                continue
            weight = ndimage.gaussian_filter(
                present.astype(np.float32), sigma_px, mode="nearest"
            )
            better = weight > best
            out[better] = index
            best = np.maximum(best, weight)
            del weight, better, present
        del best
    if min_px > 1:
        small = np.zeros(out.shape, dtype=bool)
        structure = np.ones((3, 3), dtype=bool)
        for index in range(1, count + 1):
            labels, n = ndimage.label(out == index, structure=structure)
            if n == 0:
                continue
            sizes = np.bincount(labels.ravel())
            tiny = sizes < min_px
            tiny[0] = False
            small |= tiny[labels]
            del labels
        if small.any() and not small.all():
            out = _fill_nearest(out, ~small)
    return out


def classify(inputs: BiomeInputs, legend: dict) -> np.ndarray:
    """Final biome map (uint8): raw classes, filled, smoothed, sea = 0."""
    if inputs.koppen is not None:
        raw = classify_koppen(inputs, legend)
    else:
        raw = classify_fallback(inputs, legend)
    count = len(legend["classes"]) - 1
    known = inputs.land & (raw > 0)
    if not known.any():
        return np.zeros(inputs.shape, dtype=np.uint8)
    filled = _fill_nearest(raw, known)
    del raw
    km_px = 1000.0 / inputs.meters_per_px
    sigma_px = legend["smoothing"]["sigma_km"] * km_px
    min_px = int(round(legend["smoothing"]["min_area_km2"] * km_px * km_px))
    out = smooth(filled, count, sigma_px, min_px)
    out = np.where(inputs.water, SEA, out).astype(np.uint8)
    return despeckle(out, count)


def despeckle(biome: np.ndarray, count: int) -> np.ndarray:
    """Land pixels alone in their biome among land neighbours take the local majority.

    The Gaussian blend runs on a grid filled across the sea, so a coastal pixel
    can end up the only one of its class along the shore; one-pixel islets (no
    land neighbour) are left alone.
    """
    from scipy import ndimage

    kernel = np.ones((3, 3), dtype=np.int32)
    best = np.zeros(biome.shape, dtype=np.int32)
    majority = biome.copy()
    lonely = np.zeros(biome.shape, dtype=bool)
    others = np.zeros(biome.shape, dtype=np.int32)
    for index in range(1, count + 1):
        mask = biome == index
        if not mask.any():
            continue
        near = ndimage.convolve(mask.astype(np.int32), kernel, mode="nearest")
        lonely |= mask & (near <= 1)
        others += near
        neighbours = near - mask
        better = neighbours > best
        majority[better] = index
        best = np.maximum(best, neighbours)
    lonely &= others > 1
    return np.where(lonely, majority, biome).astype(np.uint8)


def shares(biome: np.ndarray, legend: dict) -> dict[str, float]:
    """Share (%) of the land surface of every land biome."""
    land = biome > 0
    total = max(int(land.sum()), 1)
    counts = np.bincount(biome.ravel(), minlength=256)
    return {
        entry["name"]: round(100.0 * counts[entry["index"]] / total, 2)
        for entry in legend["classes"]
        if entry["index"] > 0
    }


# ---------------------------------------------------------------------------
# Source and inputs
# ---------------------------------------------------------------------------


def koppen_source(legend: dict) -> Path | None:
    """Local GeoTIFF of the Köppen source (downloaded once), or ``None`` on failure."""
    source = legend["source"]
    target = RAW_KOPPEN / source["member"]
    if target.exists():
        return target
    try:
        archive = download.download_file(source["url"], RAW_KOPPEN / source["archive"])
        with zipfile.ZipFile(archive) as zipped:
            zipped.extract(source["member"], RAW_KOPPEN)
    except (download.DownloadError, zipfile.BadZipFile, KeyError, OSError) as error:
        print(f"biomes : source Köppen indisponible ({error}), repli par règles")
        return None
    return target


def reproject_koppen(path: Path, grid: object) -> np.ndarray:
    """Köppen codes resampled (nearest) onto ``grid`` (EPSG:3035)."""
    import rasterio
    from rasterio.enums import Resampling
    from rasterio.warp import reproject
    from rasterio.windows import from_bounds

    from cent_ans_tools.geo.project import CRS_MAP

    lon0, lon1, lat0, lat1 = grid.geographic_extent()
    out = np.zeros(grid.shape, dtype=np.uint8)
    with rasterio.open(path) as src:
        window = (
            from_bounds(lon0 - 1.0, lat0 - 1.0, lon1 + 1.0, lat1 + 1.0, src.transform)
            .round_offsets()
            .round_lengths()
        )
        data = src.read(1, window=window)
        reproject(
            source=data,
            destination=out,
            src_transform=src.window_transform(window),
            src_crs=src.crs,
            dst_transform=grid.transform,
            dst_crs=CRS_MAP,
            resampling=Resampling.nearest,
        )
    return out


def load_inputs(map_dir: Path, legend: dict, use_koppen: bool = True) -> BiomeInputs:
    """Read the map rasters (and the Köppen source) on the grid of ``map.json``."""
    from PIL import Image

    from cent_ans_tools.geo import colormap, landcover, terrain
    from cent_ans_tools.geo.project import grid_from_metadata

    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    grid = grid_from_metadata(meta)
    with Image.open(map_dir / "land_mask.png") as image:
        land = np.asarray(image.convert("L")) > 127
    with Image.open(map_dir / "coast_dist.png") as image:
        coast_px = (np.asarray(image.convert("L")).astype(np.float32) - 128.0) / 2.0
    height = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    height = height.astype(np.float32)
    lon, lat = landcover.pixel_lonlat(grid)
    style = colormap.load_style()
    lake, reservoir = colormap.classify_water(
        land, np.zeros((0, 2)), int(style["lakes"]["max_area_px"])
    )
    water = ~land & ~lake & ~reservoir
    koppen = None
    if use_koppen:
        path = koppen_source(legend)
        if path is not None:
            koppen = reproject_koppen(path, grid)
    return BiomeInputs(
        land=land | lake | reservoir,
        water=water,
        height_m=height,
        lon=lon,
        lat=lat,
        coast_km=coast_px * float(grid.meters_per_px) / 1000.0,
        dryness=landcover.dryness(lon, lat, height, None),
        meters_per_px=float(grid.meters_per_px),
        koppen=koppen,
    )


def read_biomes(map_dir: Path = MAP_DIR) -> np.ndarray | None:
    """``biomes.png`` of ``map_dir`` as uint8 indices, or ``None`` if absent."""
    from PIL import Image

    path = map_dir / BIOMES_NAME
    if not path.exists():
        return None
    with Image.open(path) as image:
        return np.asarray(image.convert("L")).copy()


def build(map_dir: Path = MAP_DIR, legend_path: Path = LEGEND_PATH) -> list[Path]:
    """``geo biomes``: bake ``biomes.png`` and record it in ``map.json``."""
    from PIL import Image

    from cent_ans_tools.geo import block_compress

    started = time.monotonic()
    legend = load_legend(legend_path)
    inputs = load_inputs(map_dir, legend)
    source = legend["source"]["name"] if inputs.koppen is not None else "règles"
    biome = classify(inputs, legend)
    del inputs
    path = map_dir / BIOMES_NAME
    Image.fromarray(biome, mode="L").save(path, optimize=True)
    surface = shares(biome, legend)
    block_compress.update_map_json(
        map_dir,
        MAP_KEY,
        {"file": BIOMES_NAME, "legend": "biomes.yaml", "source": source},
    )
    print(
        f"biomes : {source}, {time.monotonic() - started:.0f} s ; "
        + ", ".join(f"{name} {share:.1f} %" for name, share in surface.items())
    )
    return [path, map_dir / "map.json"]
