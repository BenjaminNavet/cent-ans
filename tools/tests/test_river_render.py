"""Tests for the campaign river render data (lot V4, A1-11)."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator

from cent_ans_tools.geo import river_render

DATA = Path(__file__).resolve().parents[2] / "data"

STYLES = {
    "width_by_importance": [0.1, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
    "taper_min": 0.5,
    "taper_low_m": 0.0,
    "taper_high_m": 1000.0,
    "bank_px": 1.0,
    "rivers": {"Loire": 1.2},
}


def test_styles_file_matches_schema() -> None:
    """``data/map/river_styles.json`` is valid against its schema."""
    schema = json.loads(
        (DATA / "schemas" / "river_styles.schema.json").read_text(encoding="utf-8")
    )
    styles = json.loads(
        (DATA / "map" / "river_styles.json").read_text(encoding="utf-8")
    )
    Draft202012Validator(schema).validate(styles)


def test_crossings_file_still_matches_schema() -> None:
    """The ``structure`` field added to ``crossings.json`` is allowed by the schema."""
    schema = json.loads(
        (DATA / "schemas" / "crossings.schema.json").read_text(encoding="utf-8")
    )
    crossings = json.loads(
        (DATA / "map" / "crossings.json").read_text(encoding="utf-8")
    )
    Draft202012Validator(schema).validate(crossings)
    structures = {
        c.get("structure") for c in crossings["crossings"] if c["type"] != "pass"
    }
    assert structures <= {"stone", "wood", "boats", "ferry", "ford"}


def test_chaikin_keeps_ends_and_doubles_points() -> None:
    """One Chaikin pass keeps both ends and gives 2(n-1) points."""
    line = np.array([[0.0, 0.0], [4.0, 0.0], [4.0, 4.0]])
    smooth = river_render.chaikin(line, 1)
    assert len(smooth) == 4
    np.testing.assert_allclose(smooth[0], line[0])
    np.testing.assert_allclose(smooth[-1], line[-1])


def test_rivers_flow_downhill_and_widen_downstream() -> None:
    """Lines are reversed to start upstream; widths grow as the river descends."""
    size = 32
    height = np.tile(np.linspace(0.0, 1000.0, size, dtype=np.float32), (size, 1))
    feature = {
        "properties": {"name": "Loire", "scalerank": 2},
        "geometry": {
            "type": "LineString",
            "coordinates": [[2.0, 10.0], [16.0, 10.0], [30.0, 10.0]],
        },
    }
    rivers = river_render.prepare_rivers([feature], height, STYLES, [])
    assert len(rivers) == 1
    points = rivers[0]["points"]
    widths = rivers[0]["widths"]
    assert points[0][0] > points[-1][0]  # starts at the high (east) end
    assert widths[0] < widths[-1]
    assert abs(widths[-1] - 1.2) < 0.05


def test_custom_zone_cuts_the_river() -> None:
    """A custom zone splits a line in two runs, none inside the circle."""
    points = np.array([[float(x), 5.0] for x in range(21)])
    widths = np.ones(len(points))
    zone = {"px": [10.0, 5.0], "radius_px": 3.0}
    runs = river_render.split_outside(points, widths, [zone])
    assert len(runs) == 2
    for run_points, _ in runs:
        assert np.all(np.abs(run_points[:, 0] - 10.0) >= 3.0)


def test_bed_is_negative_inside_and_positive_on_banks() -> None:
    """The signed distance is < 0 in the bed, > 0 beyond the bank, saturated far away."""
    river = {
        "points": np.array([[2.0, 8.5], [30.0, 8.5]]),
        "widths": np.array([2.0, 2.0]),
    }
    bed = river_render.compute_bed([river], 32)
    assert bed[8, 16] < -0.5
    assert 0.5 < bed[11, 16] < 3.0
    assert bed[30, 16] == river_render.BED_RANGE_PX
    encoded = river_render.encode_bed(bed)
    assert encoded[8, 16] < 128 < encoded[11, 16]


def test_crossings_snap_onto_their_named_river() -> None:
    """A bridge is moved onto its river, with the river direction and width."""
    loire = {
        "name": "Loire",
        "importance": 6,
        "points": np.array([[0.0, 10.0], [40.0, 10.0]]),
        "widths": np.array([1.0, 1.0]),
    }
    other = {
        "name": "Cher",
        "importance": 3,
        "points": np.array([[0.0, 13.0], [40.0, 13.0]]),
        "widths": np.array([0.5, 0.5]),
    }
    entries = [
        {
            "id": "bridge_test",
            "name": "Test",
            "type": "bridge",
            "structure": "wood",
            "river": "Loire",
            "lonlat": [20.0, 12.5],
        },
        {
            "id": "pass_test",
            "name": "Col",
            "type": "pass",
            "lonlat": [0.0, 0.0],
            "route": [],
        },
    ]
    placed, unsnapped = river_render.place_crossings(
        entries, [loire, other], lambda lon, lat: (lon, lat), []
    )
    assert unsnapped == []
    assert len(placed) == 1
    bridge = placed[0]
    assert bridge["structure"] == "wood"
    assert bridge["px"] == [20.0, 10.0]
    assert abs(abs(bridge["dir"][0]) - 1.0) < 1e-6
    assert bridge["in_custom_zone"] is False
