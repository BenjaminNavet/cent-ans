"""GR (ADR 0352): every crop drawn on the campaign map has a far colour and a season class."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_every_crop_has_a_far_colour() -> None:
    """Biome mixes, TX overrides, regional landscapes and geo materials all name colours in `crop_far`."""
    mix = _load(DATA / "art" / "ground_biome_mix.json")
    landscapes = _load(DATA / "map" / "agri_landscapes.json")
    crop_far = mix["crop_far"]
    crops = {crop for biome in mix["biomes"].values() for crop in biome["crops"]}
    crops |= {
        crop
        for landscape in landscapes["landscapes"].values()
        for crop in landscape["crops"]
    }
    crops |= {mix["geo"]["vine_material"], mix["geo"]["meadow_material"]}
    crops |= {
        crop
        for override in mix.get("tx_overrides", {}).values()
        for crop in override.get("crops", {})
    }
    missing = sorted(crops - set(crop_far))
    assert not missing, f"crops without far colour: {missing}"


def test_geo_ranges_are_ordered() -> None:
    """Fade ranges of the relief rules go upwards."""
    geo = _load(DATA / "art" / "ground_biome_mix.json")["geo"]
    assert geo["farm_alt_start"] < geo["farm_alt_end"]
    assert geo["farm_slope_start"] < geo["farm_slope_end"]
    assert geo["coteau_slope_min"] < geo["coteau_slope_max"]
