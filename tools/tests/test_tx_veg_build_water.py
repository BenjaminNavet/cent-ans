"""TX T3/T4 : cartes, écorces, matières de bâtiments par région et eau (paquets et données liées)."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest
import yaml
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools.texture_factory.cards import (
    defringe,
    make_card,
    mean_linear_luma,
    plant_bbox,
)
from cent_ans_tools.texture_factory.catalog import load_catalog
from cent_ans_tools.texture_factory.regions import region_materials
from cent_ans_tools.texture_factory.tiles import plain_normal

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / "data" / "art"


def _json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_catalogs_declare_valid_packs() -> None:
    for family in (
        "vegetation_cards",
        "foliage_bark",
        "building_materials",
        "water_surfaces",
    ):
        document = load_catalog(family)
        assert document["packs"], family
        for pack in document["packs"]:
            assert pack.get("kind", "tile") in {"tile", "cards", "sheet", "files", "tiles"}


def test_defringe_floods_colour_and_erodes_alpha() -> None:
    rgba = np.zeros((16, 16, 4), dtype=np.uint8)
    rgba[4:12, 4:12] = (200, 30, 30, 255)
    rgba[3, 4:12] = (230, 230, 230, 120)  # grey backdrop fringe
    out = defringe(rgba, erode=1)
    assert out[0, 0, 3] == 0
    assert out[0, 0, 0] == 200  # transparent pixels take the nearest solid colour
    assert out[8, 8, 3] == 255


def test_make_card_stands_plant_on_bottom_edge(tmp_path: Path) -> None:
    rgba = np.zeros((200, 200, 4), dtype=np.uint8)
    rgba[40:120, 80:120] = (30, 160, 30, 255)  # a tall plant, 40 x 80
    src = tmp_path / "plant.png"
    Image.fromarray(rgba, "RGBA").save(src)
    card, aspect = make_card(src, 128)
    assert card.size == (128, 128)
    assert aspect == pytest.approx(0.5, abs=0.05)
    box = plant_bbox(np.asarray(card)[..., 3])
    assert box is not None
    assert box[3] >= 120  # bottom aligned
    assert box[3] - box[1] >= 115  # longest side fills the cell
    assert mean_linear_luma(card) > 0.0


def test_plain_normal_has_blue_up_and_slope() -> None:
    flat = np.full((8, 8, 3), 128, dtype=np.uint8)
    image, slope = plain_normal(flat)
    assert slope < 0.02
    assert np.asarray(image)[..., 2].min() >= 250


def test_every_region_covers_every_building_role() -> None:
    document = load_catalog("building_materials")
    table = region_materials(document)
    regions = _json(ART / "building_regions.json")["regions"]
    roles = {entry["role"] for entry in document["entries"]}
    assert set(regions) <= set(table), "région sans matière régionale"
    for region, materials in table.items():
        assert set(materials) == roles, (region, roles - set(materials))


def test_building_regions_materials_match_the_pack_and_schema() -> None:
    doc = _json(ART / "building_regions.json")
    schema = _json(ROOT / "data" / "schemas" / "art_building_regions.schema.json")
    assert not list(Draft202012Validator(schema).iter_errors(doc))
    pack_ids = {layer["id"] for layer in _json(ART / "tx_building_pack.json")["layers"]}
    for region, style in doc["regions"].items():
        assert style.get("materials"), region
        assert set(style["materials"].values()) <= pack_ids, region


def test_building_pack_layers_fit_the_shader_tables() -> None:
    pack = _json(ART / "tx_building_pack.json")
    assert len(pack["layers"]) <= 96  # `regional_tile[96]`
    assert len(_json(ART / "building_regions.json")["regions"]) <= 40  # `region_table[640]`


def test_ground_cards_five_per_biome() -> None:
    pack = _json(ART / "tx_ground_cards_pack.json")
    per_biome: dict[int, int] = {}
    for layer in pack["layers"]:
        for biome in layer["biomes"]:
            per_biome[biome] = per_biome.get(biome, 0) + 1
    assert sorted(per_biome) == list(range(1, 15))
    assert all(count >= 5 for count in per_biome.values()), per_biome
    assert len(pack["layers"]) <= 96


def test_battle_grass_has_three_cards_per_group() -> None:
    pack = _json(ART / "tx_battle_grass_pack.json")
    groups: dict[str, set[str]] = {}
    for card in pack["cards"]:
        group = card["id"].rsplit("_", 1)[1]
        groups.setdefault(group, set()).add(card["role"])
    assert set(groups) == {"green", "dry", "steppe", "alpine", "arctic"}
    assert all(len(roles) == 3 for roles in groups.values())
    biomes = {b for card in pack["cards"] for b in card["biomes"]}
    assert biomes >= set(range(1, 15)) - {1000}


def test_tree_species_textures_point_into_the_packs() -> None:
    bark = {layer["id"] for layer in _json(ART / "tx_bark_pack.json")["layers"]}
    leaves = {layer["id"] for layer in _json(ART / "tx_leaves_pack.json")["layers"]}
    table = yaml.safe_load((ART / "tree_species.yaml").read_text(encoding="utf-8"))
    pointed = [s for s in table["species"] if "textures" in s]
    assert len(pointed) >= 10
    for species in pointed:
        assert species["textures"]["bark"] in bark, species["id"]
        assert species["textures"]["leaves"] in leaves, species["id"]
    compiled = _json(ART / "tree_species.json")["species"]
    assert {s["id"] for s in compiled if "textures" in s} == {s["id"] for s in pointed}


def test_water_detail_tx_ids_exist_in_the_water_pack() -> None:
    pack = {layer["id"]: layer for layer in _json(ART / "tx_water_pack.json")["layers"]}
    detail = _json(ROOT / "data" / "fx" / "water_detail.json")
    for name, material in detail["materials"].items():
        if "tx" in material:
            assert material["tx"] in pack, name
    for slot, material_id in detail["basins"].items():
        assert detail["materials"][material_id]["tx"] in pack, slot
        assert not pack[detail["materials"][material_id]["tx"]]["flat"], slot
