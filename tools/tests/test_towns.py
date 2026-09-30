"""Town footprints around 1340 (lot ZG6): rules, schema and footprint geometry."""

import json
import math
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator

from cent_ans_tools.geo import towns

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _check(schema_name: str, document: dict) -> None:
    schema = _load(DATA / "schemas" / schema_name)
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: list(e.path)
    )
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors[:10]]


def _settlement(**extra) -> dict:
    base = {
        "id": "set_test",
        "kind": "town",
        "weight": 18,
        "fortification_level": 2,
        "buildings": ["bld_market", "bld_stone_walls"],
    }
    base.update(extra)
    return base


def flat(x, y):  # noqa: ANN001, ANN201
    """Flat plain at 50 m."""
    return np.full(np.shape(x), 50.0)


def spur(x, y):  # noqa: ANN001, ANN201
    """Plateau at 100 m, 200 m wide to the east and west, dropping 40 m beyond."""
    x = np.asarray(x, dtype=float)
    return np.where(
        np.abs(x) < 200.0, 100.0, 100.0 - np.minimum((np.abs(x) - 200.0) * 0.5, 40.0)
    )


def test_rules_match_schema() -> None:
    """data/rules/town_footprint.json matches its schema and references real settlements."""
    rules = towns.load_rules()
    _check("town_footprint_rules.schema.json", rules)
    ids = {s["id"] for s in towns.load_settlements()}
    missing = [
        r["settlement"] for r in rules["references"] if r["settlement"] not in ids
    ]
    assert not missing
    source_ids = {s["id"] for s in rules["sources"]}
    assert all(r["source"] in source_ids for r in rules["references"])


def test_generated_file_matches_schema() -> None:
    """data/map/towns_1340.json (generated) matches its schema and skips the landmarks."""
    path = DATA / "map" / towns.OUT_FILE
    document = _load(path)
    _check("towns_1340.schema.json", document)
    landmarks = towns.landmark_settlements()
    assert not landmarks & set(document["towns"])
    assert len(document["towns"]) >= 500


def test_population_reference_and_model() -> None:
    """References win (poll tax scaled to 1340); the model grows with weight and buildings."""
    rules = towns.load_rules()
    ghent, basis = towns.population_estimate(_settlement(id="set_gand"), rules)
    assert ghent == 64000 and basis.startswith("reference:")
    york, _ = towns.population_estimate(_settlement(id="set_york"), rules)
    assert york == round(10872 * 1.6, -1)
    small, basis = towns.population_estimate(_settlement(weight=10), rules)
    big, _ = towns.population_estimate(_settlement(weight=30), rules)
    rich, _ = towns.population_estimate(
        _settlement(weight=30, buildings=["bld_cathedral", "bld_fair"]), rules
    )
    assert basis == "model" and small < big < rich


def test_density_grows_with_size() -> None:
    """100-250 inhabitants per hectare, growing with the population."""
    rules = towns.load_rules()
    values = [towns.density_for(p, rules) for p in (500, 2000, 10000, 60000)]
    assert values == sorted(values)
    assert 80 <= values[0] <= 110 and 180 <= values[-1] <= 250


def test_wall_kind() -> None:
    """Explicit walls, palisade, fortified cities, open bourgs."""
    rules = towns.load_rules()
    assert towns.wall_kind(_settlement(), rules) == "stone"
    assert towns.wall_kind(_settlement(buildings=["bld_palisade"]), rules) == "palisade"
    assert towns.wall_kind(_settlement(kind="city", buildings=[]), rules) == "stone"
    assert towns.wall_kind(_settlement(buildings=[]), rules) == "none"
    assert towns.wall_kind(_settlement(kind="village", buildings=[]), rules) == "none"


def test_flat_enclosure_matches_area() -> None:
    """On a plain the enclosure keeps the target area, with some irregularity."""
    rules = towns.load_rules()
    rng = np.random.default_rng(1)
    target = 40.0 * 10_000.0
    radii, site = towns.enclosure_radii(
        flat, None, target, None, True, rules["site"], rng
    )
    assert site == "plain"
    assert abs(towns.polygon_area(radii) - target) / target < 0.06
    assert radii.std() > 1.0


def test_spur_enclosure_follows_edges() -> None:
    """On a narrow plateau the walls stop at the break of slope (spur town)."""
    rules = towns.load_rules()
    rng = np.random.default_rng(2)
    radii, site = towns.enclosure_radii(
        spur, None, 30.0 * 10_000.0, None, True, rules["site"], rng
    )
    angles = towns.bearings(len(radii))
    east_west = np.abs(np.cos(angles)) > 0.95
    assert site == "spur" or radii[east_west].max() < 240.0
    assert radii[east_west].max() < 240.0


def test_river_clips_small_towns() -> None:
    """A small town stays on its bank: no bearing reaches across the river."""
    rules = towns.load_rules()
    rng = np.random.default_rng(3)
    river = {"point": [0.0, 150.0], "tangent": [1.0, 0.0], "width_m": 40.0}
    radii, site = towns.enclosure_radii(
        flat, None, 20.0 * 10_000.0, river, True, rules["site"], rng
    )
    angles = towns.bearings(len(radii))
    south = np.sin(angles) * radii
    assert south.max() < 150.0 - 20.0
    assert site == "river"


def test_road_bearings_merge_and_order() -> None:
    """Exits are found where roads cross the circle, close ones merged (main first)."""
    lines = [
        ("secondary", np.array([[0.0, 0.0], [1000.0, 10.0]])),
        ("main", np.array([[0.0, 0.0], [1000.0, -60.0]])),
        ("secondary", np.array([[0.0, 0.0], [0.0, -1000.0]])),
    ]
    exits = towns.road_bearings(lines, 500.0, 6)
    assert len(exits) == 2
    east = next(e for e in exits if abs(math.sin(e["bearing"])) < 0.3)
    assert east["type"] == "main"
    completed = towns.complete_gates(exits[:1], 3, 7)
    assert len(completed) == 3


def test_plan_town_is_deterministic() -> None:
    """Same settlement, same footprint; faubourgs and finage present."""
    rules = towns.load_rules()
    lines = [("main", np.array([[-2000.0, 0.0], [2000.0, 0.0]]))]
    a = towns.plan_town(_settlement(), rules, flat, None, lines)
    b = towns.plan_town(_settlement(), rules, flat, None, lines)
    assert a == b
    assert a["walls"] == "stone" and len(a["gates"]) >= 2
    assert a["faubourgs"] and a["finage_radius_m"] >= 900
    assert a["core_ha"] > 0 and a["households"] > 0


def test_river_reach_orders_points() -> None:
    """A scattered reach becomes an ordered polyline through the site."""
    xs = np.linspace(-600, 600, 50)
    rng = np.random.default_rng(4)
    pts = np.column_stack([xs, 100 + 0.1 * xs])[rng.permutation(50)]
    reach = towns.river_reach(pts, np.full(50, 30.0), 800.0)
    assert reach is not None
    line = np.array(reach["line"])
    assert np.all(np.diff(line @ np.array(reach["tangent"])) > 0)


def test_major_river_beats_nearby_brook() -> None:
    """RS-G: a brook near the centre gives way to a much wider river in reach (Tours, Loire)."""
    assert towns.pick_river_feature({}) is None
    # Brook 50 m away, 15 m wide; Loire 450 m away, 400 m wide: the Loire.
    assert towns.pick_river_feature({3: (50.0, 15.0), 7: (450.0, 400.0)}) == 7
    # Two comparable rivers: the nearest one.
    assert towns.pick_river_feature({3: (50.0, 60.0), 7: (450.0, 120.0)}) == 3


def test_tours_river_corridor() -> None:
    """RS-G: Tours and Orléans keep the Loire (not a brook) as their river corridor."""
    data = json.loads((DATA / "map" / "towns_1340.json").read_text(encoding="utf-8"))
    for sid, width in (("set_tours", 300.0), ("set_orleans", 300.0)):
        river = data["towns"][sid]["river"]
        assert river is not None and river["width_m"] >= width, (
            sid,
            river and river["width_m"],
        )


def test_ground_grid_layout_and_fallback() -> None:
    """65 values: centre ~ z_m, rings follow the bearings, gaps fall back on z_m."""
    rules = towns.load_rules()
    town = towns.plan_town(_settlement(), rules, spur)
    ground = town["ground_m"]
    assert len(ground) == 1 + 2 * len(town["radii"]) == 65
    assert abs(ground[0] - town["z_m"]) < 0.06
    assert all(isinstance(v, float) for v in ground)

    def holey(x, y):  # noqa: ANN001, ANN202
        return np.where(np.asarray(x) > 1.0, np.nan, 42.0)

    filled = towns.ground_grid(holey, np.full(32, 100.0), 7.0)
    assert len(filled) == 65 and filled[0] == 7.0
    assert filled[1] == 7.0  # bearing 0 = east, x > 1: missing -> z_m
    assert filled[1 + 16] == 42.0  # bearing 180 deg = west
    assert not any(math.isnan(v) for v in filled)
