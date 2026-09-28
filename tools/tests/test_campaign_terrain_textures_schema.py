"""Validates the campaign terrain texture data against its schema (lot GA4)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"
SHADER_ORDER = ["grass", "farmland", "forest", "rock", "heath", "snow", "sand"]


def _document() -> dict:
    return json.loads(
        (DATA / "fx" / "campaign_terrain_textures.json").read_text(encoding="utf-8")
    )


def test_campaign_terrain_textures_match_schema() -> None:
    """``campaign_terrain_textures.json`` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "fx_campaign_terrain_textures.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_document()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_campaign_terrain_layers_follow_shader_order() -> None:
    """Layer order is the fixed index contract of ``terrain.gdshader``."""
    assert [layer["id"] for layer in _document()["layers"]] == SHADER_ORDER


def test_campaign_terrain_layer_means_filled() -> None:
    """Layer means were written by the texture build (never left at zero)."""
    for layer in _document()["layers"]:
        assert sum(layer["mean_linear"]) > 0.0, layer["id"]
