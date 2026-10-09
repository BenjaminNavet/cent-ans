"""Vegetation texture catalogues (TX 3): ground cards, battle grass, bark and leaves."""

from collections import Counter
from pathlib import Path

import pytest
import yaml

from cent_ans_tools.texture_factory.catalog import load_catalog

SPECIES_PATH = (
    Path(__file__).resolve().parents[2] / "data" / "art" / "tree_species.yaml"
)
GRASS_GROUPS = ("green", "dry", "steppe", "alpine", "arctic")
GRASS_ROLES = ("grass_blades", "grass_tufts", "grass_clump")
TREES = (
    "oak",
    "beech",
    "birch",
    "scots_pine",
    "spruce",
    "olive",
    "holm_oak",
    "aleppo_pine",
    "date_palm",
    "poplar",
    "willow",
    "larch",
)


@pytest.fixture(scope="module")
def cards():
    """Vegetation cards catalogue."""
    return load_catalog("vegetation_cards")


@pytest.fixture(scope="module")
def bark():
    """Foliage and bark catalogue."""
    return load_catalog("foliage_bark")


def test_ids_and_seeds_unique(cards, bark):
    """Check."""
    ids = [e["id"] for c in (cards, bark) for e in c["entries"]]
    seeds = [e["seed"] for c in (cards, bark) for e in c["entries"]]
    assert len(ids) == len(set(ids))
    assert len(seeds) == len(set(seeds))


def test_cards_alpha_and_size(cards):
    """Check."""
    assert cards["size"] == 1024
    assert all(e["alpha"] for e in cards["entries"])


def test_each_biome_has_three_ground_cards(cards):
    """Check."""
    counts = Counter(
        b for e in cards["entries"] if e["role"] == "ground_card" for b in e["biomes"]
    )
    for biome in range(1, 15):
        assert counts[biome] >= 3, biome


def test_grass_groups_and_roles(cards):
    """Check."""
    for role in GRASS_ROLES:
        entries = [e for e in cards["entries"] if e["role"] == role]
        assert sorted(e["id"] for e in entries) == sorted(
            f"{role}_{g}" for g in GRASS_GROUPS
        )
        covered = [b for e in entries for b in e["biomes"]]
        assert sorted(covered) == list(range(1, 15))


def test_trees_two_roles_and_species(bark):
    """Check."""
    assert bark["size"] == 2048
    roles = {}
    for entry in bark["entries"]:
        roles.setdefault(entry["id"].rsplit("_", 1)[0], set()).add(entry["role"])
        assert (entry["role"] == "leaves") == bool(entry.get("alpha"))
    assert set(roles) == set(TREES)
    assert all(r == {"bark", "leaves"} for r in roles.values())
    known = {s["id"] for s in yaml.safe_load(SPECIES_PATH.read_text())["species"]}
    for entry in bark["entries"]:
        if "species" in entry:
            assert entry["species"] in known
