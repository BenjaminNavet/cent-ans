"""Tests for the campaign river render data (lot V4, A1-11)."""

import json
from pathlib import Path

import numpy as np

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


def test_crossings_file_still_matches_schema() -> None:
    """The ``structure`` field added to ``crossings.json`` is allowed by the schema."""
    crossings = json.loads(
        (DATA / "map" / "crossings.json").read_text(encoding="utf-8")
    )
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


# ------------------------------------------------------- fine rivers (lot RC4)


def _fine_pyramid(tmp_path: Path, origin: tuple[int, int] = (0, 1)) -> Path:
    """Tiny fine network: three features cut over two E2 tiles (64 units each)."""
    from cent_ans_tools.geo import fine_tiles

    map_dir = tmp_path / "map"
    tiles_dir = map_dir / "pyramid" / "hydro_fine"
    (map_dir / "relief_pyramid.json").parent.mkdir(parents=True, exist_ok=True)
    (map_dir / "relief_pyramid.json").write_text(
        json.dumps({"root_origin_tiles": list(origin)}), encoding="utf-8"
    )
    features = [
        # 0: long order-6 river crossing the tile border at x = 64
        {"name": "La Vienne", "source": "topage", "order": 6, "length_km": 80.0},
        # 1: order 3, filtered out
        {"name": "Ruisseau", "source": "topage", "order": 3, "length_km": 40.0},
        # 2: order 5 but too short
        {"name": "Le Court", "source": "topage", "order": 5, "length_km": 5.0},
        # 3: Natural Earth line (already in rivers.geojson)
        {"name": "Loire", "source": "naturalearth", "order": 7, "length_km": 900.0},
    ]
    line = np.array([[10.0, 20.0], [40.0, 20.0], [100.0, 20.0]])
    tiles: dict = {}
    order_bits = 6 << 24
    for col, row, xy, _att in fine_tiles.split_by_tiles(line, np.zeros((3, 1)), 64.0):
        z = np.zeros(len(xy))
        tiles.setdefault((col, row), fine_tiles.TileLines()).add(
            0, order_bits, xy, z, z
        )
    short = np.array([[5.0, 50.0], [15.0, 50.0]])
    tiles[(0, 0)].add(1, 3 << 24, short, np.zeros(2), np.zeros(2))
    tiles[(0, 0)].add(2, 5 << 24, short + [0, 5], np.zeros(2), np.zeros(2))
    tiles[(0, 0)].add(3, 7 << 24, short + [0, 9], np.zeros(2), np.zeros(2))
    index = fine_tiles.write_tiles(tiles_dir / "E2", fine_tiles.LAYER_RIVERS, 2, tiles)
    (tiles_dir / "features.json").write_text(
        json.dumps({"features": features}), encoding="utf-8"
    )
    (map_dir / "rivers_fine.json").write_text(
        json.dumps(
            {
                "dir": "pyramid/hydro_fine",
                "pattern": "E2/{col}_{row}.bin",
                "features_file": "pyramid/hydro_fine/features.json",
                "tiles": index,
            }
        ),
        encoding="utf-8",
    )
    return map_dir


def test_fine_lines_filtered_chained_and_shifted(tmp_path: Path) -> None:
    """Order / length / source filters, tile pieces joined, pyramid frame → map px."""
    map_dir = _fine_pyramid(tmp_path)
    assert river_render.fine_missing_reason(map_dir) is None
    fine = river_render.read_fine_lines(map_dir, river_render.FineOptions(5, 15.0))
    assert fine.read == 1
    assert len(fine.lines) == 1
    line = fine.lines[0]
    assert line["name"] == "La Vienne"
    assert line["order"] == 6
    points = line["points"]
    # one polyline across the tile border (shared vertex not duplicated)
    np.testing.assert_allclose(points[0], [10.0, 20.0 + 256.0])
    np.testing.assert_allclose(points[-1], [100.0, 20.0 + 256.0])
    assert len(points) == 4  # 3 vertices + the cut at x = 64
    lower = river_render.read_fine_lines(map_dir, river_render.FineOptions(3, 1.0))
    assert {ln["name"] for ln in lower.lines} == {"La Vienne", "Ruisseau", "Le Court"}


def test_fine_missing_pyramid_is_reported(tmp_path: Path) -> None:
    """Without the pyramid, the reason is given (the build goes on without fine rivers)."""
    map_dir = _fine_pyramid(tmp_path)
    reason_manifest = river_render.fine_missing_reason(tmp_path)
    assert reason_manifest is not None and "rivers_fine.json" in reason_manifest
    import shutil

    shutil.rmtree(map_dir / "pyramid")
    reason = river_render.fine_missing_reason(map_dir)
    assert reason is not None and "pyramid/hydro_fine" in reason


def test_chain_pieces_joins_out_of_order() -> None:
    """Pieces listed out of order are joined on their shared vertices."""
    a = np.array([[0.0, 0.0], [1.0, 0.0]])
    b = np.array([[1.0, 0.0], [2.0, 0.0]])
    c = np.array([[2.0, 0.0], [3.0, 1.0]])
    chains = river_render.chain_pieces([c, a, b])
    assert len(chains) == 1
    np.testing.assert_allclose(chains[0][[0, -1]], [[0.0, 0.0], [3.0, 1.0]])
    assert len(chains[0]) == 4


def test_dedup_drops_duplicates_and_keeps_new_runs() -> None:
    """A line along a displayed river is dropped; a tributary keeps its new run."""
    existing = [{"points": np.array([[0.0, 10.0], [100.0, 10.0]])}]
    xs = np.linspace(0.0, 100.0, 51)
    duplicate = {
        "name": "Loire",
        "order": 7,
        "points": np.column_stack([xs, np.full_like(xs, 11.0)]),
    }
    ys = np.linspace(60.0, 10.0, 26)
    tributary = {
        "name": "Le Cher",
        "order": 5,
        "points": np.column_stack([np.full_like(ys, 50.0), ys]),
    }
    kept, dropped = river_render.dedup_fine([duplicate, tributary], existing, 20.0)
    assert dropped == 1
    assert len(kept) == 1
    points = kept[0]["points"]
    assert kept[0]["name"] == "Le Cher"
    assert points[0][1] == 60.0
    # extended by one vertex into the river's band so the tributary reaches it
    assert points[-1][1] <= 10.0 + river_render.FINE_DEDUP_PX
    assert points[-2][1] > 10.0 + river_render.FINE_DEDUP_PX


def test_prepare_fine_low_importance() -> None:
    """Fine rivers get importance 0-2 from their order and the width of that importance."""
    assert [river_render.fine_importance(o) for o in (3, 4, 5, 9)] == [0, 1, 2, 2]
    height = np.zeros((32, 32), dtype=np.float32)
    line = {
        "name": "La Vienne",
        "order": 6,
        "points": np.array([[2.0, 5.0], [16.0, 6.0], [30.0, 5.0]]),
    }
    rivers = river_render.prepare_fine([line], height, STYLES, [])
    assert len(rivers) == 1
    assert rivers[0]["importance"] == 2
    assert rivers[0]["name"] == "La Vienne"
    np.testing.assert_allclose(rivers[0]["widths"], STYLES["width_by_importance"][2])
