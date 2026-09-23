"""Map grid definition: EPSG:3035 bounds and pixel <-> projected helpers.

Convention (see ``docs/design/m1-campaign-map.md``): the map is a square raster
of :data:`SIZE_PX` pixels whose origin is the north-west corner of the
projected bounds, X grows eastwards and Y grows southwards, one unit = one
pixel. Continuous pixel coordinates place the centre of pixel ``(i, j)`` at
``(i + 0.5, j + 0.5)``.

The geographic extent (lon -11..16, lat 35..60) is not square once projected
(about 2464 km x 2945 km), so the X extent is widened symmetrically to obtain
square pixels; ``meters_per_px`` is therefore isotropic.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from affine import Affine
from pyproj import Transformer

CRS_MAP = "EPSG:3035"
CRS_GEO = "EPSG:4326"
LON_MIN, LON_MAX = -11.0, 16.0
LAT_MIN, LAT_MAX = 35.0, 60.0
SIZE_PX = 4096

_to_map = Transformer.from_crs(CRS_GEO, CRS_MAP, always_xy=True)
_to_geo = Transformer.from_crs(CRS_MAP, CRS_GEO, always_xy=True)


@dataclass(frozen=True)
class MapGrid:
    """Square raster grid over projected bounds.

    Attributes:
        bounds: ``(minx, miny, maxx, maxy)`` in EPSG:3035 metres.
        size_px: Width and height in pixels.
    """

    bounds: tuple[float, float, float, float]
    size_px: int = SIZE_PX

    @property
    def meters_per_px(self) -> float:
        """Isotropic pixel size in metres."""
        return (self.bounds[2] - self.bounds[0]) / self.size_px

    @property
    def transform(self) -> Affine:
        """Rasterio affine transform (projected coordinates of pixel corners)."""
        minx, _, _, maxy = self.bounds
        mpp = self.meters_per_px
        return Affine(mpp, 0.0, minx, 0.0, -mpp, maxy)

    def projected_to_pixel(
        self, x: np.ndarray | float, y: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Convert EPSG:3035 metres into continuous pixel coordinates.

        Args:
            x: Easting(s) in metres.
            y: Northing(s) in metres.

        Returns:
            ``(px, py)`` with ``px`` eastwards and ``py`` southwards.
        """
        minx, _, _, maxy = self.bounds
        mpp = self.meters_per_px
        return (np.asarray(x) - minx) / mpp, (maxy - np.asarray(y)) / mpp

    def pixel_to_projected(
        self, px: np.ndarray | float, py: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Inverse of :meth:`projected_to_pixel`."""
        minx, _, _, maxy = self.bounds
        mpp = self.meters_per_px
        return minx + np.asarray(px) * mpp, maxy - np.asarray(py) * mpp

    def lonlat_to_pixel(
        self, lon: np.ndarray | float, lat: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Project WGS84 degrees straight into pixel coordinates."""
        x, y = _to_map.transform(lon, lat)
        return self.projected_to_pixel(x, y)

    def pixel_to_lonlat(
        self, px: np.ndarray | float, py: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Inverse of :meth:`lonlat_to_pixel`."""
        x, y = self.pixel_to_projected(px, py)
        return _to_geo.transform(x, y)

    def geographic_extent(
        self, samples: int = 400
    ) -> tuple[float, float, float, float]:
        """Lon/lat bounding box of the projected bounds (for source clipping).

        Args:
            samples: Points sampled along each edge.

        Returns:
            ``(lon_min, lon_max, lat_min, lat_max)`` in degrees.
        """
        minx, miny, maxx, maxy = self.bounds
        xs = np.linspace(minx, maxx, samples)
        ys = np.linspace(miny, maxy, samples)
        edge_x = np.concatenate(
            [xs, np.full(samples, maxx), xs, np.full(samples, minx)]
        )
        edge_y = np.concatenate(
            [np.full(samples, miny), ys, np.full(samples, maxy), ys]
        )
        lon, lat = _to_geo.transform(edge_x, edge_y)
        return float(lon.min()), float(lon.max()), float(lat.min()), float(lat.max())


def projected_bounds_of_extent(
    lon_min: float = LON_MIN,
    lon_max: float = LON_MAX,
    lat_min: float = LAT_MIN,
    lat_max: float = LAT_MAX,
    samples: int = 400,
) -> tuple[float, float, float, float]:
    """Projected bounding box of a geographic rectangle (edges densified)."""
    lons = np.linspace(lon_min, lon_max, samples)
    lats = np.linspace(lat_min, lat_max, samples)
    edge_lon = np.concatenate(
        [lons, np.full(samples, lon_max), lons, np.full(samples, lon_min)]
    )
    edge_lat = np.concatenate(
        [np.full(samples, lat_min), lats, np.full(samples, lat_max), lats]
    )
    x, y = _to_map.transform(edge_lon, edge_lat)
    return float(x.min()), float(y.min()), float(x.max()), float(y.max())


def squared_bounds(
    bounds: tuple[float, float, float, float],
) -> tuple[float, float, float, float]:
    """Widen the shorter side of ``bounds`` symmetrically so the box is square.

    Coordinates are rounded to whole metres (outwards).
    """
    minx, miny, maxx, maxy = bounds
    width, height = maxx - minx, maxy - miny
    if width < height:
        pad = (height - width) / 2
        minx, maxx = minx - pad, maxx + pad
    elif height < width:
        pad = (width - height) / 2
        miny, maxy = miny - pad, maxy + pad
    side = float(np.ceil(max(maxx - minx, maxy - miny)))
    minx, miny = float(np.floor(minx)), float(np.floor(miny))
    return minx, miny, minx + side, miny + side


def default_grid() -> MapGrid:
    """The M1 campaign grid: contract extent, squared, 4096 px."""
    return MapGrid(squared_bounds(projected_bounds_of_extent()))
