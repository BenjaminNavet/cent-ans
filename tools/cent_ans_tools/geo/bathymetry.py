"""Bathymétrie de la carte de campagne (lot VIS-R5, ADR « vis-r5-eau »).

Source : GEBCO 2025 Grid (© GEBCO Compilation Group 2025, usage libre avec attribution ;
https://www.gebco.net), tuiles GeoTIFF 90° x 90° de 15 secondes d'arc (int16, mètres, lignes
non compressées) servies par le BODC / CEDA. Seules les lignes utiles (latitudes 27 à 67 N) sont
téléchargées, en plages d'octets parallèles, dans ``~/dev/cent-ans-raw/gebco/`` (hors dépôt).

Sortie : ``data/map/bathymetry.png`` (L8, un texel = ``factor`` pixels de la carte composée) et
``data/map/bathymetry.json`` (encodage). Valeur codée ``v = 255 * sqrt(profondeur / max_depth_m)``,
profondeur en mètres, 0 à la côte. Le shader de mer (``sea.wesl``) la lit.

La carte composée déforme le monde réel (cartogramme) et modifie ses côtes : la profondeur réelle
est donc relue au point réel qui correspond à chaque pixel composé (``ComposedWarp.inverse``), et
les pixels de mer composée qui tombent sur la terre réelle prennent la profondeur de la mer réelle
la plus proche.
"""

from __future__ import annotations

import json
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

import httpx
import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.geo import splat
from cent_ans_tools.paths import DATA_DIR

Image.MAX_IMAGE_PIXELS = None

MAP_DIR = DATA_DIR / "map"
RAW_DIR = Path.home() / "dev" / "cent-ans-raw" / "gebco"
GEBCO_BASE = (
    "https://dap.ceda.ac.uk/bodc/gebco/global/gebco_2025/ice_surface_elevation/geotiff"
)
#: Tuiles de l'hémisphère nord : ouest (lon -90 à 0) et est (lon 0 à 90).
TILE_WEST = "gebco_2025_n90.0_s0.0_w-90.0_e0.0.tif"
TILE_EAST = "gebco_2025_n90.0_s0.0_w0.0_e90.0.tif"
TILE_SIZE = 21600
CELLS_PER_DEG = 240
ROW_BYTES = TILE_SIZE * 2
#: Octet du début des données (lignes d'un pixel de haut, contiguës) dans les tuiles GEBCO 2025.
DATA_OFFSET = 135948
#: Emprise lue (degrés) : la carte composée couvre lon -11 à 45, lat 28 à 66, avec une marge.
LON_MIN, LON_MAX = -11.0, 46.0
LAT_MIN, LAT_MAX = 27.0, 67.0
#: Réduction de la source (15" -> 1') avant l'échantillonnage.
SOURCE_BLOCK = 4
FETCH_ROWS_PER_REQUEST = 160
FETCH_WORKERS = 8
#: Profondeur maximale codée (m) et pixels de carte par texel (1/4 de la carte de couleur).
MAX_DEPTH_M = 4000.0
FACTOR = 4
SMOOTH_SIGMA = 0.8


@dataclass
class BathymetryResult:
    """Sorties de :func:`build`."""

    png: Path
    meta: Path
    size: tuple[int, int]
    sea_share: float


def _row_range() -> tuple[int, int]:
    """Lignes (premières, dernières exclue) des tuiles qui couvrent ``LAT_MIN`` à ``LAT_MAX``."""
    first = int((90.0 - LAT_MAX) * CELLS_PER_DEG)
    last = int((90.0 - LAT_MIN) * CELLS_PER_DEG)
    return first, last


def fetch(directory: Path = RAW_DIR, workers: int = FETCH_WORKERS) -> list[Path]:
    """Télécharge les lignes utiles des deux tuiles dans des fichiers creux (plages d'octets)."""
    directory.mkdir(parents=True, exist_ok=True)
    first, last = _row_range()
    paths = []
    for name in (TILE_WEST, TILE_EAST):
        path = directory / name
        done = directory / f"{name}.rows-{first}-{last}.ok"
        paths.append(path)
        if done.exists() and path.exists():
            continue
        url = f"{GEBCO_BASE}/{name}"
        with httpx.Client(follow_redirects=True, timeout=120.0) as client:
            size = int(client.head(url).headers["content-length"])
            if not path.exists() or path.stat().st_size != size:
                with path.open("wb") as handle:
                    handle.truncate(size)
            jobs = []
            for start in range(first, last, FETCH_ROWS_PER_REQUEST):
                stop = min(last, start + FETCH_ROWS_PER_REQUEST)
                jobs.append(
                    (
                        DATA_OFFSET + start * ROW_BYTES,
                        DATA_OFFSET + stop * ROW_BYTES - 1,
                    )
                )
            jobs.insert(0, (0, DATA_OFFSET + ROW_BYTES - 1))

            def get(job: tuple[int, int], url: str = url, path: Path = path) -> None:
                lo, hi = job
                for attempt in range(5):
                    try:
                        with httpx.Client(follow_redirects=True, timeout=180.0) as one:
                            response = one.get(
                                url, headers={"Range": f"bytes={lo}-{hi}"}
                            )
                        response.raise_for_status()
                        if len(response.content) != hi - lo + 1:
                            raise OSError("plage incomplète")
                        with path.open("r+b") as handle:
                            handle.seek(lo)
                            handle.write(response.content)
                        return
                    except (httpx.HTTPError, OSError):
                        if attempt == 4:
                            raise

            with ThreadPoolExecutor(workers) as pool:
                list(pool.map(get, jobs))
        done.write_text("ok\n", encoding="utf-8")
    return paths


def read_depth_grid(directory: Path = RAW_DIR) -> np.ndarray:
    """Profondeur moyenne (m, NaN sur terre) à 1' sur lon ``LON_MIN``..``LON_MAX``, lat ``LAT_MAX``..``LAT_MIN``.

    Ligne ``r`` : latitude ``LAT_MAX - (r + 0.5) / 60`` ; colonne ``c`` : longitude
    ``LON_MIN + (c + 0.5) / 60``. La moyenne ne compte que les cellules de mer ; un bloc dont moins
    d'un quart est en mer reste NaN (terre).
    """
    import rasterio
    from rasterio.windows import Window

    first, last = _row_range()
    rows = last - first
    west_cols = int(-LON_MIN * CELLS_PER_DEG)
    east_cols = int(LON_MAX * CELLS_PER_DEG)
    elev = np.empty((rows, west_cols + east_cols), dtype=np.int16)
    with rasterio.open(directory / TILE_WEST) as west:
        elev[:, :west_cols] = west.read(
            1, window=Window(TILE_SIZE - west_cols, first, west_cols, rows)
        )
    with rasterio.open(directory / TILE_EAST) as east:
        elev[:, west_cols:] = east.read(1, window=Window(0, first, east_cols, rows))
    block = SOURCE_BLOCK
    rows_b, cols_b = rows // block, elev.shape[1] // block
    elev = elev[: rows_b * block, : cols_b * block]
    sea = elev < 0
    depth = np.where(sea, -elev.astype(np.float32), 0.0)
    sum_depth = depth.reshape(rows_b, block, cols_b, block).sum(axis=(1, 3))
    count = sea.reshape(rows_b, block, cols_b, block).sum(axis=(1, 3))
    out = np.full((rows_b, cols_b), np.nan, dtype=np.float32)
    ok = count >= (block * block) // 4
    out[ok] = sum_depth[ok] / count[ok]
    return out


def fill_nearest(grid: np.ndarray) -> np.ndarray:
    """Remplace les NaN par la valeur valide la plus proche."""
    invalid = np.isnan(grid)
    if not invalid.any():
        return grid
    _, (rows, cols) = ndimage.distance_transform_edt(invalid, return_indices=True)
    return grid[rows, cols]


def composed_sea_signed(map_dir: Path = MAP_DIR, factor: int = FACTOR) -> np.ndarray:
    """Distance signée à la côte (px carte, négative en mer) moyennée par blocs de ``factor``."""
    with Image.open(map_dir / "coast_dist.png") as image:
        coast = np.asarray(image.convert("L"), dtype=np.float32)
    signed = (coast - 128.0) / splat.COAST_DIST_SCALE
    rows, cols = (
        (signed.shape[0] // factor) * factor,
        (signed.shape[1] // factor) * factor,
    )
    signed = signed[:rows, :cols]
    return signed.reshape(rows // factor, factor, cols // factor, factor).mean(
        axis=(1, 3)
    )


def sample_composed(
    depth_filled: np.ndarray,
    shape: tuple[int, int],
    factor: int = FACTOR,
    map_dir: Path = MAP_DIR,
) -> np.ndarray:
    """Profondeur (m) au centre de chaque texel de la carte composée (réel -> composé inverse)."""
    warp = ComposedWarp.load(map_dir / "compose")
    rows, cols = shape
    yy, xx = np.mgrid[0:rows, 0:cols].astype(np.float64)
    x, y = (xx + 0.5) * factor, (yy + 0.5) * factor
    lon, lat = warp.inverse(x, y)
    col = (lon - LON_MIN) * 60.0 - 0.5
    row = (LAT_MAX - lat) * 60.0 - 0.5
    out = ndimage.map_coordinates(depth_filled, [row, col], order=1, mode="nearest")
    return np.where(np.isfinite(lon), out, MAX_DEPTH_M).astype(np.float32)


def encode(depth_m: np.ndarray, max_depth_m: float = MAX_DEPTH_M) -> np.ndarray:
    """``uint8`` : ``255 * sqrt(profondeur / max_depth_m)``."""
    ratio = np.clip(depth_m / max_depth_m, 0.0, 1.0)
    return np.rint(255.0 * np.sqrt(ratio)).astype(np.uint8)


def decode(encoded: np.ndarray, max_depth_m: float = MAX_DEPTH_M) -> np.ndarray:
    """Inverse de :func:`encode` (comme le shader)."""
    unit = encoded.astype(np.float32) / 255.0
    return unit * unit * max_depth_m


def build(map_dir: Path = MAP_DIR, directory: Path = RAW_DIR) -> BathymetryResult:
    """Écrit ``bathymetry.png`` et ``bathymetry.json`` (télécharge GEBCO si besoin)."""
    fetch(directory)
    grid = fill_nearest(read_depth_grid(directory))
    signed = composed_sea_signed(map_dir)
    depth = sample_composed(grid, signed.shape, map_dir=map_dir)
    depth = ndimage.gaussian_filter(depth, SMOOTH_SIGMA)
    sea = signed < 0.0
    depth = np.where(sea, depth, 0.0)
    png = map_dir / "bathymetry.png"
    Image.fromarray(encode(depth), mode="L").save(png, compress_level=9)
    meta_path = map_dir / "bathymetry.json"
    meta = {
        "description": (
            "Bathymétrie cuite par `cent-ans geo bathymetry` (VIS-R5). Valeur L8 v = 255 * "
            "sqrt(profondeur / max_depth_m) ; un texel = `factor` pixels de la carte ; 0 sur terre."
        ),
        "source": "GEBCO 2025 Grid, GEBCO Compilation Group (2025), https://www.gebco.net",
        "licence": "Usage libre, attribution obligatoire (GEBCO)",
        "encoding": "sqrt",
        "max_depth_m": MAX_DEPTH_M,
        "factor": FACTOR,
        "size_px": [int(depth.shape[1]), int(depth.shape[0])],
    }
    meta_path.write_text(
        json.dumps(meta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return BathymetryResult(
        png=png,
        meta=meta_path,
        size=(depth.shape[1], depth.shape[0]),
        sea_share=float(sea.mean()),
    )
