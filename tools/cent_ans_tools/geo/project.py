"""Map grid definition: EPSG:3035 bounds and pixel <-> projected helpers.

Convention (see ``docs/design/m1-campaign-map.md``): the map is a rectangular
raster of :data:`WIDTH_PX` x :data:`HEIGHT_PX` pixels whose origin is the
north-west corner of the projected bounds, X grows eastwards and Y grows
southwards, one unit = one pixel. Continuous pixel coordinates place the centre
of pixel ``(i, j)`` at ``(i + 0.5, j + 0.5)``; ``meters_per_px`` is isotropic.

Extent (ADR 0115, Urals-Mediterranean): the projected bounds are fixed
explicitly (:data:`BOUNDS_PROJECTED`), 28 x 24 root tiles of 256 units at
718.9765625 m per unit. The west edge is the one of the former 4096² map, whose
pixels keep their X and gain 1280 in Y. :data:`EXTENT_LONLAT` is indicative only.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from affine import Affine
from pyproj import Transformer

CRS_MAP = "EPSG:3035"
CRS_GEO = "EPSG:4326"
#: Indicative geographic extent (lon_min, lat_min, lon_max, lat_max): the map is
#: defined by :data:`BOUNDS_PROJECTED`, which covers more than this box.
EXTENT_LONLAT = (-11.0, 28.0, 61.0, 66.0)
LON_MIN, LAT_MIN, LON_MAX, LAT_MAX = EXTENT_LONLAT
METERS_PER_PX = 718.9765625
WIDTH_PX = 7168
HEIGHT_PX = 6144
#: EPSG:3035 bounds ``(minx, miny, maxx, maxy)`` in metres (ADR 0115).
BOUNDS_PROJECTED = (2169486.0, 775684.0, 7323110.0, 5193076.0)
#: Rows added above the former 4096² map: legacy pixel ``(x, y)`` is ``(x, y + 1280)``.
LEGACY_Y_OFFSET_PX = 1280

_to_map = Transformer.from_crs(CRS_GEO, CRS_MAP, always_xy=True)
_to_geo = Transformer.from_crs(CRS_MAP, CRS_GEO, always_xy=True)


@dataclass(frozen=True)
class MapGrid:
    """Raster grid over projected bounds (isotropic pixels).

    Attributes:
        bounds: ``(minx, miny, maxx, maxy)`` in EPSG:3035 metres.
        width_px: Width in pixels.
        height_px: Height in pixels (defaults to a square grid).
    """

    bounds: tuple[float, float, float, float]
    width_px: int = WIDTH_PX
    height_px: int | None = None

    def __post_init__(self) -> None:
        """Default a missing height to the width (square grid)."""
        if self.height_px is None:
            object.__setattr__(self, "height_px", self.width_px)

    @property
    def shape(self) -> tuple[int, int]:
        """Array shape ``(rows, cols)`` = ``(height_px, width_px)``."""
        return (self.height_px, self.width_px)

    @property
    def meters_per_px(self) -> float:
        """Isotropic pixel size in metres."""
        return (self.bounds[2] - self.bounds[0]) / self.width_px

    def scaled(self, factor: float) -> MapGrid:
        """Same bounds with ``factor`` times as many pixels per side."""
        return MapGrid(
            self.bounds,
            int(round(self.width_px * factor)),
            int(round(self.height_px * factor)),
        )

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


def default_grid() -> MapGrid:
    """The campaign grid (ADR 0115): explicit bounds, 7168 x 6144 px."""
    return MapGrid(BOUNDS_PROJECTED, WIDTH_PX, HEIGHT_PX)


def grid_from_metadata(metadata: dict, scale: float = 1.0) -> MapGrid:
    """Grid of a ``map.json`` (``bounds_projected``, ``size_px`` [w, h]), scaled."""
    width, height = metadata["size_px"]
    return MapGrid(
        tuple(metadata["bounds_projected"]),
        int(round(width * scale)),
        int(round(height * scale)),
    )
