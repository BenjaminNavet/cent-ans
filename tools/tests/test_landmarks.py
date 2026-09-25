"""Validates data/landmarks/*.json (lot L1, landmark cities such as Paris)."""

import json
import math
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from cent_ans_tools.geo.project import default_grid

DATA = Path(__file__).resolve().parents[2] / "data"
LANDMARKS = sorted((DATA / "landmarks").glob("*.json"))


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _warp(scale: dict, meters_per_px: float, radius_m: float) -> float:
    """Radial magnifier of the plan, same formula as the Blender generator."""
    a = 1.0 / scale["center_meters_per_unit"]
    core_m = scale["core_radius_m"]
    core_px = scale["core_radius_px"]
    b = (core_px - a * core_m) / core_m**2
    if radius_m <= core_m:
        return a * radius_m + b * radius_m**2
    zone_px = scale["zone_radius_px"]
    zone_m = zone_px * meters_per_px
    if radius_m <= zone_m:
        return core_px + (radius_m - core_m) / (zone_m - core_m) * (zone_px - core_px)
    return radius_m / meters_per_px


def test_landmarks_exist() -> None:
    """Paris (L1) and the cities of lot L2 are described."""
    for city in ("paris", "london", "avignon", "calais", "rouen", "bordeaux", "bruges"):
        assert (DATA / "landmarks" / f"{city}.json").exists(), city


MODELS = DATA.parent / "game" / "assets" / "models" / "landmarks"
BLENDER_SCRIPTS = DATA.parent / "tools" / "blender_scripts"


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_model_weight(path: Path) -> None:
    """Each landmark has its generated model; L2 cities stay under 6 MB (Paris, L1: 10 MB)."""
    glb = MODELS / f"{path.stem}.glb"
    assert glb.exists(), f"run landmark_city.py for {path.stem}"
    if path.stem != "paris":
        assert glb.stat().st_size <= 6_000_000, glb.stat().st_size


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_monuments_build(path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Every monument (and each dated variant) builds a valid mesh from its template and params."""
    monkeypatch.syspath_prepend(str(BLENDER_SCRIPTS))
    import landmark_monuments

    for monument in _load(path)["monuments"]:
        builder = landmark_monuments.BUILDERS[monument["model"]]
        for variant in ["", *monument.get("variant_from_year", {})]:
            parts = builder(
                monument.get("size", 1.0), variant, monument.get("params", {})
            )
            assert parts, (monument["id"], variant)
            for _material, (verts, faces) in parts:
                assert all(0 <= i < len(verts) for face in faces for i in face), (
                    monument["id"],
                    variant,
                )


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_matches_schema(path: Path) -> None:
    """Every landmark file matches landmark.schema.json and its id equals its file name."""
    schema = _load(DATA / "schemas" / "landmark.schema.json")
    Draft202012Validator.check_schema(schema)
    landmark = _load(path)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(landmark), key=lambda e: list(e.path)
    )
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors]
    assert landmark["id"] == path.stem


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_anchor_and_settlement(path: Path) -> None:
    """The anchor pixel and north bearing agree with the map projection; the settlement exists."""
    landmark = _load(path)
    grid = default_grid()
    lon, lat = landmark["anchor"]["lonlat"]
    x, y = grid.lonlat_to_pixel(lon, lat)
    px = landmark["anchor"]["px"]
    assert math.hypot(x - px[0], y - px[1]) < 0.01
    x2, y2 = grid.lonlat_to_pixel(lon, lat + 0.01)
    bearing = math.degrees(math.atan2(x2 - x, -(y2 - y)))
    assert abs(bearing - landmark["anchor"]["north_bearing_deg"]) < 0.05
    positions = _load(DATA / "map" / "settlements_px.json")
    assert landmark["settlement"] in positions
    settlement = positions[landmark["settlement"]]
    # The settlement point lies inside the magnified core.
    assert math.hypot(settlement[0] - px[0], settlement[1] - px[1]) < 2.0


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_warp_is_monotonic(path: Path) -> None:
    """The radial magnifier grows strictly with the distance and joins the map scale at the zone edge."""
    scale = _load(path)["scale"]
    meters_per_px = _load(DATA / "map" / "map.json")["meters_per_px"]
    previous = -1.0
    for step in range(0, 400):
        radius = step * 20.0
        value = _warp(scale, meters_per_px, radius)
        assert value > previous
        previous = value
    zone_m = scale["zone_radius_px"] * meters_per_px
    assert abs(_warp(scale, meters_per_px, zone_m) - scale["zone_radius_px"]) < 1e-6
    assert scale["core_radius_px"] < scale["zone_radius_px"]


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_elements_have_unique_ids(path: Path) -> None:
    """Ids are unique within each list; year ranges are ordered."""
    landmark = _load(path)
    for key in (
        "monuments",
        "walls",
        "bridges",
        "streets",
        "islands",
        "areas",
        "open_spaces",
    ):
        items = landmark.get(key, [])
        ids = [item["id"] for item in items]
        assert len(ids) == len(set(ids)), key
        for item in items:
            if "from_year" in item and "until_year" in item:
                assert item["from_year"] <= item["until_year"], item["id"]


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_siege_backdrop_references(path: Path) -> None:
    """The siege backdrop lists existing elements and names an existing province."""
    landmark = _load(path)
    siege = landmark.get("siege")
    if siege is None:
        return
    for key in ("areas", "monuments", "bridges", "streets", "walls"):
        known = {item["id"] for item in landmark.get(key, [])}
        missing = set(siege.get(key, [])) - known
        assert not missing, (key, missing)
    assert (DATA / "provinces" / f"{landmark['province']}.json").exists()
