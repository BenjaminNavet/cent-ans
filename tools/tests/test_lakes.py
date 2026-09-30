"""Lake extraction (lot SS3): synthetic masks and lakes.json schema."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from shapely.geometry import Point, Polygon

from cent_ans_tools.geo import lakes

DATA = Path(__file__).resolve().parents[2] / "data"


def _world() -> tuple[np.ndarray, np.ndarray]:
    """60 × 60 land with a sea strip on the west edge and four inland waters.

    - sea: columns 0-4 (touches the border);
    - lake A: 10 × 12 rectangle at 100 m (level), shore 140 m;
    - pond B: 6 × 6 at -2 m (below sea level, polder);
    - lake C: 8 × 8 at 50 m (stands for a reservoir);
    - puddle D: 2 × 2 at 30 m (below the area threshold).
    """
    land = np.ones((60, 60), dtype=bool)
    heights = np.full((60, 60), 140.0)
    land[:, :5] = False
    heights[:, :5] = -20.0
    land[10:20, 20:32] = False
    heights[10:20, 20:32] = 100.0
    heights[10, 20:32] = 118.0  # blended shore row: the mode ignores it
    land[30:36, 30:36] = False
    heights[30:36, 30:36] = -2.0
    land[45:53, 10:18] = False
    heights[45:53, 10:18] = 50.0
    land[50:52, 40:42] = False
    heights[50:52, 40:42] = 30.0
    return land, heights


def test_inland_labels_drop_the_sea() -> None:
    """Water touching the border is not a lake; the others are labelled."""
    land, _ = _world()
    labels, sizes = lakes.inland_water_labels(land)
    assert labels[:, :5].max() == 0
    assert labels[15, 25] > 0 and labels[33, 33] > 0
    assert sizes[labels[15, 25]] == 120


def test_extract_keeps_lakes_above_sea_level() -> None:
    """Lakes A and C are kept, the polder and the puddle are not."""
    land, heights = _world()
    params = lakes.LakesParams(min_area_px=10)
    found, below_sea = lakes.extract(land, heights, 719.0, params)
    assert below_sea == 1
    assert [lake["level_m"] for lake in found] == [100.0, 50.0]
    first = found[0]
    assert first["id"] == "lake_000"
    assert first["area_km2"] == round(120 * 719.0**2 / 1e6, 1)
    # The outline hugs the pixel edges: x in [20, 32], y in [10, 20].
    polygon = Polygon(first["polygon_px"])
    minx, miny, maxx, maxy = polygon.bounds
    assert 19.4 <= minx <= 20.6 and 31.4 <= maxx <= 32.6
    assert 9.4 <= miny <= 10.6 and 19.4 <= maxy <= 20.6
    assert polygon.is_valid
    assert polygon.contains(Point(first["center_px"]))


def test_extract_excludes_reservoirs_and_names_lakes() -> None:
    """Excluded labels are skipped; names are carried over."""
    land, heights = _world()
    labels = lakes.inland_water_labels(land)
    reservoir = int(labels[0][48, 12])
    lake_a = int(labels[0][15, 25])
    found, _ = lakes.extract(
        land,
        heights,
        719.0,
        lakes.LakesParams(min_area_px=10),
        excluded={reservoir: "test_dam"},
        names={lake_a: "Lac A"},
        labels=labels,
    )
    assert [lake["name"] for lake in found] == ["Lac A"]


def test_label_near_finds_water_around_a_point() -> None:
    """A reservoir point on the shore still finds its water body."""
    land, _ = _world()
    labels, _ = lakes.inland_water_labels(land)
    assert lakes._label_near(labels, 18.5, 48.5, 2) == labels[48, 12]
    assert lakes._label_near(labels, 45.5, 5.5, 2) == 0


def test_outline_simplification_reduces_vertices() -> None:
    """A round lake keeps its shape with far fewer vertices."""
    yy, xx = np.mgrid[:80, :80]
    disc = (xx - 40) ** 2 + (yy - 40) ** 2 < 30**2
    fine, _ = lakes.outline(disc, 0.0)
    coarse, _ = lakes.outline(disc, 0.8)
    assert len(coarse.exterior.coords) < len(fine.exterior.coords) / 2
    assert abs(coarse.area - disc.sum()) / disc.sum() < 0.03


def test_outline_counts_islands() -> None:
    """An island inside the lake is counted, the outer ring is kept."""
    water = np.zeros((30, 30), dtype=bool)
    water[5:25, 5:25] = True
    water[12:16, 12:16] = False
    polygon, islands = lakes.outline(water, 0.5)
    assert islands == 1
    assert polygon.area > 350


def test_lake_level_is_the_mode() -> None:
    """The level ignores shore pixels blended with the banks."""
    heights = np.full((10, 10), 200.0)
    heights[2:8, 2:8] = 80.0
    heights[2, 2:8] = 95.0
    component = np.zeros((10, 10), dtype=bool)
    component[2:8, 2:8] = True
    assert lakes.lake_level(heights, component) == 80.0


def test_lakes_json_matches_schema() -> None:
    """data/map/lakes.json matches its schema and holds Lake Geneva."""
    schema = json.loads(
        (DATA / "schemas" / "lakes.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    document = json.loads((DATA / "map" / "lakes.json").read_text(encoding="utf-8"))
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors[:5]]
    geneva = [lake for lake in document["lakes"] if lake["name"] == "Léman"]
    assert geneva and 340.0 < geneva[0]["level_m"] < 400.0
