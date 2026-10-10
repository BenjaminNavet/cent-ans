"""Settlement cap (lot CO-A, ADR 0291): rules on synthetic provinces, then the committed data."""

import json

from conftest import DATA

from cent_ans_tools import settlement_cap as cap

RULES = cap.load_rules()


def entry(
    ident: str,
    kind: str,
    weight: int,
    *,
    port: bool = False,
    fort: int = 1,
    buildings: list[str] | None = None,
) -> dict:
    """A minimal settlement entry."""
    return {
        "id": ident,
        "province": "prov_x",
        "kind": kind,
        "name": {"display": ident},
        "lonlat": [1.0, 2.0],
        "weight": weight,
        "fortification_level": fort,
        "buildings": buildings or [],
        "port": port,
    }


def province() -> list[dict]:
    """Eight places: a city, four towns, a castle, an abbey, a village."""
    return [
        entry("set_city", "city", 40),
        entry("set_t1", "town", 30),
        entry("set_t2", "town", 28),
        entry("set_t3", "town", 26),
        entry("set_t4", "town", 24),
        entry("set_castle", "castle", 12, fort=3),
        entry("set_abbey", "abbey", 9),
        entry("set_village", "village", 3, fort=0),
    ]


def test_cap_keeps_city_and_at_most_five() -> None:
    """Cap keeps city and at most five."""
    kept, over = cap.select_province(province(), RULES, set(), set())
    ids = {e["id"] for e in kept}
    assert len(kept) == RULES["max_per_province"]
    assert "set_city" in ids
    assert over == 0


def test_cap_keeps_a_village_and_a_famous_castle_or_abbey() -> None:
    """Cap keeps a village and a famous castle or abbey."""
    kept, _ = cap.select_province(province(), RULES, set(), set())
    kinds = {e["kind"] for e in kept}
    assert "village" in kinds
    assert kinds & {"castle", "abbey"}


def test_protected_ids_survive_even_above_the_cap() -> None:
    """Protected ids survive even above the cap."""
    entries = province()
    protected = {"set_t1", "set_t2", "set_t3", "set_t4", "set_castle", "set_abbey"}
    kept, over = cap.select_province(entries, RULES, protected, set())
    assert protected <= {e["id"] for e in kept}
    assert over == len(kept) - RULES["max_per_province"] > 0


def test_small_province_is_untouched() -> None:
    """Small province is untouched."""
    entries = province()[:3]
    kept, _ = cap.select_province(entries, RULES, set(), set())
    assert kept == entries


def test_plan_demotes_minor_towns_until_the_ratio_holds() -> None:
    """Plan demotes minor towns until the ratio holds."""
    entries = {"prov_x": province()}
    result = cap.plan(entries, RULES, set(), set())
    kept = {e["id"]: e for e in entries["prov_x"] if e["id"] in result.kept["prov_x"]}
    towns = sum(
        e["kind"] == "town" and e["id"] not in result.demoted for e in kept.values()
    )
    villages = sum(
        e["kind"] == "village" or e["id"] in result.demoted for e in kept.values()
    )
    assert villages >= RULES["village_ratio"] * towns or result.unreachable_ratio
    assert {e["id"] for e in result.removed}.isdisjoint(kept)


def test_ports_are_never_demoted() -> None:
    """Ports are never demoted."""
    entries = {
        "prov_x": [
            entry("set_city", "city", 40),
            entry("set_port", "town", 5, port=True),
        ]
    }
    result = cap.plan(entries, RULES, set(), set())
    assert "set_port" not in result.demoted


def test_demoted_entry_loses_forbidden_buildings_and_fortification() -> None:
    """Demoted entry loses forbidden buildings and fortification."""
    allowed = {"bld_a": {"village", "town"}, "bld_b": {"town"}, "bld_free": set()}
    town = entry("set_t", "town", 8, fort=3, buildings=["bld_a", "bld_b", "bld_free"])
    village = cap.demote_entry(town, RULES, allowed)
    assert village["kind"] == "village"
    assert (
        village["fortification_level"] == RULES["demote"]["village_max_fortification"]
    )
    assert village["buildings"] == ["bld_a", "bld_free"]
    assert town["kind"] == "town"  # the input is not mutated


def test_prune_removes_ids_from_strings_and_keys() -> None:
    """Prune removes ids from strings and keys."""
    document = {"ids": ["set_a", "set_b"], "by_id": {"set_a": 1, "set_c": 2}}
    assert cap._prune_ids(document, {"set_a"})
    assert document == {"ids": ["set_b"], "by_id": {"set_c": 2}}


def test_committed_data_respects_the_cap() -> None:
    """Every province holds at most five settlements and villages >= 2 x towns."""
    entries = cap.load_all()
    counts = [len(v) for v in entries.values()]
    assert max(counts) <= RULES["max_per_province"]
    kinds = [e["kind"] for v in entries.values() for e in v]
    assert kinds.count("village") >= RULES["village_ratio"] * kinds.count("town")
    assert kinds.count("city") == len(entries)


def test_former_settlements_are_not_settlements_any_more() -> None:
    """Former settlements are not settlements any more."""
    former = json.loads((DATA / "map" / cap.FORMER_FILE).read_text(encoding="utf-8"))
    kept = {e["id"] for v in cap.load_all().values() for e in v}
    ids = [s["id"] for s in former["settlements"]]
    assert not kept & set(ids)
    assert len(ids) == len(set(ids))
