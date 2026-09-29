"""Tests for the geo grid helpers and the height encoding (no network)."""

import numpy as np
import pytest

from cent_ans_tools.geo import download
from cent_ans_tools.geo.project import (
    BOUNDS_PROJECTED,
    MapGrid,
    default_grid,
    grid_from_metadata,
)
from cent_ans_tools.geo.terrain import (
    HEIGHT_MAX_M,
    HEIGHT_MIN_M,
    height_to_uint16,
    uint16_to_height,
)

GRID = MapGrid(
    bounds=(1000.0, 2000.0, 1000.0 + 4096 * 10.0, 2000.0 + 4096 * 10.0), width_px=4096
)


def test_grid_is_square_with_isotropic_pixels() -> None:
    """Pixel size derives from the bounds width and the pixel count."""
    assert GRID.meters_per_px == 10.0
    assert GRID.transform @ (0, 0) == (1000.0, 2000.0 + 40960.0)


def test_origin_is_north_west_and_y_grows_south() -> None:
    """The NW corner maps to (0, 0); the SE corner to (size, size)."""
    minx, miny, maxx, maxy = GRID.bounds
    assert GRID.projected_to_pixel(minx, maxy) == (0.0, 0.0)
    assert GRID.projected_to_pixel(maxx, miny) == (4096.0, 4096.0)
    px, py = GRID.projected_to_pixel(minx + 15.0, maxy - 25.0)
    assert (px, py) == (1.5, 2.5)


def test_pixel_projected_round_trip() -> None:
    """Round trips are exact to floating point noise, on arrays too."""
    px = np.array([0.0, 12.25, 4095.5])
    py = np.array([0.0, 7.75, 4096.0])
    x, y = GRID.pixel_to_projected(px, py)
    back = GRID.projected_to_pixel(x, y)
    np.testing.assert_allclose(back[0], px)
    np.testing.assert_allclose(back[1], py)


def test_rectangular_grid_shape_and_corners() -> None:
    """A rectangular grid keeps isotropic pixels; the SE corner maps to (w, h)."""
    grid = MapGrid((0.0, 0.0, 700.0, 600.0), 7, 6)
    assert grid.shape == (6, 7)
    assert grid.meters_per_px == 100.0
    assert grid.projected_to_pixel(700.0, 0.0) == (7.0, 6.0)
    assert grid.scaled(2).shape == (12, 14)
    assert grid_from_metadata(
        {"bounds_projected": [0, 0, 700, 600], "size_px": [7, 6]}, 2
    ).shape == (12, 14)


def test_default_grid_matches_contract() -> None:
    """ADR 0115: 7168 x 6144 units of 718.9765625 m, explicit EPSG:3035 bounds."""
    grid = default_grid()
    assert (grid.width_px, grid.height_px) == (7168, 6144)
    assert grid.meters_per_px == 718.9765625
    assert grid.bounds == BOUNDS_PROJECTED == (2169486.0, 775684.0, 7323110.0, 5193076.0)
    assert (grid.bounds[3] - grid.bounds[1]) / grid.height_px == grid.meters_per_px
    # Coverage (spec OM): Atlantic Morocco, the Urals, the White Sea, the Nile delta.
    for lon, lat in (
        (-9.6, 30.4),  # Agadir
        (60.6, 56.8),  # Iekaterinbourg
        (55.1, 51.8),  # Orenbourg
        (40.5, 64.5),  # Arkhangelsk
        (31.2, 30.0),  # Le Caire
        (48.03, 46.35),  # Astrakhan
        (42.44, 43.35),  # Elbrouz
        (16.6, 31.2),  # Syrte
        (-11.0, 60.0),
    ):
        px, py = grid.lonlat_to_pixel(lon, lat)
        assert 0.0 <= px <= 7168.0 and 0.0 <= py <= 6144.0, (lon, lat)
    # Paris is in the west of the map, Edinburgh north-west of it.
    paris = grid.lonlat_to_pixel(2.35, 48.86)
    edinburgh = grid.lonlat_to_pixel(-3.19, 55.95)
    assert edinburgh[1] < paris[1] and edinburgh[0] < paris[0]


def test_legacy_pixels_shift_by_1280_rows() -> None:
    """Former 4096² pixels keep x and gain 1280 in y (same west edge and scale)."""
    legacy = MapGrid((2169486.0, 1327858.0, 5114414.0, 4272786.0), 4096)
    grid = default_grid()
    for lon, lat in ((2.35, 48.86), (-10.0, 36.0), (15.9, 59.9)):
        old = legacy.lonlat_to_pixel(lon, lat)
        new = grid.lonlat_to_pixel(lon, lat)
        assert abs(float(new[0]) - float(old[0])) < 1e-6
        assert abs(float(new[1]) - float(old[1]) - 1280.0) < 1e-6


def test_height_encoding_endpoints_and_clamping() -> None:
    """-200 m -> 0, 4800 m -> 65535, out-of-range values clamp."""
    assert height_to_uint16(HEIGHT_MIN_M) == 0
    assert height_to_uint16(HEIGHT_MAX_M) == 65535
    assert height_to_uint16(-5000.0) == 0
    assert height_to_uint16(9000.0) == 65535
    assert height_to_uint16(0.0) == round(200 / 5000 * 65535)


def test_height_round_trip_within_quantisation() -> None:
    """Decoding an encoded height is exact to half a quantisation step (~3.8 cm)."""
    heights = np.array([-200.0, -12.5, 0.0, 123.4, 2500.0, 4800.0])
    decoded = uint16_to_height(height_to_uint16(heights))
    step = (HEIGHT_MAX_M - HEIGHT_MIN_M) / 65535
    assert np.all(np.abs(decoded - heights) <= step / 2 + 1e-9)


@pytest.mark.parametrize(
    ("lon_west", "lat_north", "expected"),
    [
        (-15, 60, "ETOPO_2022_v1_15s_N60W015_surface.tif"),
        (0, 45, "ETOPO_2022_v1_15s_N45E000_surface.tif"),
    ],
)
def test_etopo_tile_names(lon_west: int, lat_north: int, expected: str) -> None:
    """Tiles are named after their north-west corner."""
    assert download.etopo_tile_name(lon_west, lat_north) == expected


def test_etopo_tiles_covering_extent() -> None:
    """An extent inside one tile yields that tile; a straddling one yields both."""
    assert download.etopo_tiles_covering(-10, -1, 46, 59) == [
        "ETOPO_2022_v1_15s_N60W015_surface.tif"
    ]
    assert download.etopo_tiles_covering(-1, 1, 46, 59) == [
        "ETOPO_2022_v1_15s_N60W015_surface.tif",
        "ETOPO_2022_v1_15s_N60E000_surface.tif",
    ]


def test_etopo_tiles_for_grid_keep_only_intersecting_tiles() -> None:
    """The map's curved lon/lat outline keeps 26 of the 32 bounding-box tiles."""
    names = download.etopo_tiles_for_grid(default_grid())
    assert len(names) == 26
    assert "ETOPO_2022_v1_15s_N45E045_surface.tif" in names  # Caucase, Caspienne
    assert "ETOPO_2022_v1_15s_N30E030_surface.tif" in names  # delta du Nil
    assert "ETOPO_2022_v1_15s_N30W045_surface.tif" not in names  # plein Atlantique
