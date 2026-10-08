"""SG2: animated siege engines settings, models and battle demos."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
MODELS = ROOT / "game" / "assets" / "models" / "siege"

# Triangle budget per engine model (a regiment shows a few; UR1/UR2 figures stay under 2.4 k).
TRIANGLE_BUDGET = 2400


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def _check(schema_name: str, document: dict) -> None:
    schema = _load(f"schemas/{schema_name}")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_siege_engines_match_schema() -> None:
    """The engine animation settings exist and match their schema."""
    _check("siege_engines.schema.json", _load("fx/siege_engines.json"))


def test_engines_are_siege_units_with_a_model() -> None:
    """Every animated engine is a siege unit type and has its model and manifest entry."""
    settings = _load("fx/siege_engines.json")
    manifest = json.loads((MODELS / "manifest.json").read_text(encoding="utf-8"))
    for unit_type, model in settings["engines"].items():
        unit = _load(f"unit_types/{unit_type}.json")
        assert unit["category"] == "siege", unit_type
        assert (MODELS / f"{model}.glb").is_file(), model
        assert model in manifest
    for model in ("ram", "siege_tower"):
        assert (MODELS / f"{model}.glb").is_file(), model


def test_engine_models_stay_within_budget() -> None:
    """Each engine model stays under the figure triangle budget."""
    manifest = json.loads((MODELS / "manifest.json").read_text(encoding="utf-8"))
    for name, info in manifest.items():
        assert 0 < info["triangles"] <= TRIANGLE_BUDGET, (name, info["triangles"])


def test_trebuchet_timeline_is_ordered() -> None:
    """The sling lets go during the swing, after the arm has passed the vertical."""
    trebuchet = _load("fx/siege_engines.json")["trebuchet"]
    assert trebuchet["cocked_deg"] < trebuchet["rest_deg"] < trebuchet["overswing_deg"]
    assert 0.3 < trebuchet["release_phase"] < 0.9
    assert trebuchet["sling_release_deg"] < trebuchet["sling_end_deg"]
    assert (
        trebuchet["swing_s"] + trebuchet["settle_s"] + trebuchet["ready_margin_s"]
        < 12.0
    )


def test_battle_demos_match_schema() -> None:
    """The demo battles of the main menu match their schema and name real places."""
    demos = _load("ui/battle_demos.json")
    _check("battle_demos.schema.json", demos)
    landmarks = {p.stem for p in (DATA / "landmarks").glob("*.json")}
    engines = set(_load("fx/siege_engines.json")["engines"])
    ids = [demo["id"] for demo in demos["demos"]]
    assert len(ids) == len(set(ids))
    for demo in demos["demos"]:
        if "landmark" in demo:
            assert demo["landmark"] in landmarks, demo["id"]
        if "province" in demo:
            assert (DATA / "provinces" / f"{demo['province']}.json").is_file() or any(
                DATA.glob(f"provinces/**/{demo['province']}.json")
            ), demo["id"]
        for engine in demo.get("engines", []):
            assert engine in engines, (demo["id"], engine)
    assert {"avignon", "bruges"} <= {demo.get("landmark") for demo in demos["demos"]}


def test_ga3_variants_have_their_models() -> None:
    """GA3-L5: every generated variant of an animated engine exists at both levels of detail."""
    settings = _load("fx/siege_engines.json")
    for engine, variant in settings.get("ga3", {}).items():
        if engine == "description":
            continue
        assert engine in (*settings["engines"].values(), "ram", "siege_tower"), engine
        for suffix in ("", "_lod"):
            assert (MODELS / f"{variant['model']}{suffix}.glb").is_file(), variant


def test_trebuchet_swing_curve_matches_schema() -> None:
    """The measured swing curve (own file, own licence) matches its schema."""
    curve = _load("fx/trebuchet_swing_curve.json")
    _check("fx_trebuchet_swing_curve.schema.json", curve)
    assert "swing_curve" not in _load("fx/siege_engines.json")["trebuchet"]
    assert "CC BY-SA" in curve["licence"]
