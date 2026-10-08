"""Validates the group formation presets against their schema (CB6)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"

ROLES = ("infantry", "foot_ranged", "cavalry", "siege", "general")


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "group_formations.json").read_text(encoding="utf-8")
    )


def test_the_six_presets_of_the_plan() -> None:
    """Plan CB6 after the historian's review: six presets, battle line first."""
    presets = _rules()["presets"]
    ids = [p["id"] for p in presets]
    assert ids == [
        "battle_line",
        "harrow",
        "three_battles",
        "knight_charge",
        "foot_battle",
        "march",
    ]
    assert len(set(ids)) == len(ids)
    by_id = {p["id"]: p for p in presets}
    assert by_id["march"]["stance"] == "march"
    assert by_id["knight_charge"]["stance"] == "attack"
    assert by_id["harrow"]["stance"] == "defense"
    assert by_id["harrow"]["name_fr"] == "La herse"


def test_battle_line_keeps_the_shooters_behind() -> None:
    """Non-regression decision: the battle line keeps the shooters behind the foot."""
    line = next(p for p in _rules()["presets"] if p["id"] == "battle_line")
    roles = line["roles"]
    assert roles["foot_ranged"]["depth_m"] < 0
    assert roles["cavalry"]["lateral"] == "wings"
    assert roles["siege"]["depth_m"] < roles["foot_ranged"]["depth_m"]


def test_rows_agree_with_depths() -> None:
    """`row` is consistent with `depth_m` (front ahead, behind and reserve at the back)."""
    for preset in _rules()["presets"]:
        if preset["layout"] != "blocks":
            continue
        for role in ROLES:
            place = preset["roles"][role]
            depth = place["depth_m"]
            if place["row"] == "front":
                assert depth > 0, (preset["id"], role)
            elif place["row"] in ("behind", "reserve"):
                assert depth < 0, (preset["id"], role)
            else:
                assert abs(depth) <= 20, (preset["id"], role)


def test_march_column_covers_every_role() -> None:
    """Every role (and both halves of the foot, both kinds of horse) has a place in the column."""
    march = next(p for p in _rules()["presets"] if p["id"] == "march")
    seen = {
        (s["role"], s.get("filter", "all"), s.get("part", "all"))
        for s in march["column"]
    }
    for role in ("foot_ranged", "general", "siege"):
        assert (role, "all", "all") in seen, role
    assert ("infantry", "all", "first_half") in seen
    assert ("infantry", "all", "second_half") in seen
    assert ("cavalry", "light", "all") in seen
    assert ("cavalry", "heavy", "all") in seen
