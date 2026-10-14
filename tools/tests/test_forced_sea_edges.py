"""Forced sea edges (JR2b): declared in data/map/forced_sea_edges.json, reapplied by the generator."""

import json

import pytest

from cent_ans_tools.geo import provinces as provinces_step
from cent_ans_tools.geo import settlements
from cent_ans_tools.geo.settlements import Edge, Settlement

MAP_DIR = settlements.MAP_DIR


def _port(sid: str, x: float, port: bool = True) -> Settlement:
    return Settlement(sid, "prov_x", "town", sid, (0.0, 0.0), port, px=(x, 0.0))


def _forced(a: str, b: str) -> list[dict]:
    return [{"a": a, "b": b, "note": "t"}]


def test_apply_adds_sea_edge_and_is_idempotent() -> None:
    """The forced pair appears once, with cost 100 + km, sorted; a second pass changes nothing."""
    nodes = [_port("set_a", 0.0), _port("set_b", 100.0), _port("set_c", 5.0)]
    land = [Edge("set_a", "set_c", 3.0, 1.0)]
    once = settlements.apply_forced_sea_edges(
        land, nodes, _forced("set_b", "set_a"), 0.5
    )
    twice = settlements.apply_forced_sea_edges(
        once, nodes, _forced("set_b", "set_a"), 0.5
    )
    assert [(e.a, e.b, e.sea) for e in once] == [
        ("set_a", "set_b", True),
        ("set_a", "set_c", False),
    ]
    assert once[0].cost == pytest.approx(150.0)
    assert [e.to_json() for e in twice] == [e.to_json() for e in once]


def test_apply_errors() -> None:
    """Unknown settlement, non-port and existing land edge are refused."""
    nodes = [_port("set_a", 0.0), _port("set_b", 9.0), _port("set_d", 4.0, port=False)]
    land = [Edge("set_a", "set_b", 9.0, 1.0)]
    with pytest.raises(ValueError, match="inconnue"):
        settlements.apply_forced_sea_edges([], nodes, _forced("set_a", "set_zz"), 1.0)
    with pytest.raises(ValueError, match="pas un port"):
        settlements.apply_forced_sea_edges([], nodes, _forced("set_a", "set_d"), 1.0)
    with pytest.raises(ValueError, match="terrestre"):
        settlements.apply_forced_sea_edges(land, nodes, _forced("set_a", "set_b"), 1.0)


def test_committed_graph_has_every_forced_edge() -> None:
    """Each forced edge is a sea edge of settlement_graph.json at the formula cost."""
    forced = json.loads(
        (MAP_DIR / "forced_sea_edges.json").read_text(encoding="utf-8")
    )["edges"]
    graph = json.loads((MAP_DIR / "settlement_graph.json").read_text(encoding="utf-8"))[
        "edges"
    ]
    positions = json.loads(
        (MAP_DIR / "settlements_px.json").read_text(encoding="utf-8")
    )
    km_per_px = provinces_step.load_grid(MAP_DIR).meters_per_px / 1000.0
    by_pair = {(e["from"], e["to"]): e for e in graph}
    assert forced
    for entry in forced:
        a, b = sorted((entry["a"], entry["b"]))
        edge = by_pair[(a, b)]
        assert edge["sea"] and not edge["road"]
        (x0, y0), (x1, y1) = positions[a], positions[b]
        expected = (
            settlements.SEA_FIXED_COST
            + ((x0 - x1) ** 2 + (y0 - y1) ** 2) ** 0.5 * km_per_px
        )
        assert edge["cost"] == pytest.approx(expected, abs=0.1)
