"""HB4 tree species catalogue: schema and coverage of every biome."""

import json
from pathlib import Path

import yaml
from jsonschema import Draft202012Validator

REPO = Path(__file__).resolve().parents[2]
SCHEMA = json.loads(
    (REPO / "data/schemas/art_tree_species.schema.json").read_text(encoding="utf-8")
)
CATALOGUE = yaml.safe_load(
    (REPO / "data/art/tree_species.yaml").read_text(encoding="utf-8")
)
COMPILED = json.loads((REPO / "data/art/tree_species.json").read_text(encoding="utf-8"))


def test_catalogue_and_table_match_schema() -> None:
    """The YAML source and its JSON compilation both validate."""
    validator = Draft202012Validator(SCHEMA)
    assert not list(validator.iter_errors(CATALOGUE))
    assert not list(validator.iter_errors(COMPILED))


def test_first_rows_are_the_fc2_essences() -> None:
    """Rows 0-2 stay oak, beech, fir: the FC atlas and meshes remain valid fallbacks."""
    ids = [s["id"] for s in COMPILED["species"]]
    assert ids[:3] == ["oak", "beech", "fir"]
    assert [s["row"] for s in COMPILED["species"]] == list(range(len(ids)))
    assert 16 <= len(ids) <= 32


def test_every_biome_has_two_species_per_main_role() -> None:
    """Each biome offers at least two species for its forests and its open land."""
    for biome in map(str, range(1, 8)):
        forest = {
            s["id"]
            for s in COMPILED["species"]
            if s["biomes"].get(biome, 0) > 0
            and max(s["roles"].get("massif", 0), s["roles"].get("lisiere", 0)) > 0
        }
        riparian = {
            s["id"]
            for s in COMPILED["species"]
            if s["biomes"].get(biome, 0) > 0 and s["roles"].get("ripisylve", 0) > 0
        }
        assert len(forest) >= 2, (biome, forest)
        assert len(riparian) >= 1, (biome, riparian)


def test_dry_mediterranean_has_no_temperate_broadleaves() -> None:
    """Player request: no temperate deciduous forest trees in the dry Mediterranean biomes."""
    temperate = {"oak", "beech", "maple", "birch", "apple"}
    for biome in ("3", "7"):
        present = {
            s["id"] for s in COMPILED["species"] if s["biomes"].get(biome, 0) > 0
        }
        assert not present & temperate, (biome, present & temperate)
