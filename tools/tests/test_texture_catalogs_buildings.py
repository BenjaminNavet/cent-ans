"""Building material catalogue (TX 4): schema, ids, roles, regions, wall and roof coverage."""

import json
from pathlib import Path

import pytest

from cent_ans_tools.texture_factory.catalog import build_prompt, load_catalog

ART_DIR = Path(__file__).resolve().parents[2] / "data" / "art"
WALL_ROLES = {
    "Plaster",
    "Rubble",
    "Ashlar",
    "Masonry",
    "Timber",
    "Planks",
    "TimberFrame",
}
ROOF_ROLES = {"RoofTile", "RoofFlat", "RoofSlate", "Thatch"}


def _known_roles():
    document = json.loads((ART_DIR / "building_materials.json").read_text())
    return {material["name"] for material in document["textured"]}


def _known_regions():
    return json.loads((ART_DIR / "building_regions.json").read_text())["regions"]


@pytest.fixture(scope="module")
def catalog():
    """Catalog."""
    return load_catalog("building_materials")


def test_roles_exist_in_building_materials():
    """Test roles exist in building materials."""
    assert _known_roles() >= (WALL_ROLES | ROOF_ROLES)


def test_ids_and_seeds_unique(catalog):
    """Test ids and seeds unique."""
    ids = [entry["id"] for entry in catalog["entries"]]
    seeds = [entry["seed"] for entry in catalog["entries"]]
    assert len(ids) == len(set(ids))
    assert len(seeds) == len(set(seeds))
    assert all(seed >= 9000 for seed in seeds)


def test_roles_and_regions_known(catalog):
    """Test roles and regions known."""
    roles = _known_roles()
    regions = set(_known_regions())
    for entry in catalog["entries"]:
        assert entry["role"] in roles, entry["id"]
        assert set(entry["regions"]) <= regions, entry["id"]


def test_every_region_covers_every_wall_and_roof_role(catalog):
    """Test every region covers every wall and roof role."""
    covered = {
        (region, entry["role"])
        for entry in catalog["entries"]
        for region in entry["regions"]
    }
    missing = [
        (region, role)
        for region in _known_regions()
        for role in sorted(WALL_ROLES | ROOF_ROLES)
        if (region, role) not in covered
    ]
    assert not missing, missing


def test_tile_sizes_realistic(catalog):
    """Test tile sizes realistic."""
    for entry in catalog["entries"]:
        low, high = (2.0, 3.0) if entry["role"] in ROOF_ROLES else (1.5, 4.0)
        assert low <= entry["tile_m"] <= high, entry["id"]


def test_prompts_positive_only(catalog):
    """Test prompts positive only."""
    for entry in catalog["entries"]:
        prompt = build_prompt(catalog, entry).lower()
        assert " no " not in f" {prompt} ", entry["id"]
        assert " not " not in f" {prompt} ", entry["id"]
