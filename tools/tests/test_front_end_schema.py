"""Validates data/ui/front_end.json (lot MM1: menu backdrop, faction select, intro, loading)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def _res_exists(path: str) -> bool:
    return (ROOT / "game" / path.removeprefix("res://")).is_file()


def test_front_end_references_exist() -> None:
    """Every image, model, sky, banner and faction referenced exists."""
    front_end = _load("ui/front_end.json")
    backdrop = front_end["backdrop"]
    assert _res_exists(backdrop["model"]), backdrop["model"]
    assert backdrop["sky"] in _load("fx/atmosphere.json")["skies"]
    banners = ROOT / "game" / "assets" / "heraldry" / "banners"
    for regiment in backdrop.get("host", {}).get("regiments", []):
        assert (DATA / "factions" / f"{regiment['faction']}.json").is_file()
        if "banner" in regiment:
            assert (banners / regiment["banner"]).is_file(), regiment["banner"]
    for loop in backdrop.get("ambience", []):
        assert _res_exists(loop["stream"]), loop["stream"]
    paths = [card["illustration"] for card in front_end["intro"]["cards"]]
    paths += front_end["loading"]["illustrations"]
    for faction_paths in front_end["loading"].get("faction_illustrations", {}).values():
        paths += faction_paths
    for faction in front_end["factions"]:
        if "illustration" in faction:
            paths.append(faction["illustration"])
        data = _load(f"factions/{faction['id']}.json")
        assert data.get("playable"), faction["id"]
        assert faction["difficulty"] < len(front_end["difficulty_labels"])
    for path in paths:
        assert _res_exists(path), path


def test_all_playable_factions_are_presented() -> None:
    """Each playable faction of data/factions has a presentation card."""
    presented = {faction["id"] for faction in _load("ui/front_end.json")["factions"]}
    playable = {
        path.stem
        for path in (DATA / "factions").glob("*.json")
        if json.loads(path.read_text(encoding="utf-8")).get("playable")
    }
    assert playable == presented


def test_special_starts_are_presented() -> None:
    """JR6: the singular challenges tab only names presented (playable) factions."""
    front_end = _load("ui/front_end.json")
    presented = {faction["id"] for faction in front_end["factions"]}
    special = front_end.get("special_starts", {}).get("factions", [])
    assert "fac_crusaders" in special
    assert set(special) <= presented
