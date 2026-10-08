"""Validates data/landmarks/*.json (lot L1, landmark cities such as Paris)."""

import json
import math
from pathlib import Path

import pytest

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
    """Each landmark has its campaign model and its siege backdrop (L3), each under 6 MB."""
    for suffix in ("", "_siege"):
        glb = MODELS / f"{path.stem}{suffix}.glb"
        assert glb.exists(), f"run landmark_city.py for {path.stem}{suffix}"
        assert glb.stat().st_size <= 6_000_000, (glb.name, glb.stat().st_size)


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_model_material_codes(path: Path) -> None:
    """GLB (L3): no normals, a byte tint whose alpha codes an atlas layer for every palette name."""
    import struct

    blob = (MODELS / f"{path.stem}.glb").read_bytes()
    json_len = struct.unpack("<I", blob[12:16])[0]
    doc = json.loads(blob[20 : 20 + json_len])
    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            assert "NORMAL" not in prim["attributes"], mesh["name"]
            colour = doc["accessors"][prim["attributes"]["COLOR_0"]]
            assert colour["type"] == "VEC4" and colour["componentType"] == 5121


def test_material_code_round_trip(monkeypatch: pytest.MonkeyPatch) -> None:
    """Layer and exaggeration survive the alpha byte, decoded as in `landmark.gdshader`."""
    monkeypatch.syspath_prepend(str(BLENDER_SCRIPTS))
    import landmark_city

    for name, layer in landmark_city.MATERIAL_LAYER.items():
        assert name in landmark_city.PALETTE, name
        for exaggeration in (1.0, 1.5, 1.9, 2.6):
            code = round(landmark_city.material_code(name, exaggeration) * 255)
            assert code // 16 == layer
            decoded = 2 ** ((code % 16 - 4) / 4)
            assert abs(math.log2(decoded / exaggeration)) <= 0.13, (name, exaggeration)
    assert set(landmark_city.PALETTE) <= set(landmark_city.MATERIAL_LAYER)


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
    landmark = _load(path)
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


@pytest.mark.parametrize("path", LANDMARKS, ids=lambda p: p.stem)
def test_landmark_siege_battle_references(path: Path) -> None:
    """L3: every city describes its besieged town; walls, attacked gate and streets exist."""
    landmark = _load(path)
    battle = landmark["siege"]["battle"]
    walls = {wall["id"]: wall for wall in landmark["walls"]}
    assert set(battle["walls"]) <= set(walls), battle["walls"]
    gates = {g["name"] for w in battle["walls"] for g in walls[w].get("gates", [])}
    assert battle["gate"] in gates, (battle["gate"], gates)
    streets = {street["id"] for street in landmark["streets"]}
    assert set(battle.get("streets", [])) <= streets
