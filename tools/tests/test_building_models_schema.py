"""Validates data/map/building_models.json (lot TB3: buildings outside the walls)."""

import json
import math
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
MODELS = ROOT / "game" / "assets" / "models" / "outbuildings"
FAMILIES = (
    "farm",
    "mill",
    "vineyard",
    "mine",
    "saltworks",
    "abbey",
    "market",
    "port",
)


def _document() -> dict:
    return json.loads((DATA / "map" / "building_models.json").read_text("utf-8"))


def _levels() -> list[tuple[str, dict]]:
    return [
        (family, level)
        for family, entry in _document()["families"].items()
        for level in entry["levels"]
    ]


def test_building_models_match_schema() -> None:
    """The mapping matches ``building_models.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "building_models.schema.json").read_text("utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_document()))
    assert not errors, [error.message for error in errors]


def test_eight_families_of_three_levels() -> None:
    """Each of the eight families has levels 1, 2 and 3, in order, with distinct models."""
    families = _document()["families"]
    assert set(families) == set(FAMILIES)
    models = []
    for name, entry in families.items():
        assert [level["level"] for level in entry["levels"]] == [1, 2, 3], name
        for level in entry["levels"]:
            assert level["model"] == f"{name}_{level['level']}"
            models.append(level["model"])
    assert len(models) == len(set(models)) == 24


def test_rules_reference_existing_buildings_and_resources() -> None:
    """Every building and resource named by a rule exists in ``data/``."""
    buildings = {path.stem for path in (DATA / "buildings").glob("bld_*.json")}
    resources = {path.stem for path in (DATA / "resources").glob("res_*.json")}
    for family, level in _levels():
        for group in level["when"]["require"]:
            assert set(group["of"]) <= buildings, (family, level["level"])
            assert group["min"] <= len(group["of"]), (family, level["level"])
        assert set(level["when"].get("resources", [])) <= resources, family


def test_a_higher_level_is_never_easier_to_reach() -> None:
    """A level asks for at least as many buildings as the level below (levels only grow)."""
    for name, entry in _document()["families"].items():
        needed = [
            sum(group["min"] for group in level["when"]["require"])
            for level in entry["levels"]
        ]
        assert needed == sorted(needed), name
        assert needed[2] > needed[0] or name in ("market", "mill"), name


def test_screen_held_sizes() -> None:
    """Level 1 holds at least 28 px of a 900 px screen, level 3 at least 1.8 times level 1."""
    render = _document()["render"]
    screen = render["screen"]
    fractions = screen["fractions"]
    assert fractions[0] * 900 >= 28
    assert fractions == sorted(fractions)
    assert fractions[2] >= 1.8 * fractions[0]
    assert screen["real_below"] < screen["full_from"] <= 15
    assert render["fade_from_units"] < render["view_range_units"]
    assert render["fade_from_units"] >= 120 and render["view_range_units"] <= 160
    assert render["ring_min_m"] < render["ring_max_m"]


def _manifest() -> dict:
    return json.loads((MODELS / "manifest.json").read_text("utf-8"))


def test_every_model_is_exported_under_its_triangle_cap() -> None:
    """Each mapped model has a GLB and stays within a modest triangle budget per level."""
    manifest = _manifest()
    caps = {1: 800, 2: 1800, 3: 3500}
    for family, level in _levels():
        name = level["model"]
        assert (MODELS / f"{name}.glb").is_file(), name
        entry = manifest[name]
        assert entry["family"] == family and entry["level"] == level["level"]
        assert entry["triangles"] <= caps[entry["level"]], name
    assert manifest["worksite_1"]["triangles"] <= caps[1]


def test_settlement_signs_exist_and_outsize_the_outbuildings() -> None:
    """Each settlement kind with a sign names an exported model, larger than a level 3 model."""
    manifest = _manifest()
    screen = _document()["render"]["screen"]
    assert set(screen["signs"]) <= {"city", "town", "village", "castle", "abbey"}
    for kind, rule in screen["signs"].items():
        for key in ("model", "walled_model"):
            if key in rule:
                assert (MODELS / f"{rule[key]}.glb").is_file(), (kind, rule[key])
                assert manifest[rule[key]]["triangles"] <= 6000, rule[key]
    assert screen["signs"]["city"]["fraction"] > 1.3 * screen["fractions"][2]
    assert screen["signs"]["town"]["fraction"] > screen["fractions"][2]


def test_every_model_lists_its_rigid_pieces() -> None:
    """Pieces (centre and plan box) let Godot stand each building on its own ground."""
    for name, entry in _manifest().items():
        assert entry["pieces"], name
        for cx, cz, x0, z0, x1, z1 in entry["pieces"]:
            assert x0 <= cx <= x1 and z0 <= cz <= z1, name


def test_models_grow_with_their_level() -> None:
    """Level 1 is the signature piece, level 3 a rich compound: footprint and detail grow."""
    manifest = _manifest()
    for family in FAMILIES:
        radii = [manifest[f"{family}_{n}"]["radius"] for n in (1, 2, 3)]
        triangles = [manifest[f"{family}_{n}"]["triangles"] for n in (1, 2, 3)]
        assert radii == sorted(radii) and radii[2] > 1.5 * radii[0], family
        assert triangles[2] > triangles[0], family


def test_growth_uses_existing_kit_houses_and_fortifications() -> None:
    """Suburb houses come from the town kit; enclosure kinds name real fortification buildings."""
    growth = _document()["growth"]
    kit = json.loads((MODELS.parent / "town_kit" / "manifest.json").read_text("utf-8"))
    assert set(growth["suburbs"]["house_models"]) <= set(kit)
    buildings = {path.stem for path in (DATA / "buildings").glob("bld_*.json")}
    kinds = [entry["kind"] for entry in growth["enclosure"]["kinds"]]
    assert kinds == ["palisade", "stone"]
    walls = json.loads((DATA / "map" / "towns_1340.json").read_text("utf-8"))["walls"]
    for entry in growth["enclosure"]["kinds"]:
        assert set(entry["buildings"]) <= buildings
        assert entry["kind"] in walls


def test_soot_of_a_sack_is_stronger_than_a_storm_or_a_siege() -> None:
    """A sacked town is blacker than a stormed one, itself blacker than a besieged one."""
    soot = _document()["soot"]
    assert soot["sack"] > soot["storm"] > soot["siege"] > 0
    assert soot["devastation_start"] < soot["devastation_full"]
    capture = json.loads((DATA / "rules" / "capture.json").read_text("utf-8"))
    assert soot["sack_devastation_rise"] <= capture["sack"]["devastation"]


def test_maquette_style_keeps_outbuildings_smaller_than_the_town_maquette() -> None:
    """In the maquette town style an outbuilding has a constant world size below a town's."""
    document = _document()
    maquette = document["render"]["maquette"]
    screen = document["render"]["screen"]
    assert set(maquette["built_in_game_only"]) <= set(FAMILIES)
    assert set(maquette["kinds"]) <= {"city", "town", "village", "castle", "abbey"}
    assert maquette["fade_from_units"] < maquette["view_range_units"]
    view_span = 2.0 * math.tan(math.radians(55.0) / 2.0) * maquette["size_distance"]
    widths = [fraction * view_span for fraction in screen["fractions"]]
    town_sizes = json.loads((DATA / "art" / "town_maquettes.json").read_text("utf-8"))[
        "sizes"
    ]
    smallest_host = min(town_sizes[kind] for kind in maquette["kinds"])
    assert widths == sorted(widths)
    assert widths[0] >= 1.5 and widths[-1] <= 0.6 * smallest_host
