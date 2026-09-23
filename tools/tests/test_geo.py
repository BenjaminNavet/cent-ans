"""Tests for the geo grid helpers and the height encoding (no network)."""

import numpy as np
import pytest

from cent_ans_tools.geo import download
from cent_ans_tools.geo.project import MapGrid, default_grid, squared_bounds
from cent_ans_tools.geo.terrain import (
    HEIGHT_MAX_M,
    HEIGHT_MIN_M,
    height_to_uint16,
    uint16_to_height,
)

GRID = MapGrid(bounds=(1000.0, 2000.0, 1000.0 + 4096 * 10.0, 2000.0 + 4096 * 10.0))


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


def test_squared_bounds_pads_shorter_side_symmetrically() -> None:
    """A tall box is widened around its centre; a wide box is heightened."""
    assert squared_bounds((0.0, 0.0, 100.0, 300.0)) == (-100.0, 0.0, 200.0, 300.0)
    assert squared_bounds((0.0, 0.0, 300.0, 100.0)) == (0.0, -100.0, 300.0, 200.0)


def test_default_grid_matches_contract() -> None:
    """The campaign grid is 4096 px, ~719 m/px, and contains the four corners."""
    grid = default_grid()
    assert grid.size_px == 4096
    assert 700.0 < grid.meters_per_px < 740.0
    for lon, lat in ((-11.0, 35.0), (16.0, 35.0), (-11.0, 60.0), (16.0, 60.0)):
        px, py = grid.lonlat_to_pixel(lon, lat)
        assert 0.0 <= px <= 4096.0 and 0.0 <= py <= 4096.0
    # Paris is roughly in the west-centre of the map, Edinburgh north of it.
    paris = grid.lonlat_to_pixel(2.35, 48.86)
    edinburgh = grid.lonlat_to_pixel(-3.19, 55.95)
    assert edinburgh[1] < paris[1] and edinburgh[0] < paris[0]


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
