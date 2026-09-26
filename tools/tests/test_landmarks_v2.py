"""Validates data/landmarks_v2/*.json (ADR 0078, lots VH0/VH4) and the ``geo landmarks`` tool."""

import json
import math
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator

from cent_ans_tools.geo import landmarks_v2
from cent_ans_tools.geo.project import default_grid

DATA = Path(__file__).resolve().parents[2] / "data"
CITIES = sorted((DATA / "landmarks_v2").glob("*.json"))
SCHEMA = json.loads(
    (DATA / "schemas" / "landmark_v2.schema.json").read_text(encoding="utf-8")
)


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_rouen_exists() -> None:
    """Rouen proves the format (VH4)."""
    assert (DATA / "landmarks_v2" / "rouen.json").exists()


@pytest.mark.parametrize("path", CITIES, ids=lambda p: p.stem)
def test_schema(path: Path) -> None:
    """Each v2 city validates against its schema."""
    errors = sorted(Draft202012Validator(SCHEMA).iter_errors(_load(path)), key=str)
    assert not errors, [f"{list(e.path)}: {e.message[:160]}" for e in errors[:5]]


@pytest.mark.parametrize("path", CITIES, ids=lambda p: p.stem)
def test_links_and_origin(path: Path) -> None:
    """Linked to its settlement and L1 maquette; origin within 300 m of the maquette anchor."""
    city = _load(path)
    v1 = _load(DATA / "landmarks" / f"{city['landmark']}.json")
    assert v1["settlement"] == city["settlement"]
    grid = default_grid()
    px, py = grid.projected_to_pixel(*city["origin_3035"])
    offset = math.dist((float(px), float(py)), v1["anchor"]["px"]) * grid.meters_per_px
    assert offset < 300.0, offset


@pytest.mark.parametrize("path", CITIES, ids=lambda p: p.stem)
def test_geometry_within_extent(path: Path) -> None:
    """Walls, streets, monuments and districts stay within ``extent_m`` (streaming radius)."""
    city = _load(path)
    reach = city["extent_m"] * 1.05
    points = [m["at"] for m in city["monuments"]]
    for key in ("walls", "streets", "quays"):
        for item in city.get(key, []):
            points += item["points"]
    for d in city["districts"]:
        points += d["polygon"]
    far = [p for p in points if math.hypot(p[0], p[1]) > reach]
    assert not far, far[:3]


@pytest.mark.parametrize("path", CITIES, ids=lambda p: p.stem)
def test_unique_ids_and_dates(path: Path) -> None:
    """Unique ids; ``from_year`` before ``until_year``."""
    city = _load(path)
    for key in ("monuments", "walls", "streets", "districts", "open_spaces", "bridges"):
        ids = [item["id"] for item in city.get(key, [])]
        assert len(ids) == len(set(ids)), key
        for item in city.get(key, []):
            if "from_year" in item and "until_year" in item:
                assert item["from_year"] <= item["until_year"], item["id"]


@pytest.mark.parametrize("path", CITIES, ids=lambda p: p.stem)
def test_gates_on_walls(path: Path) -> None:
    """Every gate lies on its wall line (within 15 m)."""
    city = _load(path)
    for wall in city["walls"]:
        pts = wall["points"] + ([wall["points"][0]] if wall.get("closed") else [])
        for gate in wall.get("gates", []):
            d = min(
                _segment_distance(gate["at"], a, b)
                for a, b in zip(pts, pts[1:], strict=False)
            )
            assert d < 15.0, (gate["name"], d)


@pytest.mark.parametrize("path", CITIES, ids=lambda p: p.stem)
def test_osm_credit(path: Path) -> None:
    """Streets extracted from OSM require an extracted ODbL source; non-commercial sources never extracted."""
    city = _load(path)
    if any(s["origin"] == "osm" for s in city["streets"]):
        assert any(s["extracted"] and "ODbL" in s["license"] for s in city["sources"])
    for source in city["sources"]:
        if any(tag in source["license"] for tag in ("NC", "non commercial", "Gallica")):
            assert not source["extracted"], source["title"]


def test_rouen_1340_facts() -> None:
    """Rouen around 1340: Gros-Horloge belfry only from 1389, no tour de Beurre, Seine from the fine map."""
    city = _load(DATA / "landmarks_v2" / "rouen.json")
    monuments = {m["id"]: m for m in city["monuments"]}
    assert monuments["gros_horloge"]["from_year"] == 1389
    assert monuments["beffroi_communal"]["until_year"] < 1389
    towers = monuments["cathedrale"]["params"]["west_towers"]
    assert [t["side"] for t in towers] == [
        "north"
    ]  # tour Saint-Romain seule (Beurre : 1485)
    assert monuments["cathedrale"]["params"]["length_m"] == pytest.approx(137, abs=3)
    assert "vieux_palais" not in monuments  # Henri V, 1419-1420
    assert any(w["origin"] == "rivers_fine" for w in city["waters"])
    assert len([s for s in city["streets"] if s["origin"] == "osm"]) > 200
    assert 4000.0 < landmarks_v2.wall_length(city) < 7000.0  # ≈ 5 km


def test_local_roundtrip() -> None:
    """lon/lat → local offsets → lon/lat."""
    origin = [3676847.0, 2964336.0]
    local = landmarks_v2.lonlat_to_local([1.09493, 1.1], [49.4402, 49.45], origin)
    back = landmarks_v2.local_to_lonlat(local, origin)
    assert np.allclose(back, [[1.09493, 49.4402], [1.1, 49.45]], atol=1e-7)
    assert abs(local[0][0]) < 5 and abs(local[0][1]) < 5


def test_osm_streets_recipe() -> None:
    """Recipe: highway filter, exclusions, main streets, clipping to districts."""
    city = {
        "origin_3035": [3676847.0, 2964336.0],
        "districts": [
            {
                "zone": "intra",
                "polygon": [[-300, -300], [300, -300], [300, 300], [-300, 300]],
            }
        ],
        "osm_streets": {
            "bbox_lonlat": [1.08, 49.43, 1.11, 49.45],
            "highways": ["residential"],
            "exclude": ["Rue Moderne"],
            "main": ["Grand-Rue"],
        },
    }
    lon0, lat0 = landmarks_v2.local_to_lonlat(
        np.array([[-500.0, 0.0]]), city["origin_3035"]
    )[0]
    lon1, lat1 = landmarks_v2.local_to_lonlat(
        np.array([[500.0, 0.0]]), city["origin_3035"]
    )[0]

    def way(way_id: int, name: str, highway: str = "residential") -> dict:
        return {
            "type": "way",
            "id": way_id,
            "tags": {"highway": highway, "name": name},
            "geometry": [{"lon": lon0, "lat": lat0}, {"lon": lon1, "lat": lat1}],
        }

    osm = {
        "elements": [
            way(1, "Grand-Rue"),
            way(2, "Rue Moderne"),
            way(3, "Chemin", "footway"),
            way(4, "Rue Basse"),
        ]
    }
    streets = landmarks_v2.osm_streets(city, osm)
    assert [s["name"] for s in streets] == ["Grand-Rue", "Rue Basse"]
    assert streets[0]["rank"] == "main" and streets[1]["rank"] == "secondary"
    xs = [p[0] for p in streets[0]["points"]]
    assert min(xs) >= -330 and max(xs) <= 330  # clipped to the district (+25 m buffer)


def test_chain_lines() -> None:
    """Tile pieces of one river are joined end to end."""
    a = {
        "name": "Seine",
        "points": np.array([[0.0, 0.0], [100.0, 0.0]]),
        "width": np.array([200.0, 200.0]),
    }
    b = {
        "name": "Seine",
        "points": np.array([[105.0, 0.0], [200.0, 0.0]]),
        "width": np.array([200.0, 210.0]),
    }
    merged = landmarks_v2.chain_lines([b, a])
    assert len(merged) == 1 and len(merged[0]["points"]) == 3


def _segment_distance(p: list[float], a: list[float], b: list[float]) -> float:
    ax, ay, bx, by, px, py = *a, *b, *p
    dx, dy = bx - ax, by - ay
    t = max(
        0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / max(dx * dx + dy * dy, 1e-9))
    )
    return math.hypot(px - ax - t * dx, py - ay - t * dy)
