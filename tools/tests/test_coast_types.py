"""Validates data/map/coast_types.json (lot TB5: cliffs and beaches of the campaign map)."""

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator
from pyproj import Transformer

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"

#: Reference places (lon, lat) and the coast geology expected there: (rock, beach).
PLACES = {
    "Douvres": ((1.31, 51.13), ("chalk", "shingle")),
    "Beachy Head": ((0.25, 50.74), ("chalk", "shingle")),
    "Étretat": ((0.20, 49.71), ("chalk", "shingle")),
    "Dieppe": ((1.08, 49.93), ("chalk", "shingle")),
    "Cap Blanc-Nez": ((1.71, 50.93), ("chalk", "sand")),
    "Pointe du Raz": ((-4.74, 48.04), ("granite", "sand")),
    "Ploumanac'h": ((-3.48, 48.83), ("granite", "sand")),
    "Land's End": ((-5.71, 50.07), ("granite", "sand")),
    "Mimizan (Landes)": ((-1.30, 44.21), ("rock", "sand")),
    "Marseille": ((5.37, 43.29), ("rock", "sand")),
}


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def _to_px(lon: float, lat: float) -> tuple[float, float]:
    """Map pixel of a lon/lat point (grid of data/map/map.json)."""
    map_json = _load("map/map.json")
    west, _south, _east, north = map_json["bounds_projected"]
    easting, northing = Transformer.from_crs(
        "EPSG:4326", map_json["crs"], always_xy=True
    ).transform(lon, lat)
    scale = map_json["meters_per_px"]
    return (easting - west) / scale, (north - northing) / scale


def _inside(point: tuple[float, float], polygon: list[list[float]]) -> bool:
    """Even-odd point-in-polygon test (the rule of CoastLook.region_at)."""
    x, y = point
    inside = False
    for index, (xa, ya) in enumerate(polygon):
        xb, yb = polygon[index - 1]
        if (ya > y) != (yb > y) and x < xa + (y - ya) * (xb - xa) / (yb - ya):
            inside = not inside
    return inside


def _geology(coast: dict, point: tuple[float, float]) -> tuple[str, str]:
    """(rock, beach) of the last region containing the point, else the default."""
    found = coast["default"]
    for region in coast["regions"]:
        if _inside(point, region["polygon_px"]):
            found = region
    return found["rock"], found["beach"]


def test_coast_types_match_schema() -> None:
    """The coast settings match their schema."""
    schema = _load("schemas/coast_types.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("map/coast_types.json"))
    )
    assert not errors, [error.message for error in errors]


def test_region_ids_are_unique_and_inside_the_map() -> None:
    """Region ids are unique and every polygon touches the map."""
    coast = _load("map/coast_types.json")
    width, height = _load("map/map.json")["size_px"]
    ids = [region["id"] for region in coast["regions"]]
    assert len(ids) == len(set(ids))
    for region in coast["regions"]:
        assert any(
            0 <= x <= width and 0 <= y <= height for x, y in region["polygon_px"]
        ), region["id"]


@pytest.mark.parametrize("place", sorted(PLACES))
def test_reference_places_have_the_expected_geology(place: str) -> None:
    """Dover and the pays de Caux are chalk, Brittany and Cornwall granite, the Landes sand."""
    (lon, lat), expected = PLACES[place]
    assert _geology(_load("map/coast_types.json"), _to_px(lon, lat)) == expected


def test_swash_is_slow_and_narrow() -> None:
    """The swash stays sober: a slow wave, narrower than the beach, weaker under cliffs."""
    coast = _load("map/coast_types.json")
    swash = coast["swash"]
    assert swash["period_s"] >= 4
    assert swash["in_px"] < coast["band"]["beach_px"]
    assert swash["edge_px"] < swash["film_px"] < swash["out_px"] + swash["in_px"]
    assert swash["cliff"] < 1


def test_cliff_rule_and_colours_are_readable() -> None:
    """The slope rule is ordered; chalk is much lighter than granite, sand lighter than shingle."""
    coast = _load("map/coast_types.json")
    assert coast["cliff"]["min_m"] < coast["cliff"]["max_m"]
    types = coast["types"]
    assert min(types["chalk"]["color"]) > 2 * max(types["granite"]["color"])
    assert sum(types["sand"]["color"]) > sum(types["shingle"]["color"])
    for name, entry in types.items():
        assert sum(entry["shade"]) < sum(entry["color"]), name
