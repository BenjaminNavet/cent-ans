"""Lot ME8: regional agricultural landscapes (data file and baked mask)."""

import json
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.geo import agri_regions
from cent_ans_tools.geo.project import grid_from_metadata

MAP = Path(__file__).resolve().parents[2] / "data" / "map"


def _document() -> dict:
    return json.loads((MAP / "agri_landscapes.json").read_text(encoding="utf-8"))


def test_regions_reference_known_landscapes_and_rows_fit_the_table() -> None:
    """Every region names a landscape and rows stay inside the 16-row table."""
    document = _document()
    rows = agri_regions.landscape_rows(document)
    assert max(rows.values()) <= 15
    for region in document["regions"]:
        assert region["landscape"] in rows


def test_seasonal_variants_exist_in_the_catalogue() -> None:
    """Seasonal tree models are ids of the map-extra catalogue."""
    catalogue = json.loads(
        (MAP.parent / "art" / "dn_catalog_map_extra.json").read_text(encoding="utf-8")
    )
    known = {entry["id"] for entry in catalogue}
    for variant in _document()["seasonal_variants"]:
        assert variant["model_id"] in known


def test_baked_mask_places_landscapes_where_expected() -> None:
    """Reference points land in the intended landscape; open sea stays empty."""
    document = _document()
    rows = agri_regions.landscape_rows(document)
    grid = grid_from_metadata(
        json.loads((MAP / "map.json").read_text(encoding="utf-8"))
    )
    mask = np.asarray(Image.open(MAP / "agri_regions.png"))

    def at(lon: float, lat: float) -> int:
        x, y = grid.lonlat_to_pixel(lon, lat)
        return int(mask[int(y) // 4, int(x) // 4])

    assert at(1.8, 48.3) == rows["openfield_north"]  # Beauce
    assert at(-3.0, 48.1) == rows["bocage_west"]  # Brittany
    assert at(5.4, 43.55) == rows["olive_orchard_south"]  # Aix, Provence
    assert at(-7.6, 41.15) == rows["vineyard_terraces"]  # Douro
    assert at(-0.4, 39.4) == rows["irrigated_huerta"]  # Valencia
    assert at(34.0, 57.5) == rows["swidden_east"]  # Tver
    assert at(0.0, 60.0) == 0  # North Sea
