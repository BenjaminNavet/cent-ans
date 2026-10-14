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

from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path

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
    width_px: int
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


#: The former EPSG:3035 grid, kept to read real rasters (battle relief, climate,
#: land cover). Same class as the campaign grid of the real map: use it explicitly
#: when real data is read, whatever ``map.json`` says (composed map, lot CC).
RealGrid = MapGrid


@lru_cache(maxsize=4)
def _load_warp(directory: str):  # noqa: ANN202
    """Composed warp contract of ``directory`` (cached)."""
    from cent_ans_tools.compose.contract import ComposedWarp

    return ComposedWarp.load(Path(directory))


@dataclass(frozen=True)
class ComposedGrid(MapGrid):
    """Campaign grid of the composed map (``"crs": "composed"``, lot CC).

    Same interface as :class:`MapGrid`: ``lonlat_to_pixel`` goes real longitude /
    latitude -> real EPSG:3035 pixel (:attr:`real`) -> forward warp -> composed
    pixel (times :attr:`scale`); ``pixel_to_lonlat`` is the inverse. ``bounds``
    stay those of the real grid; ``width_px`` / ``height_px`` are composed.
    Code that reads real rasters (``transform``, pixel indexing into real data)
    must use :attr:`real`; ``transform`` raises here on purpose.

    Attributes:
        warp: ``ComposedWarp`` of ``data/map/compose``.
        scale: Pixels per full-resolution composed pixel.
        nominal_mpp: Nominal metres per composed pixel at ``scale`` 1.
    """

    warp: object = field(default=None, compare=False, repr=False)
    scale: float = 1.0
    nominal_mpp: float = 719.0

    @property
    def real(self) -> MapGrid:
        """The former EPSG:3035 grid (real rasters)."""
        return self.warp.real_map_grid()

    @property
    def meters_per_px(self) -> float:
        """Nominal metres per composed pixel (game kilometres), not geodesic."""
        return self.nominal_mpp / self.scale

    def scaled(self, factor: float) -> ComposedGrid:
        """Same warp with ``factor`` times as many pixels per side."""
        return ComposedGrid(
            self.bounds,
            int(round(self.width_px * factor)),
            int(round(self.height_px * factor)),
            self.warp,
            self.scale * factor,
            self.nominal_mpp,
        )

    @property
    def transform(self) -> Affine:
        """Not defined: composed pixels are not an affine image of EPSG:3035."""
        raise NotImplementedError(
            "ComposedGrid has no affine transform; use grid.real for real rasters"
        )

    def projected_to_pixel(
        self, x: np.ndarray | float, y: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """EPSG:3035 metres -> composed pixels (real pixel, then forward warp)."""
        rx, ry = self.real.projected_to_pixel(x, y)
        cx, cy = self.warp.forward_px(rx, ry)
        return cx * self.scale, cy * self.scale

    def pixel_to_projected(
        self, px: np.ndarray | float, py: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Composed pixels -> EPSG:3035 metres (inverse warp, then real grid)."""
        rx, ry = self.warp.inverse_px(
            np.asarray(px) / self.scale, np.asarray(py) / self.scale
        )
        return self.real.pixel_to_projected(rx, ry)

    def lonlat_to_pixel(
        self, lon: np.ndarray | float, lat: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """WGS84 degrees -> composed pixels."""
        x, y = _to_map.transform(lon, lat)
        return self.projected_to_pixel(x, y)

    def pixel_to_lonlat(
        self, px: np.ndarray | float, py: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Composed pixels -> WGS84 degrees."""
        x, y = self.pixel_to_projected(px, py)
        return _to_geo.transform(x, y)

    def geographic_extent(
        self, samples: int = 400
    ) -> tuple[float, float, float, float]:
        """Lon/lat box of the real grid (source clipping reads real data)."""
        return self.real.geographic_extent(samples)


def composed_geometry(geometry, warp, scale: float = 1.0):  # noqa: ANN001, ANN201
    """Real EPSG:3035 shapely geometry drawn in composed pixels (lot CC C4a).

    The geometry is clipped to the extent of the warp (the warp is NaN outside), densified every
    8 real pixels so that edges bend with the deformation, then each vertex goes through the
    forward warp. Empty when it lies outside the extent.
    """
    import shapely
    from shapely.geometry import box

    real = warp.real_map_grid()
    x0, y0, x1, y1 = warp.crop_box_px
    crop = box(x0 + 2, y0 + 2, x1 - 2, y1 - 2)
    px = shapely.transform(
        geometry, lambda c: np.column_stack(real.projected_to_pixel(c[:, 0], c[:, 1]))
    )
    px = px.intersection(crop)
    if px.is_empty:
        return px
    px = shapely.segmentize(px, 8.0)

    def forward(c: np.ndarray) -> np.ndarray:
        x, y = warp.forward_px(c[:, 0], c[:, 1])
        return np.column_stack([np.asarray(x), np.asarray(y)]) * scale

    return shapely.transform(px, forward)


def default_grid() -> MapGrid:
    """The campaign grid (ADR 0115): explicit bounds, 7168 x 6144 px."""
    return MapGrid(BOUNDS_PROJECTED, WIDTH_PX, HEIGHT_PX)


def grid_from_metadata(metadata: dict, scale: float = 1.0) -> MapGrid:
    """Grid of a ``map.json``, scaled.

    ``"crs": "composed"`` (lot CC) gives a :class:`ComposedGrid` whose warp is
    read from ``metadata["composed"]["warp"]`` (relative to ``data/map``); any
    other CRS gives the EPSG:3035 :class:`MapGrid` (``bounds_projected``).
    """
    width, height = metadata["size_px"]
    if metadata.get("crs") == "composed":
        from cent_ans_tools.paths import DATA_DIR

        info = metadata["composed"]
        warp = _load_warp(str((DATA_DIR / "map" / info["warp"]).parent))
        return ComposedGrid(
            tuple(warp.real_grid["bounds_projected"]),
            int(round(width * scale)),
            int(round(height * scale)),
            warp,
            scale,
            float(info["meters_per_px"]),
        )
    return MapGrid(
        tuple(metadata["bounds_projected"]),
        int(round(width * scale)),
        int(round(height * scale)),
    )


def real_grid_from_metadata(metadata: dict, scale: float = 1.0) -> MapGrid:
    """The former EPSG:3035 grid described by a ``map.json`` (real or composed)."""
    block = metadata["real_grid"] if metadata.get("crs") == "composed" else metadata
    width, height = block["size_px"]
    return MapGrid(
        tuple(block["bounds_projected"]),
        int(round(width * scale)),
        int(round(height * scale)),
    )
