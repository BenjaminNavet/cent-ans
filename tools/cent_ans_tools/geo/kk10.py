"""KK10 anthropogenic land cover change, extracted for the map around AD 1340 (lot R1).

Source: Kaplan, J. O., Krumhardt, K. M., Ellis, E. C., Ruddiman, W. F., Lemmen, C.
and Klein Goldewijk, K. (2011), *Holocene carbon emissions as a result of
anthropogenic land cover change*, The Holocene 21(5), 775-791,
doi:10.1177/0959683610386983. Data: Kaplan, J. O. and Krumhardt, K. M. (2017),
*The KK10 Anthropogenic Land Cover Change scenario for the preindustrial
Holocene*, PANGAEA, doi:10.1594/PANGAEA.871369 (CC-BY 3.0).

The full file (``KK10.nc``, netCDF4, 18.5 GB, annual 8000 BC - AD 1850, 5′ grid)
is never downloaded: HDF5 chunks are read over HTTP range requests with
``h5py`` + ``fsspec`` (not project dependencies, pulled in for this one
command: ``uv run --project tools --with h5py --with fsspec --with aiohttp
cent-ans geo kk10``). The result, the mean fraction of each 5′ cell under
anthropogenic land use over :data:`YEARS`, is cached as a small ``.npz`` in
``tools/geo/raw/kk10/`` and read by :mod:`cent_ans_tools.geo.landcover`.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import download

KK10_URL = "https://hs.pangaea.de/model/ALCC/KK10.nc"
RAW_DIR = download.RAW_DIR / "kk10"
CACHE_FILE = RAW_DIR / "kk10_1330_1349_europe.npz"
#: Years (AD) averaged: the generation before the Black Death, around 1337.
YEARS = (1330, 1349)
#: ``(lon_min, lat_min, lon_max, lat_max)`` extracted (map extent plus a margin).
BBOX = (-12.0, 34.0, 17.0, 61.0)


@dataclass(frozen=True)
class LandUse:
    """Fraction of each cell under anthropogenic land use (0-1, NaN at sea)."""

    fraction: np.ndarray
    lon: np.ndarray  # cell centres, ascending
    lat: np.ndarray  # cell centres, as stored (see ``lat_descending``)

    @property
    def lat_descending(self) -> bool:
        """True when row 0 is the northernmost row."""
        return bool(self.lat[0] > self.lat[-1])


def extract(url: str = KK10_URL, target: Path = CACHE_FILE) -> Path:
    """Read the KK10 years :data:`YEARS` over :data:`BBOX` by HTTP range requests.

    Requires ``h5py``, ``fsspec`` and ``aiohttp`` (see the module docstring).
    """
    import fsspec  # noqa: PLC0415 - optional, see module docstring
    import h5py  # noqa: PLC0415

    target.parent.mkdir(parents=True, exist_ok=True)
    with (
        fsspec.open(url, block_size=4 * 2**20).open() as remote,
        h5py.File(remote, "r") as handle,
    ):
        lon = handle["lon"][:]
        lat = handle["lat"][:]
        years = handle["year"][:]
        lon_min, lat_min, lon_max, lat_max = BBOX
        cols = np.nonzero((lon >= lon_min) & (lon <= lon_max))[0]
        rows = np.nonzero((lat >= lat_min) & (lat <= lat_max))[0]
        steps = np.nonzero((years >= YEARS[0]) & (years <= YEARS[1]))[0]
        dataset = handle["land_use"]
        block = dataset[
            steps.min() : steps.max() + 1,
            rows.min() : rows.max() + 1,
            cols.min() : cols.max() + 1,
        ].astype(np.float32)
        fill = float(dataset.attrs.get("_FillValue", [-32768])[0])
        scale = float(dataset.attrs.get("scale_factor", [1e-4])[0])
    block[block == fill] = np.nan
    fraction = np.nanmean(block, axis=0) * scale
    np.savez_compressed(
        target,
        fraction=fraction.astype(np.float32),
        lon=lon[cols.min() : cols.max() + 1],
        lat=lat[rows.min() : rows.max() + 1],
    )
    return target


def load(path: Path = CACHE_FILE) -> LandUse | None:
    """Cached extraction, ``None`` if :func:`extract` has not been run."""
    if not path.exists():
        return None
    with np.load(path) as data:
        return LandUse(
            fraction=data["fraction"].astype(np.float32),
            lon=data["lon"].astype(np.float64),
            lat=data["lat"].astype(np.float64),
        )
