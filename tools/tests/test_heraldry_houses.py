"""Tests for the house arms (lot DA1): data, schema and the grammar v2 renderer."""

import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import heraldry

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def _digest(image: Image.Image) -> str:
    return hashlib.sha256(image.tobytes()).hexdigest()


def test_houses_match_schema() -> None:
    """houses.json exists and matches its schema."""
    schema = _load("schemas/heraldry_houses.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("heraldry/houses.json"))
    )
    assert not errors, [error.message for error in errors]


def test_every_character_house_has_arms() -> None:
    """Each `house` of data/characters has exactly one entry, ids are unique."""
    houses = _load("heraldry/houses.json")["houses"]
    names = [house["name"] for house in houses]
    ids = [house["id"] for house in houses]
    assert len(set(names)) == len(names)
    assert len(set(ids)) == len(ids)
    used = {
        json.loads(path.read_text(encoding="utf-8"))["house"]
        for path in (DATA / "characters").glob("*.json")
    }
    assert used <= set(names), used - set(names)


def test_faction_references_exist() -> None:
    """`arms_of`, `factions`, `vassal_of` and badge keys name existing factions."""
    factions = {path.stem for path in (DATA / "factions").glob("*.json")}
    data = _load("heraldry/houses.json")
    assert set(data["badges"]) <= factions
    for house in data["houses"]:
        referenced = set(house["factions"]) | set(house["vassal_of"])
        if "arms_of" in house:
            referenced.add(house["arms_of"])
        assert referenced <= factions, (house["id"], referenced - factions)


def test_grammar_v2_reads_tinctures_and_counts() -> None:
    """Numbers come from the word before, tinctures from the first one after."""
    pieces = heraldry.parse_pieces(
        heraldry.normalize_blazon("à trois fusées de gueules posées en fasce")
    )
    assert [(p.kind, p.count, p.arrangement) for p in pieces] == [("fusee", 3, "fasce")]
    assert pieces[0].paint == heraldry.TINCTURES["gueules"]
    chief = heraldry.parse_pieces(
        heraldry.normalize_blazon(
            "au chef d'argent chargé d'une croix de gueules, au lion couronné d'or"
        )
    )
    assert [p.kind for p in chief] == ["chef", "croix", "lion"]
    assert chief[1].on is chief[0]
    assert chief[2].paint == heraldry.TINCTURES["or"]
    luxembourg = heraldry.parse_pieces(
        heraldry.normalize_blazon("au lion de gueules, la queue passée en sautoir")
    )
    assert [p.kind for p in luxembourg] == ["lion"]


def test_every_house_blazon_renders() -> None:
    """All v2 blazons parse (no unreadable field) and give distinct fields."""
    houses = _load("heraldry/houses.json")["houses"]
    digests = {}
    for house in houses:
        if "arms_of" in house:
            continue
        digests[house["id"]] = _digest(heraldry.render_house_field(house["blazon"]))
    duplicates = len(digests) - len(set(digests.values()))
    assert duplicates == 0


def test_build_houses_deterministic_and_factions_unchanged(tmp_path: Path) -> None:
    """Two builds give identical pixels; arms_of houses reuse the faction drawing."""
    first = heraldry.build_houses(out_dir=tmp_path / "a")
    second = heraldry.build_houses(out_dir=tmp_path / "b")
    assert len(first) == len(_load("heraldry/houses.json")["houses"])
    for path_a, path_b in zip(first, second, strict=True):
        image = Image.open(path_a)
        assert image.size == (128, 128)
        assert image.mode == "RGBA"
        assert _digest(image) == _digest(Image.open(path_b))
    factions = {faction["id"]: faction for faction in heraldry.load_factions()}
    savoie = next(path for path in first if path.stem == "savoie")
    assert _digest(Image.open(savoie).convert("RGBA")) == _digest(
        heraldry.shield_for_faction(factions["fac_savoy"])
    )
