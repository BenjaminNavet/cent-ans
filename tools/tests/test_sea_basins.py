"""Validates data/map/sea_basins.json (lot TB5: seas of the campaign map by basin)."""

import json
from pathlib import Path

import pytest
from pyproj import Transformer

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"

#: Reference points at sea (lon, lat) and the basin expected there ("" = default).
PLACES = {
    "Pas de Calais": ((1.5, 51.0), "north_sea_channel"),
    "Manche (large de Cherbourg)": ((-1.0, 50.2), "north_sea_channel"),
    "Mer du Nord (Dogger Bank)": ((3.0, 56.0), "north_sea_channel"),
    "Golfe de Gascogne": ((-4.0, 45.5), "atlantic"),
    "Mer Celtique": ((-7.5, 50.0), "atlantic"),
    "Golfe du Lion": ((4.5, 42.5), "mediterranean"),
    "Mer Égée": ((25.0, 38.0), "mediterranean"),
    "Baltique (Gotland)": ((19.0, 57.0), "baltic"),
    "Mer Noire": ((34.0, 43.0), ""),
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
    """Even-odd point-in-polygon test (the rule of SeaBasins.basin_at)."""
    x, y = point
    inside = False
    for index, (xa, ya) in enumerate(polygon):
        xb, yb = polygon[index - 1]
        if (ya > y) != (yb > y) and x < xa + (y - ya) * (xb - xa) / (yb - ya):
            inside = not inside
    return inside


def _basin(seas: dict, point: tuple[float, float]) -> str:
    """Id of the last basin containing the point, "" for the default sea."""
    found = ""
    for basin in seas["basins"]:
        if _inside(point, basin["polygon_px"]):
            found = basin["id"]
    return found


def _looks() -> dict:
    return {
        basin["id"]: basin["look"] for basin in _load("map/sea_basins.json")["basins"]
    }


def test_basin_ids_are_unique() -> None:
    """Basin ids are unique (one texture channel each)."""
    ids = [basin["id"] for basin in _load("map/sea_basins.json")["basins"]]
    assert len(ids) == len(set(ids))


@pytest.mark.parametrize("place", sorted(PLACES))
def test_reference_seas_are_in_the_expected_basin(place: str) -> None:
    """The Channel and the North Sea, the Atlantic, the Mediterranean are told apart."""
    (lon, lat), expected = PLACES[place]
    assert _basin(_load("map/sea_basins.json"), _to_px(lon, lat)) == expected


def test_northern_seas_are_dark_and_the_mediterranean_is_clear() -> None:
    """North Sea and Channel dark, Mediterranean clear and blue."""
    looks = _looks()
    north, atlantic, med = (
        looks["north_sea_channel"],
        looks["atlantic"],
        looks["mediterranean"],
    )
    assert sum(north["tint"]) < sum(atlantic["tint"]) < sum(med["tint"])
    assert north["clarity"] < atlantic["clarity"] < med["clarity"]
    assert med["tint"][2] > med["tint"][0]
    assert med["grey_scale"] < atlantic["grey_scale"]


def test_the_atlantic_has_the_heaviest_swell() -> None:
    """The Atlantic is the rough sea: strongest chop, long swell, whitecaps and surf."""
    looks = _looks()
    atlantic = looks["atlantic"]
    for name, look in looks.items():
        if name == "atlantic":
            continue
        for key in ("swell", "long_swell", "whitecaps", "foam"):
            assert atlantic[key] > look[key], (name, key)
