"""Tests of the vector heraldic charges (lot DA1b): data, licences and blazon regression.

The faction shields changed on purpose with DA1b (drawn charges instead of polygons), so the
shields are no longer compared byte for byte with a previous build. Instead each shield is
checked against its blazon: the tinctures it names must cover a share of the shield, and the
drawn charges must carry their inner lines.
"""

import json
import re
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import banners, heraldic_charges, heraldry

DATA = Path(__file__).resolve().parents[2] / "data"
CHARGES = DATA / "heraldry" / "charges"


def _manifest() -> dict:
    return json.loads((CHARGES / "charges.json").read_text(encoding="utf-8"))


def test_manifest_matches_schema() -> None:
    """charges.json matches its schema (licences restricted to PD, CC0, CC BY)."""
    schema = json.loads(
        (DATA / "schemas" / "heraldry_charges.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_manifest()))
    assert not errors, [error.message for error in errors]


def test_every_charge_is_credited_and_every_colour_has_a_role() -> None:
    """Each SVG exists, is listed in SOURCE.md, and all its colours are mapped."""
    source = (CHARGES / "SOURCE.md").read_text(encoding="utf-8")
    for charge_id, entry in _manifest()["charges"].items():
        path = CHARGES / entry["file"]
        assert path.exists(), charge_id
        assert f"`{entry['file']}`" in source, charge_id
        text = path.read_text(encoding="utf-8")
        colours = {
            heraldic_charges._long_hex(c)
            for c in re.findall(r"#(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{3})\b", text)
        }
        mapped = set(entry["colors"]) | set(entry.get("drop", []))
        # Inkscape page settings (#666666 border, #ffffff page) are harmless if unmapped.
        missing = colours - mapped - {"#666666", "#ffffff"}
        assert not missing, (charge_id, missing)


@pytest.mark.parametrize("charge_id", heraldic_charges.charge_ids())
def test_charge_masks_have_body_and_lines(charge_id: str) -> None:
    """Every drawing yields a painted role and an outline, cropped to the drawing."""
    art = heraldic_charges.charge_art(charge_id)
    assert "outline" in art.masks
    painted = [role for role in art.masks if role not in ("outline", "shade")]
    assert painted
    for mask in art.masks.values():
        assert mask.size == art.size
    outline = art.masks["outline"].histogram()
    assert sum(outline[128:]) > 0.01 * art.size[0] * art.size[1]


def _share(
    image: Image.Image, color: tuple[int, int, int], tolerance: int = 60
) -> float:
    """Share of the opaque pixels of ``image`` close to ``color``."""
    array = np.asarray(image.convert("RGBA"), dtype=np.int32)
    opaque = array[..., 3] == 255
    distance = np.abs(array[..., :3] - np.array(color)).sum(axis=-1)
    return float((opaque & (distance < tolerance)).sum() / opaque.sum())


T = heraldry.TINCTURES
# Blazon regression: (shield, {tincture: minimum share of the shield}), about 60 % of
# the shares measured when DA1b was drawn (shading and outlines lower the raw shares).
EXPECTED = [
    ("fac_england", {"gueules": 0.4, "or": 0.04}),
    ("fac_empire", {"or": 0.18, "sable": 0.3}),
    ("fac_flanders", {"or": 0.3, "sable": 0.18}),
    ("fac_brabant", {"sable": 0.5, "or": 0.04}),
    ("fac_bohemia", {"gueules": 0.4, "argent": 0.035}),
    ("fac_guelders", {"azur": 0.4, "or": 0.04}),
    ("fac_milan", {"argent": 0.18, "azur": 0.06}),
    ("fac_scotland", {"or": 0.3, "gueules": 0.12}),
    ("fac_venice", {"gueules": 0.4, "or": 0.04}),
    ("fac_castile", {"gueules": 0.2, "argent": 0.12, "or": 0.006, "pourpre": 0.012}),
    ("fac_hainaut", {"or": 0.4, "sable": 0.05, "gueules": 0.012}),
    ("armagnac", {"argent": 0.09, "gueules": 0.24, "or": 0.013}),
    ("habsbourg", {"or": 0.4, "gueules": 0.035}),
    ("luxembourg", {"argent": 0.12, "azur": 0.24, "gueules": 0.035}),
    ("la_tour_du_pin", {"or": 0.36, "azur": 0.04, "gueules": 0.015}),
    ("lancastre", {"gueules": 0.35, "or": 0.04, "azur": 0.06}),
    ("plantagenet", {"azur": 0.2, "or": 0.035, "gueules": 0.22}),
    ("bohun", {"azur": 0.3, "argent": 0.06, "or": 0.04}),
]


def _shield(shield_id: str) -> Image.Image:
    factions = {faction["id"]: faction for faction in heraldry.load_factions()}
    if shield_id in factions:
        return heraldry.shield_for_faction(factions[shield_id])
    houses = {house["id"]: house for house in heraldry.load_houses()["houses"]}
    return heraldry.shield_for_house(houses[shield_id], factions)


@pytest.mark.parametrize(("shield_id", "tinctures"), EXPECTED)
def test_shield_shows_the_tinctures_of_its_blazon(shield_id, tinctures) -> None:
    """Field and charges keep the tinctures named by the blazon."""
    image = _shield(shield_id)
    for name, minimum in tinctures.items():
        share = _share(image, T[name])
        assert share >= minimum, (shield_id, name, round(share, 3))


def test_drawn_charges_have_inner_lines() -> None:
    """A drawn lion shows its outline and inner lines, not a flat silhouette."""
    lion = _shield("fac_guelders")
    dark = _share(lion, heraldry.OUTLINE, tolerance=120)
    assert dark > 0.1


def test_lion_variants_follow_the_blazon() -> None:
    """Crown, forked tail and the winged lion of Saint Mark are read from the text."""
    assert heraldry.lion_variant("de gueules couronne d'or") == ("lion_couronne", False)
    assert heraldry.lion_variant("d'argent a la queue fourchee, couronne d'or") == (
        "lion_queue_fourchee",
        True,
    )
    assert heraldry.lion_variant("de gueules, la queue fourchee et passee en ")[0] == (
        "lion_queue_fourchee_sautoir"
    )
    assert heraldry.lion_variant("de saint marc d'or")[0] == "lion_aile"
    assert heraldry.armed_tincture("arme et lampasse d'azur", T["or"]) == T["azur"]
    assert heraldry.armed_tincture("", T["gueules"]) == T["azur"]
    assert heraldry.crown_tincture("arme, lampasse et couronne d'azur") == T["azur"]


def test_house_banners_are_built(tmp_path: Path) -> None:
    """Every house gets a banner, a pennon and a standard (EP5 standards of the general)."""
    paths = banners.build_houses(tmp_path)
    houses = heraldry.load_houses()["houses"]
    assert len(paths) == 3 * len(houses)
    banner = Image.open(tmp_path / "plantagenet_banner.png")
    assert banner.size == banners.BANNER_SIZE
    assert banner.getpixel((128, 100))[3] == 255
    assert banners.house_livery("D'or au lion de gueules.") == (T["or"], T["gueules"])
