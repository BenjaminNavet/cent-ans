"""Validates data/map/building_models.json (lot TB3: buildings outside the walls)."""

import json
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


def test_exaggeration_defaults_to_real_scale() -> None:
    """Models are at real scale by default, like the towns (ADR 0138)."""
    render = _document()["render"]
    assert render["exaggeration"]["max"] >= 1.0
    assert (
        render["exaggeration"]["full_size_below"] < render["exaggeration"]["max_above"]
    )
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


def test_models_grow_with_their_level() -> None:
    """Level 1 is one building, level 3 a small estate: footprint and triangles grow."""
    manifest = _manifest()
    for family in FAMILIES:
        radii = [manifest[f"{family}_{n}"]["radius"] for n in (1, 2, 3)]
        triangles = [manifest[f"{family}_{n}"]["triangles"] for n in (1, 2, 3)]
        assert radii == sorted(radii) and radii[2] > 1.5 * radii[0], family
        assert triangles == sorted(triangles), family
