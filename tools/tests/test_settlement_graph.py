"""Lot C3 geo outputs: settlement graph, positions, roads, hamlets, relief tiles.

Unit tests use synthetic inputs; the ``data/map`` checks read the committed
outputs. When settlement files change (lot C2), rerun
``uv run --project tools cent-ans geo roads`` (roads + graph) and
``cent-ans geo hamlets``.
"""

import json
from pathlib import Path

import numpy as np
import pytest
from PIL import Image
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from scipy.spatial import cKDTree

from cent_ans_tools.geo import edge_paths, hamlets, relief, settlements

MAP_DIR = settlements.MAP_DIR
RERUN = "relancer `uv run --project tools cent-ans geo roads` puis `geo hamlets`"


# ----------------------------------------------------------------------------
# Unit tests
# ----------------------------------------------------------------------------


def test_slugify_matches_rust() -> None:
    """Same cases as `slug_folds_accents_and_separators` in settlement.rs."""
    assert settlements.slugify("Édimbourg") == "edimbourg"
    assert settlements.slugify("Bar-le-Duc") == "bar_le_duc"
    assert settlements.slugify("Saint Andrews") == "saint_andrews"
    assert settlements.slugify("Lödöse") == "lodose"
    assert settlements.slugify("L'Isle-Jourdain") == "l_isle_jourdain"
    assert settlements.slugify("Málaga") == "malaga"
    assert settlements.slugify("---") == ""
    assert settlements.slugify("Łódź") == "od"  # Ł and ź are not in the fold table


def _connected(count: int, pairs: list[tuple[int, int]]) -> bool:
    matrix = coo_matrix(
        (np.ones(len(pairs)), ([i for i, _ in pairs], [j for _, j in pairs])),
        shape=(count, count),
    )
    return connected_components(matrix, directed=False)[0] == 1


def test_province_edges_drop_long_edges_but_stay_connected() -> None:
    """A far outlier keeps exactly its shortest link; the cluster is triangulated."""
    points = np.array([[0, 0], [10, 0], [0, 10], [10, 10], [5, 5], [200, 5]], float)
    pairs = settlements.province_edges(points)
    assert _connected(len(points), pairs)
    outlier = [pair for pair in pairs if 5 in pair]
    assert len(outlier) == 1


def test_province_edges_small_and_collinear() -> None:
    """One point: no edge; two: one edge; collinear points stay connected."""
    assert settlements.province_edges(np.array([[0.0, 0.0]])) == []
    assert settlements.province_edges(np.array([[0.0, 0.0], [1.0, 0.0]])) == [(0, 1)]
    line = np.array([[0, 0], [1, 0], [2, 0], [3, 0]], float)
    assert _connected(4, settlements.province_edges(line))


def test_closest_pairs_are_disjoint_and_close() -> None:
    """Up to three pairs, no settlement reused, within the slack of the best."""
    a = np.array([[0, 0], [0, 10], [0, 20], [0, 100]], float)
    b = np.array([[5, 0], [5, 10], [6, 20], [50, 100]], float)
    pairs = settlements.closest_pairs(a, b)
    assert pairs[0] == (0, 0)
    assert len(pairs) == 3
    assert len({i for i, _ in pairs}) == 3 and len({j for _, j in pairs}) == 3
    assert (3, 3) not in pairs


def test_edge_cost_rules() -> None:
    """Terrain multiplies, road halves, sea adds the fixed embarking cost."""
    assert settlements.Edge("set_a", "set_b", 10.0, 2.0).cost == 20.0
    assert settlements.Edge("set_a", "set_b", 10.0, 2.0, road=True).cost == 10.0
    sea = settlements.Edge("set_a", "set_b", 10.0, 1.0, sea=True)
    assert sea.cost == settlements.SEA_FIXED_COST + 10.0


def test_fallback_city_ids_follow_the_rust_loader(tmp_path: Path) -> None:
    """Provinces without a city get `set_<slug>`, suffixed with the province on collision."""
    provinces = {
        "prov_a": {
            "id": "prov_a",
            "capital_city": {"name": {"display": "Évreux"}, "lon": 1.0, "lat": 49.0},
            "coastal": True,
            "ports": ["x"],
        },
        "prov_b": {
            "id": "prov_b",
            "capital_city": {"name": {"display": "Agen"}, "lon": 0.6, "lat": 44.2},
        },
    }
    folder = tmp_path / "settlements"
    folder.mkdir()
    entry = {
        "id": "set_agen",
        "province": "prov_a",
        "kind": "town",
        "name": {"display": "Agen"},
        "lonlat": [0.6, 44.2],
    }
    (folder / "prov_a.json").write_text(json.dumps([entry]), encoding="utf-8")
    loaded = {s.id: s for s in settlements.load_settlements(provinces, folder)}
    assert loaded["set_evreux"].fallback and loaded["set_evreux"].port
    assert loaded["set_agen_b"].province == "prov_b"
    assert not loaded["set_agen_b"].port


# ----------------------------------------------------------------------------
# Committed outputs
# ----------------------------------------------------------------------------


def _require(path: Path) -> Path:
    if not path.exists():
        pytest.skip(f"{path.name} absent ({RERUN})")
    return path


@pytest.fixture(scope="module")
def placed() -> dict[str, settlements.Settlement]:
    """Current settlements (files + fallback cities) with their game positions."""
    items, *_ = settlements.prepare()
    return {s.id: s for s in items}


@pytest.fixture(scope="module")
def graph_edges() -> list[dict]:
    """Edges of the committed ``settlement_graph.json``."""
    path = _require(MAP_DIR / settlements.GRAPH_FILE)
    return json.loads(path.read_text(encoding="utf-8"))["edges"]


def test_graph_format_matches_the_rust_loader(graph_edges: list[dict]) -> None:
    """Exactly the C1 format: from < to, unique pairs, cost ≥ 0, road/sea booleans."""
    seen = set()
    for edge in graph_edges:
        assert set(edge) == {"from", "to", "cost", "road", "sea"}
        assert edge["from"] < edge["to"]
        assert (edge["from"], edge["to"]) not in seen
        seen.add((edge["from"], edge["to"]))
        assert isinstance(edge["cost"], float | int) and edge["cost"] >= 0
        assert isinstance(edge["road"], bool) and isinstance(edge["sea"], bool)
        assert not (edge["road"] and edge["sea"])


def test_graph_references_existing_settlements(
    graph_edges: list[dict], placed: dict
) -> None:
    """Every edge endpoint is a current settlement or fallback city."""
    unknown = {end for edge in graph_edges for end in (edge["from"], edge["to"])} - set(
        placed
    )
    assert not unknown, f"colonies inconnues {sorted(unknown)[:10]} : {RERUN}"


def test_graph_is_connected_islands_by_sea(
    graph_edges: list[dict], placed: dict
) -> None:
    """The whole graph is connected; land-only islands are reached by sea edges."""
    ids = sorted(placed)
    index = {sid: i for i, sid in enumerate(ids)}

    def components(edges: list[dict]) -> np.ndarray:
        matrix = coo_matrix(
            (
                np.ones(len(edges)),
                ([index[e["from"]] for e in edges], [index[e["to"]] for e in edges]),
            ),
            shape=(len(ids), len(ids)),
        )
        return connected_components(matrix, directed=False)[1]

    usable = [e for e in graph_edges if e["from"] in index and e["to"] in index]
    full = components(usable)
    assert len(set(full)) == 1, f"graphe non connexe : {RERUN}"
    land = components([e for e in usable if not e["sea"]])
    main = np.bincount(land).argmax()
    sea_ends = {index[e[k]] for e in usable if e["sea"] for k in ("from", "to")}
    for component in set(land) - {main}:
        members = set(np.nonzero(land == component)[0])
        assert members & sea_ends, [ids[i] for i in members]


def test_every_settlement_is_placed_inside_its_province(placed: dict) -> None:
    """Game positions (``settlements_px.json``) fall inside the settlement's province."""
    path = _require(MAP_DIR / settlements.POSITIONS_FILE)
    positions = json.loads(path.read_text(encoding="utf-8"))
    assert set(positions) == set(placed), RERUN
    labels = settlements.load_labels()
    index_of = {
        pid: props["index"]
        for pid, props in settlements.load_province_geometry().items()
    }
    for sid, (x, y) in positions.items():
        assert labels[int(y), int(x)] == index_of[placed[sid].province], sid


def test_roads_geojson() -> None:
    """Roads are pixel LineStrings with a known source; some edges follow them."""
    path = _require(MAP_DIR / settlements.ROADS_FILE)
    collection = json.loads(path.read_text(encoding="utf-8"))
    assert collection["type"] == "FeatureCollection" and collection["features"]
    for feature in collection["features"]:
        assert feature["geometry"]["type"] == "LineString"
        assert feature["properties"]["source"] in {"itiner-e", "computed"}
        coords = np.asarray(feature["geometry"]["coordinates"])
        assert coords.min() >= 0
        assert coords[:, 0].max() <= 7168 and coords[:, 1].max() <= 6144


def test_hamlets() -> None:
    """About 3,000 hamlets, 6 km apart, off the settlements, in known provinces."""
    path = _require(MAP_DIR / hamlets.HAMLETS_FILE)
    data = json.loads(path.read_text(encoding="utf-8"))
    assert 2700 <= len(data) <= 3300
    geometry = settlements.load_province_geometry()
    assert {h["province"] for h in data} <= set(geometry)
    assert all(set(h) == {"name", "px", "province"} and h["name"] for h in data)
    km_per_px = settlements.provinces_step.load_grid().meters_per_px / 1000.0
    points = np.array([h["px"] for h in data])
    close = cKDTree(points).query_pairs(hamlets.MIN_SPACING_KM / km_per_px - 0.2)
    assert not close
    positions = json.loads(
        _require(MAP_DIR / settlements.POSITIONS_FILE).read_text(encoding="utf-8")
    )
    distance, _ = cKDTree(np.array(list(positions.values()))).query(points)
    assert (distance * km_per_px).min() > hamlets.MIN_SETTLEMENT_DISTANCE_KM - 0.2, (
        RERUN
    )


def test_relief_tiles() -> None:
    """``map.json`` declares 28 x 24 tiles of 512² 16-bit PNGs, all present."""
    metadata = json.loads((MAP_DIR / "map.json").read_text(encoding="utf-8"))
    tiles = metadata.get("height_tiles")
    if tiles is None:
        pytest.skip("height_tiles absent (cent-ans geo relief)")
    assert tiles == relief.height_tiles_meta(relief.fine_grid(MAP_DIR))
    assert tiles["size_px"] == [14336, 12288]
    cols, rows = (side // tiles["tile_px"] for side in tiles["size_px"])
    directory = MAP_DIR / tiles["dir"]
    for row in range(rows):
        for col in range(cols):
            assert (directory / tiles["pattern"].format(col=col, row=row)).exists()
    with Image.open(directory / tiles["pattern"].format(col=5, row=7)) as image:
        assert image.size == (512, 512)
        assert image.mode in {"I;16", "I;16B", "I"}


def test_tiles_round_trip(tmp_path: Path) -> None:
    """Writing then reading tiles restores the array."""
    array = (np.arange(1024 * 1024) % 65536).astype(np.uint16).reshape(1024, 1024)
    relief.write_tiles(array, tmp_path)
    assert len(list(tmp_path.glob("h_*_*.png"))) == 4
    np.testing.assert_array_equal(relief.read_tiles(tmp_path, 2), array)


# ----------------------------------------------------------------------------
# Road polylines of the graph edges (lot C7b)
# ----------------------------------------------------------------------------


def test_trace_follows_roads_across_small_gaps() -> None:
    """An L-shaped road with a 2 px gap is followed; a far detour is rejected."""
    lines = [
        np.array([[0.0, 0.0], [40.0, 0.0]]),
        np.array([[42.0, 0.0], [42.0, 40.0]]),
    ]
    network = edge_paths.build_network(lines)
    route = edge_paths.trace(network, (0.0, 1.0), (42.0, 40.0))
    assert route is not None
    np.testing.assert_allclose(route[0], [0.0, 1.0])
    np.testing.assert_allclose(route[-1], [42.0, 40.0])
    # The corner of the road is on the route (not the straight line).
    assert np.min(np.hypot(route[:, 0] - 41.0, route[:, 1])) < 1.5
    # 40 px apart but the only road is a 440 px loop: rejected.
    loop = [np.array([[0.0, 0.0], [0.0, 200.0], [40.0, 200.0], [40.0, 0.0]])]
    assert (
        edge_paths.trace(edge_paths.build_network(loop), (0.0, 0.0), (40.0, 0.0))
        is None
    )
    # No road near an end.
    assert edge_paths.trace(network, (0.0, 1.0), (200.0, 200.0)) is None


def test_edge_paths_file(graph_edges: list[dict]) -> None:
    """Road edges only, ends on the settlements, bounded detour, most road edges traced."""
    path = _require(MAP_DIR / edge_paths.EDGE_PATHS_FILE)
    traced = json.loads(path.read_text(encoding="utf-8"))["edges"]
    positions = json.loads(
        _require(MAP_DIR / settlements.POSITIONS_FILE).read_text(encoding="utf-8")
    )
    roads = {(e["from"], e["to"]) for e in graph_edges if e["road"]}
    seen = set()
    for entry in traced:
        key = (entry["from"], entry["to"])
        assert key in roads, f"{key} n'est pas une arête routière : {RERUN}"
        assert key not in seen
        seen.add(key)
        points = np.asarray(entry["points"])
        start, end = np.asarray(positions[key[0]]), np.asarray(positions[key[1]])
        np.testing.assert_allclose(points[0], start, atol=0.051)
        np.testing.assert_allclose(points[-1], end, atol=0.051)
        length = np.hypot(*np.diff(points, axis=0).T).sum()
        straight = np.hypot(*(end - start))
        limit = edge_paths.MAX_DETOUR * straight + edge_paths.DETOUR_SLACK_PX + 0.5
        assert length <= limit, key
    assert len(traced) >= 0.75 * len(roads), f"{len(traced)} / {len(roads)} tracées"
