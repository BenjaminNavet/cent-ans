"""Ground texture catalogues (TX 2b/2c): schema, ids, biome coverage, fal cost."""

from collections import Counter

import pytest

from cent_ans_tools.texture_factory.catalog import build_prompt, load_catalog
from cent_ans_tools.texture_factory.generate import estimate_cost

BIOMES = range(1, 15)
BATTLE_ROLES = {
    "grass",
    "meadow",
    "understory",
    "dirt",
    "mud",
    "gravel",
    "rock",
    "plowed",
    "snow",
    "flower_meadow",
    "trampled_grass",
    "stubble",
    "fresh_plow",
}


@pytest.fixture(scope="module")
def campaign():
    """Campaign."""
    return load_catalog("ground_campaign")


@pytest.fixture(scope="module")
def battle():
    """Battle."""
    return load_catalog("ground_battle")


def test_both_load_with_unique_ids_and_seeds(campaign, battle):
    """Test both load with unique ids and seeds."""
    for document in (campaign, battle):
        ids = [entry["id"] for entry in document["entries"]]
        assert len(ids) == len(set(ids))
        seeds = [entry["seed"] for entry in document["entries"]]
        assert len(seeds) == len(set(seeds))
        assert document["backend"] == "fal"
        assert document["size"] == 2048


def test_campaign_background_covers_three_roles_per_biome(campaign):
    """Test campaign background covers three roles per biome."""
    for role in ("grass_bare", "understory", "rock"):
        counts = Counter(
            biome
            for entry in campaign["entries"]
            if entry["role"] == role
            for biome in entry["biomes"]
        )
        assert set(counts) == set(BIOMES), role
        assert max(counts.values()) == 1, role


def test_campaign_parcel_materials(campaign):
    """Test campaign parcel materials."""
    parcels = [e for e in campaign["entries"] if e["id"].startswith("parcel_")]
    assert len(parcels) == 27 + 15
    for entry in campaign["entries"]:
        assert 50 <= entry["tile_m"] <= 200
        assert entry["checks"]["seam_max"] == 0.08


def test_battle_roles_covered_exactly_once_per_biome(battle):
    """Test battle roles covered exactly once per biome."""
    assert {entry["role"] for entry in battle["entries"]} == BATTLE_ROLES
    for role in BATTLE_ROLES:
        counts = Counter(
            biome
            for entry in battle["entries"]
            if entry["role"] == role
            for biome in entry["biomes"]
        )
        assert sorted(counts) == list(BIOMES), role
        assert set(counts.values()) == {1}, role
    assert len(battle["entries"]) <= 130
    assert all(2 <= entry["tile_m"] <= 9 for entry in battle["entries"])


def test_prompts_have_no_frame_inducing_words(campaign, battle):
    """Test prompts have no frame inducing words."""
    for document in (campaign, battle):
        for entry in document["entries"]:
            prompt = build_prompt(document, entry)
            assert "{" not in prompt
            assert "texture" not in entry["prompt"].lower()


def test_estimated_fal_cost_under_five_dollars(campaign, battle):
    """Test estimated fal cost under five dollars."""
    total = estimate_cost(campaign, campaign["entries"]) + estimate_cost(
        battle, battle["entries"]
    )
    assert 0 < total < 5.0
