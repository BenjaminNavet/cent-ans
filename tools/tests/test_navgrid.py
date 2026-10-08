"""Lot M1: navigation grid (``data/map/navgrid.png``) and ``crossings.json``.

Unit tests use synthetic rasters; the ``data/map`` checks recompute the grid
from the committed inputs and compare it with the committed file. After a
change of the map, the roads, the settlements or ``crossings.json``, rerun
``uv run --project tools cent-ans geo navgrid``.
"""

import json
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator
from PIL import Image
from scipy import ndimage
from shapely.geometry import LineString

from cent_ans_tools.geo import navgrid

DATA = Path(__file__).resolve().parents[2] / "data"
MAP_DIR = navgrid.MAP_DIR
RERUN = "relancer `uv run --project tools cent-ans geo navgrid`"
EIGHT = np.ones((3, 3), dtype=bool)


def _components(passable: np.ndarray) -> np.ndarray:
    return ndimage.label(passable, structure=EIGHT)[0]


# ----------------------------------------------------------------------------
# Unit tests
# ----------------------------------------------------------------------------


def test_four_connected_blocks_diagonal_moves() -> None:
    """A diagonal line lets 8-neighbour moves through until it is closed."""
    diagonal = np.eye(12, dtype=bool)
    assert _components(~diagonal).max() == 1
    closed = navgrid.four_connected(diagonal)
    assert _components(~closed).max() == 2
    assert closed.sum() == 12 + 11


def test_river_is_crossable_only_at_the_crossing() -> None:
    """A closed river splits the grid; opening a crossing joins both banks there only."""
    size = 40
    lines = [LineString([(10.3, -1), (70.7, 81)])]
    river = navgrid.four_connected(
        navgrid.rasterize_lines(lines, size, scale=2, all_touched=True)
    )
    assert _components(~river).max() == 2
    major = river.astype(np.int16)
    snapped = navgrid.snap_to_river(major, 1, (20, 22), radius=8)
    assert snapped is not None and river[snapped]
    opened = navgrid.open_crossing(river, snapped, radius=1)
    assert all(max(abs(r - snapped[0]), abs(c - snapped[1])) <= 1 for r, c in opened)
    after = river.copy()
    for cell in opened:
        after[cell] = False
    assert _components(~after).max() == 1
    # The other named river is not snapped to.
    assert navgrid.snap_to_river(major, 2, (20, 22), radius=8) is None


def test_road_crossings_keep_short_overlaps_only() -> None:
    """A road crossing a river opens it; a road running along it does not."""
    major = np.zeros((30, 30), dtype=np.int16)
    major[:, 15] = 1
    road = np.zeros((30, 30), dtype=bool)
    road[5, :] = True  # crossing: overlap of one cell
    road[12:28, 15] = True  # along the river: 16 cells
    opened = navgrid.road_river_crossings(road, major)
    assert opened[5, 15]
    assert not opened[12:28, 15].any()


def test_road_crossings_far_from_anchors_are_dropped() -> None:
    """Lot M5b: only crossings near a settlement or a known bridge stay open."""
    candidates = np.zeros((40, 40), dtype=bool)
    candidates[5, 10] = candidates[5, 11] = True  # near the anchor
    candidates[30, 30] = True  # far away
    anchors = np.zeros_like(candidates)
    anchors[8, 10] = True
    kept, dropped = navgrid.near_anchors(candidates, anchors, radius_cells=4.0)
    assert kept[5, 10] and kept[5, 11]
    assert not kept[30, 30]
    assert dropped == 1


def test_river_gaps_are_joined() -> None:
    """Two pieces of one river a few pixels apart are joined; far pieces are not."""
    upper = LineString([(0, 0), (0, 50)])
    lower = LineString([(3, 60), (3, 100)])
    far = LineString([(80, 0), (80, 40)])
    joined = navgrid.join_river_gaps([upper, lower, far], max_gap=20.0)
    assert len(joined) == 4
    assert joined[-1].length == pytest.approx(np.hypot(3, 10))


def test_pass_opens_a_steep_ridge() -> None:
    """The pass path crosses a steep ridge; water is never used."""
    size = 60
    cost = np.full((size, size), 10.0)
    steep = np.zeros((size, size), dtype=bool)
    steep[:, 28:33] = True
    cost[steep] = navgrid.IMPASSABLE
    blocked = np.zeros((size, size), dtype=bool)
    blocked[:25, 30] = True  # a lake in the northern part of the ridge
    assert _components(cost < navgrid.IMPASSABLE).max() == 2
    cells = navgrid.carve_pass(
        np.where(steep, 30.0, cost), steep, blocked, [(10, 5), (40, 30), (10, 55)]
    )
    assert cells
    assert not any(blocked[cell] for cell in cells)
    for cell in cells:
        if cost[cell] >= navgrid.IMPASSABLE:
            cost[cell] = 30.0
    labels = _components(cost < navgrid.IMPASSABLE)
    assert labels[10, 5] == labels[10, 55] != 0


def test_terrain_cost_classes() -> None:
    """Plain, hill, forest, marsh, mountain and too-steep cells get their costs."""
    rules = navgrid.load_rules()
    costs = rules["terrain_costs"]
    height = np.array([[50.0, 500.0, 50.0, 5.0, 1500.0, 50.0]])
    slope = np.array([[0.01, 0.01, 0.01, 0.001, 0.05, 1.0]])
    weights = np.zeros((1, 6, 4))
    weights[0, 2, 2] = 0.8
    marsh = np.array([[False, False, False, True, False, False]])
    result = navgrid.terrain_cost(height, slope, weights, marsh, rules)
    assert result.tolist() == [
        [
            costs["plains"],
            costs["hills"],
            costs["forest"],
            costs["marsh"],
            costs["mountains"],
            navgrid.IMPASSABLE,
        ]
    ]


def test_wetland_cells_are_marsh_whatever_their_height() -> None:
    """Lot R3: a curated wetland (``wetlands.png``) is a marsh even above 15 m."""
    rules = navgrid.load_rules()
    height = np.array([[40.0, 40.0]])
    slope = np.array([[0.02, 0.02]])
    no_marsh = np.zeros((1, 2), dtype=bool)
    wetland = np.array([[True, False]])
    result = navgrid.terrain_cost(
        height, slope, np.zeros((1, 2, 4)), no_marsh, rules, wetland
    )
    assert result.tolist() == [
        [rules["terrain_costs"]["marsh"], rules["terrain_costs"]["plains"]]
    ]


def test_splat_is_block_averaged_to_the_grid() -> None:
    """Lot R3: a cell is a forest when half of its splat pixels are wooded."""
    forest = np.zeros((4, 4), dtype=np.float32)
    forest[0, 0] = forest[0, 1] = 1.0  # half of the top-left cell
    forest[2, 2] = 1.0  # a quarter of the bottom-right cell
    cells = navgrid._block_mean(forest, 2)
    assert cells.tolist() == [[0.5, 0.0], [0.0, 0.25]]
    weights = np.zeros((1, 2, 4))
    weights[0, :, 2] = [cells[0, 0], cells[1, 1]]
    cost = navgrid.terrain_cost(
        np.full((1, 2), 50.0),
        np.full((1, 2), 0.01),
        weights,
        np.zeros((1, 2), dtype=bool),
        navgrid.load_rules(),
    )
    costs = navgrid.load_rules()["terrain_costs"]
    assert cost.tolist() == [[costs["forest"], costs["plains"]]]


def test_lowland_heath_is_not_a_mountain() -> None:
    """Lot R3: the rock/heath weight makes a mountain on hills only."""
    rules = navgrid.load_rules()
    costs = rules["terrain_costs"]
    weights = np.zeros((1, 2, 4))
    weights[0, :, 3] = 0.8
    result = navgrid.terrain_cost(
        np.array([[40.0, 500.0]]),
        np.full((1, 2), 0.01),
        weights,
        np.zeros((1, 2), dtype=bool),
        rules,
    )
    assert result.tolist() == [[costs["plains"], costs["mountains"]]]


def test_forest_and_marsh_are_never_walls() -> None:
    """Forests and marshes are slower than plains but always passable."""
    costs = navgrid.load_rules()["terrain_costs"]
    for name in ("forest", "marsh"):
        assert costs["plains"] < costs[name] < navgrid.IMPASSABLE


# ----------------------------------------------------------------------------
# Data files
# ----------------------------------------------------------------------------


def _validate(schema_name: str, document: dict) -> list[str]:
    schema = json.loads((DATA / "schemas" / schema_name).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    return [error.message for error in validator.iter_errors(document)]


def test_crossings_match_schema() -> None:
    """``crossings.json`` matches its schema; ids are unique; every entry is sourced."""
    document = json.loads(
        (MAP_DIR / navgrid.CROSSINGS_FILE).read_text(encoding="utf-8")
    )
    ids = [entry["id"] for entry in document["crossings"]]
    assert len(ids) == len(set(ids))
    kinds = {entry["type"] for entry in document["crossings"]}
    assert kinds == {"bridge", "ford", "pass"}
    for entry in document["crossings"]:
        assert entry["source"].strip()
        if entry["type"] != "pass" and entry["river"] not in navgrid.RIVER_FR:
            assert entry["river"] == "Somme", entry["id"]


def test_crossings_schema_requires_a_route_for_passes() -> None:
    """A pass without route and a bridge without river are rejected."""
    bad = {
        "crossings": [
            {
                "id": "pass_x",
                "name": "X",
                "type": "pass",
                "lonlat": [7.0, 45.0],
                "source": "Wikipédia",
            },
            {
                "id": "bridge_y",
                "name": "Y",
                "type": "bridge",
                "lonlat": [2.0, 48.0],
                "source": "Wikipédia",
            },
        ]
    }
    assert len(_validate("crossings.schema.json", bad)) == 2


def test_navgrid_file_and_map_json() -> None:
    """``navgrid.png`` is a light 8-bit image of W/2 x H/2 cells declared in ``map.json``."""
    path = MAP_DIR / navgrid.NAVGRID_FILE
    assert path.stat().st_size < 3_000_000, "navgrid.png trop lourd"
    metadata = json.loads((MAP_DIR / "map.json").read_text(encoding="utf-8"))
    width, height = metadata["size_px"]
    scale = navgrid.NAVGRID_SCALE
    with Image.open(path) as image:
        assert image.mode == "L"
        assert image.size == (width // scale, height // scale) == (3584, 3072)
    assert metadata["navgrid"] == {
        "size_px": [width // scale, height // scale],
        "file": navgrid.NAVGRID_FILE,
        "scale": scale,
    }


@pytest.fixture(scope="module")
def layers() -> navgrid.NavgridLayers:
    """The grid recomputed from the committed inputs (about 15 s)."""
    return navgrid.compute(MAP_DIR)


def test_committed_navgrid_is_up_to_date(layers: navgrid.NavgridLayers) -> None:
    """The committed grid equals the one computed from the committed inputs."""
    with Image.open(MAP_DIR / navgrid.NAVGRID_FILE) as image:
        committed = np.asarray(image)
    assert np.array_equal(committed, layers.cost), RERUN


def test_settlements_connected_within_their_land_mass(
    layers: navgrid.NavgridLayers,
) -> None:
    """Every settlement reaches every settlement of its land mass (8 neighbours)."""
    errors, _ = navgrid.check(layers)
    assert not errors, errors
    for cell in layers.settlement_cells.values():
        assert layers.cost[cell] == navgrid.load_rules()["terrain_costs"]["plains"]


def test_major_rivers_passable_only_at_passages(
    layers: navgrid.NavgridLayers,
) -> None:
    """A major river cell is passable only where a passage (or a settlement) opens it."""
    river = layers.major > 0
    passable = layers.cost < navgrid.IMPASSABLE
    settlement = np.zeros_like(river)
    for cell in layers.settlement_cells.values():
        settlement[cell] = True
    passes = np.zeros_like(river)
    for crossing in layers.crossings:
        if crossing.type == "pass":
            for cell in crossing.cells:
                passes[cell] = True
    leaks = river & passable & ~layers.reopened & ~settlement & ~passes
    assert not leaks.any(), np.argwhere(leaks)[:10].tolist()
    assert (river & ~passable).sum() > 5 * (river & passable).sum()


@pytest.mark.parametrize(
    "crossing_id",
    [
        "bridge_pont_des_tourelles_orleans",
        "bridge_pont_saint_benezet_avignon",
        "bridge_london_bridge",
        "bridge_pont_de_pierre_de_ratisbonne",
        "ford_bac_de_cologne",
    ],
)
def test_bridge_joins_both_banks(
    layers: navgrid.NavgridLayers, crossing_id: str
) -> None:
    """Around a bridge, the banks are split without passages and joined with them."""
    crossing = next(c for c in layers.crossings if c.id == crossing_id)
    assert crossing.applied, crossing_id
    row, col = crossing.cells[0]
    radius = 6
    window = (
        slice(max(0, row - radius), row + radius + 1),
        slice(max(0, col - radius), col + radius + 1),
    )
    passable = layers.cost[window] < navgrid.IMPASSABLE
    river = layers.major[window] > 0
    closed = passable & ~river
    banks = _components(closed)
    touching = {
        int(label)
        for label in np.unique(banks[ndimage.binary_dilation(river, EIGHT) & closed])
        if label
    }
    assert len(touching) >= 2, "le fleuve ne sépare pas les deux rives"
    joined = _components(passable)
    assert len({int(joined[banks == label][0]) for label in touching}) == 1


def test_passes_are_open(layers: navgrid.NavgridLayers) -> None:
    """Every pass is traced, passable, and joins both ends of its route."""
    labels = _components(layers.cost < navgrid.IMPASSABLE)
    passes = [c for c in layers.crossings if c.type == "pass"]
    assert len(passes) >= 10
    for crossing in passes:
        assert crossing.applied, crossing.id
        cells = crossing.cells
        assert all(layers.cost[cell] < navgrid.IMPASSABLE for cell in cells)
        assert len({int(labels[cell]) for cell in cells}) == 1, crossing.id
