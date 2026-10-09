"""Battle ground layers: schema, per-biome materials and pack coherence (GA2, TX T2c)."""

import json
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "data"


def _document() -> dict:
    return json.loads((DATA / "fx" / "battle_ground_layers.json").read_text(encoding="utf-8"))


def test_no_poly_haven_ground_set() -> None:
    """ADR 0244: the Poly Haven ground set (``legacy``) is gone; packs are the only way."""
    document = _document()
    assert "legacy" not in document
    assert "poly_haven" not in json.dumps(document)


def test_every_role_has_a_material_for_every_biome() -> None:
    """No hole: each of the 14 biomes has a material for each role."""
    document = _document()
    for role in document["roles"]:
        assert set(document["materials"][role]) == {str(b) for b in range(1, 15)}, role


def test_materials_match_pack_manifests() -> None:
    """Layer i of the pack of biome b is the material of role i for biome b."""
    document = _document()
    for biome in range(1, 15):
        manifest = json.loads(
            (DATA / "art" / f"tx_battle_b{biome:02d}_pack.json").read_text(encoding="utf-8")
        )
        expected = [document["materials"][role][str(biome)] for role in document["roles"]]
        assert [layer["id"] for layer in manifest["layers"]] == expected, biome
        assert [layer["layer"] for layer in manifest["layers"]] == list(range(len(expected)))


def test_battle_packs_stay_within_budget() -> None:
    """Each repository pack is under 8 MB (14 packs ~ 80 MB in all)."""
    tx = REPO / "game" / "assets" / "textures" / "battle" / "tx"
    for biome in range(1, 15):
        size = sum(
            (tx / f"tx_battle_b{biome:02d}_{kind}_array.jpg").stat().st_size
            for kind in ("albedo", "normal")
        )
        assert size < 8_000_000, biome


def test_micro_grains_exist_in_pack() -> None:
    """Every grain named by the micro section is in the micro_battle pack."""
    document = _document()
    micro = document["micro"]
    manifest = json.loads((DATA / "art" / "tx_micro_battle_pack.json").read_text(encoding="utf-8"))
    ids = {layer["id"] for layer in manifest["layers"]}
    named = {micro["default"], *micro["role_grain"].values(), *micro["dry_role_grain"].values()}
    assert named <= ids, named - ids


def test_province_biomes_are_valid() -> None:
    """Province -> biome table covers the provinces with biomes 1..14."""
    table = json.loads(
        (DATA / "fx" / "battle_province_biomes.json").read_text(encoding="utf-8")
    )["provinces"]
    assert len(table) == len(list((DATA / "provinces").glob("prov_*.json")))
    assert all(1 <= biome <= 14 for biome in table.values())


def test_catalogue_packs_declare_biome_packs() -> None:
    """ground_battle.yaml has one pack per biome, with the borrow for flagged rock."""
    catalogue = yaml.safe_load(
        (DATA / "art" / "textures" / "ground_battle.yaml").read_text(encoding="utf-8")
    )
    packs = {pack["biome"]: pack for pack in catalogue["packs"]}
    assert sorted(packs) == list(range(1, 15))
    assert packs[2]["borrow"] == {"rock": 1}
