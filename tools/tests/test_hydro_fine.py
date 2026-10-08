"""Fine hydrography and anchors of lot ZG5a (ADR 0036): pure algorithms, no network."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest

from cent_ans_tools.geo import fine_tiles, hydro_sources, valley_snap

DATA = Path(__file__).resolve().parents[2] / "data"


# ------------------------------------------------------------ valley snapping


def _thalweg_x(y: np.ndarray) -> np.ndarray:
    return 60.0 * np.sin(np.asarray(y) / 700.0)


def _valley(x: np.ndarray, y: np.ndarray) -> np.ndarray:
    """V-shaped valley meandering around ``_thalweg_x``, sloping down towards +y."""
    return 0.08 * np.abs(np.asarray(x) - _thalweg_x(y)) + 200.0 - 0.002 * np.asarray(y)


def test_snap_moves_line_to_synthetic_thalweg() -> None:
    """A line drawn 45 m off the valley floor comes back within a few metres."""
    y = np.linspace(0.0, 5000.0, 60)
    source = np.column_stack([_thalweg_x(y) + 45.0, y])
    points = valley_snap.resample(source, 20.0)
    params = valley_snap.SnapParams(radius_m=90.0, offsets=37, prior_m=1.0)
    snapped, floor, offsets = valley_snap.snap_to_valley(points, _valley, params)
    error_before = np.abs(points[:, 0] - _thalweg_x(points[:, 1]))
    error_after = np.abs(snapped[:, 0] - _thalweg_x(snapped[:, 1]))
    assert error_before.mean() > 40.0
    assert error_after.mean() < 6.0
    assert np.all(np.abs(offsets) <= 90.0 + 1e-6)
    assert floor.shape == (len(points),)


def test_snap_keeps_line_on_flat_ground() -> None:
    """Without relief the prior keeps the source geometry."""
    points = valley_snap.resample(np.array([[0.0, 0.0], [0.0, 1000.0]]), 20.0)
    snapped, _, offsets = valley_snap.snap_to_valley(
        points, lambda x, y: np.zeros_like(x), valley_snap.SnapParams()
    )
    assert np.allclose(offsets, 0.0)
    assert np.allclose(snapped, points)


def test_snap_fixed_ends() -> None:
    """Pinned ends do not move."""
    y = np.linspace(0.0, 2000.0, 20)
    points = valley_snap.resample(np.column_stack([_thalweg_x(y) + 30.0, y]), 20.0)
    snapped, _, _ = valley_snap.snap_to_valley(
        points, _valley, valley_snap.SnapParams(), fixed_ends=True
    )
    assert np.allclose(snapped[0], points[0])
    assert np.allclose(snapped[-1], points[-1])


def test_resample_spacing() -> None:
    """Vertices are evenly spaced and the ends kept."""
    line = np.array([[0.0, 0.0], [100.0, 0.0], [155.0, 0.0]])
    out = valley_snap.resample(line, 10.0)
    steps = np.hypot(*np.diff(out, axis=0).T)
    assert np.allclose(out[0], line[0]) and np.allclose(out[-1], line[-1])
    assert steps.max() - steps.min() < 1e-6
    assert steps.max() <= 10.0 + 1e-9


# ----------------------------------------------------------- downstream order


def test_isotonic_is_monotonic_and_close() -> None:
    """The water level never rises downstream and stays a least-squares fit."""
    rng = np.random.default_rng(1)
    floor = np.linspace(100.0, 20.0, 500) + rng.normal(0, 3.0, 500)
    floor[200:210] += 15.0  # a bridge or a dam of the surface model
    level = valley_snap.isotonic_decreasing(floor)
    assert np.all(np.diff(level) <= 1e-9)
    assert np.abs(level - np.linspace(100.0, 20.0, 500)).mean() < 3.0
    assert level.mean() == pytest.approx(floor.mean())


def test_isotonic_fills_nan() -> None:
    """NaN samples are interpolated before the fit."""
    level = valley_snap.isotonic_decreasing(np.array([5.0, np.nan, 3.0, 4.0]))
    assert np.all(np.isfinite(level)) and np.all(np.diff(level) <= 1e-9)


def test_water_level_ignores_bathymetry() -> None:
    """ZG7c: a channel bed below sea level (Thames at London) keeps the water at 0 m."""
    from cent_ans_tools.geo import hydro_fine

    floor = np.array([3.0, 1.5, np.nan, -12.0, -30.0, -8.0, 0.4])
    level = hydro_fine.water_level(floor)
    assert np.isnan(level[2])
    assert level[0] == 3.0 and level[1] == 1.5 and level[-1] == 0.4
    assert np.all(level[3:6] == hydro_fine.MIN_WATER_LEVEL_M)
    fitted = valley_snap.isotonic_decreasing(level)
    assert fitted.min() >= hydro_fine.MIN_WATER_LEVEL_M
    assert np.all(np.diff(fitted) <= 1e-9)


def test_enforce_cap() -> None:
    """A tributary never ends above the level of the river it joins."""
    out = valley_snap.enforce_cap(np.array([10.0, 12.0, 8.0]), 9.0)
    assert out.tolist() == [9.0, 9.0, 8.0]


def test_drape_limits_cut_and_fill() -> None:
    """Road heights are smoothed but stay within the embankment tolerance."""
    heights = np.concatenate([np.zeros(50), np.full(5, 10.0), np.zeros(50)])
    road = valley_snap.drape(heights, 20.0, smooth_m=100.0, max_cut_m=3.0)
    assert np.all(np.abs(road - heights) <= 3.0 + 1e-9)
    assert np.abs(np.diff(road)).max() < np.abs(np.diff(heights)).max()


# ---------------------------------------------------------------- topology


def _toy_table() -> hydro_sources.LinkTable:
    """Two sources joining (order 2), a third brook joining below (stays 2)."""
    lines = [
        np.array([[0.0, 0.0], [10.0, 10.0]]),
        np.array([[20.0, 0.0], [10.0, 10.0]]),
        np.array([[10.0, 10.0], [10.0, 20.0]]),
        np.array([[0.0, 20.0], [10.0, 20.0]]),
        np.array([[10.0, 20.0], [10.0, 30.0]]),
    ]
    n = len(lines)
    return hydro_sources.LinkTable(
        source="test",
        lines=lines,
        start=np.array([0, 1, 2, 3, 4]),
        end=np.array([2, 2, 4, 4, 5]),
        name=np.array(["a", "b", "main", "c", "main"], dtype=object),
        code=np.array(["a", "b", "main", "c", "main"], dtype=object),
        width_min=np.full(n, np.nan),
        width_max=np.full(n, np.nan),
        strahler=np.zeros(n, dtype=np.int16),
        canal=np.array([False, False, False, True, False]),
        intermittent=np.zeros(n, dtype=bool),
        tidal=np.zeros(n, dtype=bool),
    )


def test_strahler_orders() -> None:
    """Classic Strahler rule on a toy network."""
    table = _toy_table()
    assert hydro_sources.compute_strahler(table).tolist() == [1, 1, 2, 1, 2]


def test_braid_does_not_raise_order() -> None:
    """Two arms of the same river rejoining keep the order."""
    table = _toy_table()
    table.code = np.array(["a", "a", "a", "c", "a"], dtype=object)
    assert hydro_sources.compute_strahler(table)[2] == 1


def test_strokes_follow_main_stem() -> None:
    """Each link belongs to one stroke; the main stem runs to the mouth."""
    table = _toy_table()
    table.strahler = hydro_sources.compute_strahler(table)
    strokes = hydro_sources.build_strokes(table)
    owned = sorted(link for stroke in strokes for link in stroke.links)
    assert owned == [0, 1, 2, 3, 4]
    longest = max(strokes, key=lambda s: len(s.links))
    assert longest.links[-2:] == [2, 4]
    assert np.allclose(longest.points[-1], [10.0, 30.0])


def test_modern_canals_are_excluded() -> None:
    """Source canals and named post-1340 canals go, the Fossdyke stays."""
    notes = json.loads(
        (DATA / "map" / "historical_hydro_notes.json").read_text(encoding="utf-8")
    )
    canal_filter = hydro_sources.CanalFilter.from_notes(notes)
    table = _toy_table()
    table.name = np.array(
        [
            "Canal du Midi",
            "la Loire",
            "Fossdyke",
            "Canal de Briare",
            "New Bedford River",
        ],
        dtype=object,
    )
    table.canal = np.array([False, False, True, True, False])
    assert canal_filter.excluded(table).tolist() == [True, False, False, True, True]


# --------------------------------------------------------------------- tiles


def test_split_by_tiles_shares_border_vertices() -> None:
    """A line crossing a tile border is cut there and both pieces share the cut."""
    xy = np.array([[10.0, 10.0], [70.0, 10.0], [70.0, 130.0]])
    attrs = np.array([[0.0], [60.0], [180.0]])
    pieces = fine_tiles.split_by_tiles(xy, attrs, 64.0)
    assert [(c, r) for c, r, _, _ in pieces] == [(0, 0), (1, 0), (1, 1), (1, 2)]
    for (_, _, a, _), (_, _, b, _) in zip(pieces, pieces[1:], strict=False):
        assert np.allclose(a[-1], b[0])
    assert pieces[0][2][-1].tolist() == [64.0, 10.0]
    assert pieces[0][3][-1][0] == pytest.approx(54.0)


def test_tile_roundtrip() -> None:
    """Encode then decode a CAFV tile."""
    lines = fine_tiles.TileLines()
    lines.add(
        3,
        fine_tiles.FLAG_TIDAL,
        np.array([[1.0, 2.0], [3.0, 4.0]]),
        [5.0, 4.0],
        [100.0, 120.0],
    )
    lines.add(
        7,
        0,
        np.array([[9.0, 9.0], [9.5, 9.0], [10.0, 9.5]]),
        [1.0, 1.0, 0.5],
        [3.0, 3.0, 3.0],
    )
    data = fine_tiles.encode(fine_tiles.LAYER_RIVERS, 2, 40, 41, lines)
    decoded = fine_tiles.decode(data)
    assert (decoded["col"], decoded["row"], decoded["level"]) == (40, 41, 2)
    assert [line["feature"] for line in decoded["lines"]] == [3, 7]
    assert decoded["lines"][0]["flags"] == fine_tiles.FLAG_TIDAL
    assert np.allclose(decoded["lines"][1]["xy"][-1], [10.0, 9.5])
    assert np.allclose(decoded["lines"][0]["w"], [100.0, 120.0])


# ------------------------------------------------------------------- schemas


def test_canal_patterns_compile_and_hit_their_name() -> None:
    """Every modern canal pattern compiles; each entry matches its own name or a pattern of it."""
    import re

    notes = json.loads(
        (DATA / "map" / "historical_hydro_notes.json").read_text(encoding="utf-8")
    )
    for entry in notes["modern_canals"] + notes["kept_artificial"]:
        compiled = [re.compile(p, re.IGNORECASE) for p in entry["patterns"]]
        assert compiled, entry["id"]


def test_width_anchor_names_resolve() -> None:
    """Width anchor names normalise to distinct keys (no two rivers share a name)."""
    from cent_ans_tools.geo import hydro_fine

    widths = json.loads(
        (DATA / "map" / "river_widths.json").read_text(encoding="utf-8")
    )
    seen: dict[str, str] = {}
    for river in widths["rivers"]:
        for name in river["names"]:
            key = hydro_fine.normalise_name(name)
            assert seen.setdefault(key, river["id"]) == river["id"], name


def test_fine_relief_prefers_finest_level(tmp_path: Path) -> None:
    """The sampler reads E5 where a tile exists, E0 elsewhere."""
    from cent_ans_tools.geo import fine_relief, terrain

    bounds = (0.0, 0.0, 8192.0 * 100.0, 8192.0 * 100.0)  # E0 pixel = 100 m
    (tmp_path / "height").mkdir()
    terrain.write_png16(
        terrain.height_to_uint16(np.full((512, 512), 10.0)),
        tmp_path / "height" / "h_0_0.png",
    )
    (tmp_path / "pyramid" / "E5").mkdir(parents=True)
    terrain.write_png16(
        terrain.height_to_uint16(np.full((512, 512), 42.0)),
        tmp_path / "pyramid" / "E5" / "0_0.png",
    )
    relief = fine_relief.FineRelief(tmp_path, bounds)
    top = bounds[3]
    # E5 tile (0, 0) covers 512 * 100 / 32 = 1600 m from the north-west corner.
    inside = relief.sample(np.array([800.0]), np.array([top - 800.0]))
    outside = relief.sample(np.array([20000.0]), np.array([top - 20000.0]))
    assert inside[0] == pytest.approx(42.0, abs=0.1)
    assert outside[0] == pytest.approx(10.0, abs=0.1)
    assert relief.finest_level(
        np.array([800.0, 20000.0]), np.array([top - 800.0, top - 20000.0])
    ).tolist() == [5, 0]


def test_side_arm_rejoining_does_not_raise_order() -> None:
    """A river splitting around an island (unnamed side arm) keeps its order."""
    lines = [
        np.array([[0.0, 0.0], [0.0, 10.0]]),  # 0: river
        np.array([[0.0, 10.0], [0.0, 20.0]]),  # 1: main arm
        np.array([[0.0, 10.0], [5.0, 15.0], [0.0, 20.0]]),  # 2: side arm
        np.array([[0.0, 20.0], [0.0, 30.0]]),  # 3: river below
    ]
    n = len(lines)
    table = hydro_sources.LinkTable(
        source="test",
        lines=lines,
        start=np.array([0, 1, 1, 2]),
        end=np.array([1, 2, 2, 3]),
        name=np.array(["r", "r", "", "r"], dtype=object),
        code=np.array(["r", "r", "", "r"], dtype=object),
        width_min=np.full(n, np.nan),
        width_max=np.full(n, np.nan),
        strahler=np.zeros(n, dtype=np.int16),
        canal=np.zeros(n, dtype=bool),
        intermittent=np.zeros(n, dtype=bool),
        tidal=np.zeros(n, dtype=bool),
    )
    assert hydro_sources.compute_strahler(table).tolist() == [1, 1, 1, 1]


def test_two_way_reach_is_oriented_downstream() -> None:
    """A chain of two-way links digitised upstream is reversed after a one-way link."""
    # one-way 0 -> 1, then two-way links digitised 3 -> 2 and 2 -> 1 (backwards).
    start = np.array([0, 2, 3])
    end = np.array([1, 1, 2])
    flip = hydro_sources.orient_two_way(start, end, np.array([False, True, True]))
    assert flip.tolist() == [False, True, True]
