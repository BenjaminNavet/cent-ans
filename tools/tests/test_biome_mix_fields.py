"""The 14 biomes (ADR 0237): ground mix, field models and tree species cover 1-14."""

from __future__ import annotations

import json
import re
from pathlib import Path

import yaml

from cent_ans_tools.geo import biomes

REPO = Path(__file__).resolve().parents[2]
ART = REPO / "data" / "art"


def _load(name: str) -> dict:
    return json.loads((ART / name).read_text(encoding="utf-8"))


def test_mix_has_a_complete_entry_for_every_biome() -> None:
    """Biomes 1-14 each have crops, wild soils, canopy, rock and tint."""
    mix = _load("ground_biome_mix.json")["biomes"]
    assert sorted(mix, key=int) == [str(index) for index in range(1, 15)]
    for key, entry in mix.items():
        for field in ("cell_m", "strip", "hedge", "farm", "crops", "wild", "canopy", "rock"):
            assert field in entry, (key, field)
        assert len(entry["wild"]) == 2, key


def test_mix_materials_exist_in_the_ground_catalogue() -> None:
    """Every material the mix names is a layer of ``ground_materials.yaml``."""
    text = (ART / "ground_materials.yaml").read_text(encoding="utf-8")
    layers = set(re.findall(r"^\s*- id: (\w+)", text, re.MULTILINE))
    assert layers
    for key, entry in _load("ground_biome_mix.json")["biomes"].items():
        used = {*entry["crops"], *entry["wild"], entry["canopy"], entry["rock"]}
        assert used <= layers, (key, used - layers)


def test_every_biome_has_a_field_material_with_a_model() -> None:
    """``field_plan`` places models in every biome: at least one crop is in dn_fields.json.

    ``field_plan.gd`` itself reads the base raster (``biomes_base.png``, parents only), so
    the regional rows are complete entries for the shader and for the day it learns 8-14.
    """
    fields = set(_load("dn_fields.json")["crops"])
    for key, entry in _load("ground_biome_mix.json")["biomes"].items():
        if key == "6":  # montagnard : prés d'alpage seulement, sans modèle de champ (historique)
            continue
        assert set(entry["crops"]) & fields, f"biome {key} has no field model material"


def test_regional_biomes_differ_from_their_parent() -> None:
    """A regional entry is adapted, not a bare copy of its parent."""
    legend = biomes.load_legend()
    parent = biomes.parents(legend)
    mix = _load("ground_biome_mix.json")["biomes"]
    for index in range(8, 15):
        assert mix[str(index)] != {**mix[str(parent[index])]}, index
        assert mix[str(index)]["name"] != mix[str(parent[index])]["name"], index


def test_tree_species_cover_every_biome() -> None:
    """``tree_species.json`` has seeding parameters for 1-14 and trees in the wooded ones."""
    table = _load("tree_species.json")
    assert sorted(table["biomes"], key=int) == [str(index) for index in range(1, 15)]
    for index in (10, 11, 12, 13, 14):
        weighted = [
            species["id"]
            for species in table["species"]
            if float(species["biomes"].get(str(index), 0.0)) > 0.0
            or float(species["biomes"].get(str(biomes.parents(biomes.load_legend())[index]), 0.0)) > 0.0
        ]
        assert len(weighted) >= 2, index
    tundra = [s["id"] for s in table["species"] if float(s["biomes"].get("9", 0.0)) > 0.0]
    assert "birch" in tundra
    assert "spruce" not in tundra
    desert = [s["id"] for s in table["species"] if float(s["biomes"].get("8", 0.0)) > 0.0]
    assert "poplar" in desert
    assert "holm_oak" not in desert


def test_yaml_and_compiled_species_agree() -> None:
    """The committed JSON carries the YAML biome weights (compile step was run)."""
    source = yaml.safe_load((ART / "tree_species.yaml").read_text(encoding="utf-8"))
    compiled = _load("tree_species.json")
    assert {s["id"]: s["biomes"] for s in source["species"]} == {
        s["id"]: s["biomes"] for s in compiled["species"]
    }
