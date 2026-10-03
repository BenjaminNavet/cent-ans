"""Water of 1340 in the land mask (lot LR-10): historical lakes in, reservoirs out."""

import json
from pathlib import Path

import geopandas as gpd
import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image
from shapely.geometry import Point, box

from cent_ans_tools.geo import historical_water, lakes, project

DATA = Path(__file__).resolve().parents[2] / "data"
MAP = DATA / "map"

Image.MAX_IMAGE_PIXELS = None


def _historical() -> dict:
    return json.loads((MAP / "historical_lakes.json").read_text(encoding="utf-8"))


def test_historical_lakes_match_schema() -> None:
    """historical_lakes.json is valid, ids are unique, Fucino is there."""
    schema = json.loads(
        (DATA / "schemas" / "historical_lakes.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    document = _historical()
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors[:5]]
    ids = [entry["id"] for entry in document["lakes"]]
    assert len(ids) == len(set(ids))
    assert {"fucino", "haarlemmermeer", "grand_lieu", "loch_ness", "berre"} <= set(ids)


def test_reservoir_rows_need_the_point_and_a_close_area() -> None:
    """A lake polygon holding a dam point is a reservoir unless it is far larger."""
    frame = gpd.GeoDataFrame(
        geometry=[
            box(6.10, 43.72, 6.22, 43.80),  # about 95 km2, holds the point
            box(5.00, 45.00, 5.10, 45.05),  # elsewhere
            box(4.00, 40.00, 8.00, 44.00),  # huge, holds the second point
        ],
        crs=project.CRS_GEO,
    )
    reservoirs = [
        {"id": "dam_a", "lon": 6.16, "lat": 43.76, "area_km2": 22.0},
        {"id": "dam_b", "lon": 7.50, "lat": 41.00, "area_km2": 5.0},
    ]
    assert historical_water.reservoir_lakes(frame, reservoirs) == {0: "dam_a"}


def test_refill_gives_new_land_the_nearest_province() -> None:
    """Pixels turned to land take a neighbour's id; others are untouched."""
    ids = np.zeros((6, 8), dtype=np.int32)
    ids[:, :3] = 4
    ids[:, 5:] = 9
    old_land = ids > 0
    new_land = old_land.copy()
    new_land[:, 3:5] = True
    out, count = historical_water.refill_provinces(ids, old_land, new_land)
    assert count == 12
    assert (out[:, 3] == 4).all() and (out[:, 4] == 9).all()
    assert (out[:, :3] == 4).all() and (out[:, 5:] == 9).all()


def test_merge_replaces_auto_lakes_inside_a_historical_basin() -> None:
    """An extracted lake centred in a historical basin gives way to its sheet."""
    basin = np.zeros((20, 20), dtype=bool)
    basin[2:8, 2:8] = True
    auto = [
        {"id": "lake_000", "area_km2": 5.0, "center_px": [5.0, 5.0]},
        {"id": "lake_001", "area_km2": 3.0, "center_px": [15.0, 15.0]},
    ]
    sheet = {"id": "", "area_km2": 18.0, "center_px": [5.0, 5.0], "historical": "x"}
    merged = lakes.merge_historical(auto, [(sheet, basin)])
    assert [lake.get("historical") for lake in merged] == ["x", None]
    assert [lake["id"] for lake in merged] == ["lake_000", "lake_001"]


def _pixel(grid: project.MapGrid, lon: float, lat: float) -> tuple[int, int]:
    x, y = grid.lonlat_to_pixel(lon, lat)
    return int(y), int(x)


def test_land_mask_drops_reservoirs_and_holds_historical_lakes() -> None:
    """The committed mask: the five reservoirs are land, drained lakes are water."""
    meta = json.loads((MAP / "map.json").read_text(encoding="utf-8"))
    grid = project.grid_from_metadata(meta)
    land = np.asarray(Image.open(MAP / "land_mask.png")) >= 128
    reservoirs = {r["id"]: r for r in historical_water.load_reservoirs()}
    for rid in ("sainte_croix", "der_chantecoq", "orient", "ebro", "riano"):
        row, col = _pixel(grid, reservoirs[rid]["lon"], reservoirs[rid]["lat"])
        assert land[row - 1 : row + 2, col - 1 : col + 2].all(), rid
    for lon, lat in ((13.55, 41.99), (23.12, 38.47), (36.33, 36.33), (4.68, 52.33)):
        row, col = _pixel(grid, lon, lat)
        assert not land[row, col], (lon, lat)


def test_lakes_json_holds_the_historical_sheets() -> None:
    """Every historical lake with a level has a named sheet in lakes.json."""
    document = json.loads((MAP / "lakes.json").read_text(encoding="utf-8"))
    sheets = {
        lake["historical"]: lake for lake in document["lakes"] if "historical" in lake
    }
    for entry in _historical()["lakes"]:
        if entry["level_m"] is None:
            assert entry["id"] not in sheets
            continue
        sheet = sheets[entry["id"]]
        assert sheet["name"] == entry["name"]
        assert sheet["level_m"] >= entry["level_m"]
    fucino = sheets["fucino"]
    assert 120.0 < fucino["area_km2"] < 180.0
    assert Point(fucino["center_px"]).within(
        box(
            *np.asarray(fucino["polygon_px"]).min(0),
            *np.asarray(fucino["polygon_px"]).max(0),
        )
    )
