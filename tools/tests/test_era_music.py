"""DA4 tests: era music playlists and layered battle music reference real, credited files."""

import json
import re
from pathlib import Path

from jsonschema import Draft202012Validator

REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "data"
GAME = REPO / "game"
ASSETS = GAME / "assets"

CULTURE_REGIONS = {"france", "england", "burgundy", "iberia", "italy"}


def _music() -> dict:
    return json.loads((DATA / "audio" / "music.json").read_text(encoding="utf-8"))


def _battle_layers() -> dict:
    return json.loads(
        (DATA / "audio" / "battle_layers.json").read_text(encoding="utf-8")
    )


def _source_md_credits(path: Path) -> set[str]:
    """Filenames mentioned in a `SOURCE.md` next to `path` (backtick-quoted names)."""
    source = path.parent / "SOURCE.md"
    if not source.exists():
        return set()
    return set(
        re.findall(r"`([\w.\-]+\.(?:ogg|mp3|wav))`", source.read_text(encoding="utf-8"))
    )


def test_music_json_matches_schema() -> None:
    """``data/audio/music.json`` matches its schema (DA4: primary/fallback per context)."""
    schema = json.loads(
        (DATA / "schemas" / "music.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_music()))
    assert not errors, [error.message for error in errors]


def test_battle_layers_json_matches_schema() -> None:
    """``data/audio/battle_layers.json`` matches its schema (DA4: layered battle music)."""
    schema = json.loads(
        (DATA / "schemas" / "battle_layers.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_battle_layers()))
    assert not errors, [error.message for error in errors]


def _all_music_tracks() -> list[str]:
    tracks: list[str] = []
    for playlist in _music()["playlists"].values():
        tracks += playlist.get("primary", [])
        tracks += playlist.get("fallback", [])
    return tracks


def test_every_music_track_exists() -> None:
    """Every track referenced by a playlist is a real file under `game/assets/`."""
    missing = [track for track in _all_music_tracks() if not (GAME / track).is_file()]
    assert not missing, f"tracks manquantes : {missing}"


def test_every_third_party_music_track_is_credited() -> None:
    """Every third-party (Wikimedia / Kevin MacLeod) track is listed in its `SOURCE.md`."""
    third_party = [
        track for track in set(_all_music_tracks()) if "third_party/music/" in track
    ]
    assert third_party, "aucune piste tierce référencée : vérifier music.json"
    missing_credit = []
    for track in third_party:
        path = GAME / track
        if path.name not in _source_md_credits(path):
            missing_credit.append(track)
    assert not missing_credit, f"pistes non créditées dans SOURCE.md : {missing_credit}"


def test_culture_regions_are_valid_and_have_playlists() -> None:
    """Every `culture_regions` value has a matching, non-empty `campaign_<région>` playlist."""
    music = _music()
    regions = set(music.get("culture_regions", {}).values())
    assert regions, "culture_regions ne doit pas être vide (DA4 : au moins 5 régions)"
    assert regions <= CULTURE_REGIONS, (
        f"région(s) inconnue(s) : {regions - CULTURE_REGIONS}"
    )
    for region in regions:
        context = f"campaign_{region}"
        assert context in music["playlists"], f"playlist manquante : {context}"
        primary = music["playlists"][context]["primary"]
        assert len(primary) >= 2, f"{context} : au moins 2 pistes de campagne attendues"


def test_all_five_bible_regions_are_covered() -> None:
    """Bible DA § 9.

    France, Angleterre, Bourgogne/Flandre, Ibérie, Italie ont chacune leurs pistes de campagne.
    """
    playlists = _music()["playlists"]
    for region in CULTURE_REGIONS:
        context = f"campaign_{region}"
        assert context in playlists, f"région manquante : {region}"
        assert len(playlists[context]["primary"]) >= 2


def test_menu_and_court_playlists_exist() -> None:
    """Menu et cour ont une liste de lecture non vide (DA4)."""
    playlists = _music()["playlists"]
    for context in ("menu", "court"):
        assert context in playlists
        assert playlists[context]["primary"]


def test_kevin_macleod_is_fallback_only() -> None:
    """Les pistes Kevin MacLeod (repli, jamais supprimées) ne sont jamais en `primary`."""
    for context, playlist in _music()["playlists"].items():
        for track in playlist.get("primary", []):
            assert "kevin_macleod" not in track, (
                f"{context} : Kevin MacLeod doit être en repli (fallback), pas en primary"
            )


def test_battle_layers_reference_existing_files_with_credits() -> None:
    """Chaque couche et stinger de `battle_layers.json` existe et (hors sfx AU1) est crédité."""
    config = _battle_layers()
    paths = list(config["layers"].values()) + list(config.get("stingers", {}).values())
    for relative in paths:
        path = GAME / relative
        assert path.is_file(), f"fichier manquant : {relative}"
    # Les couches sous music/battle_layers/ (DA4) doivent être créditées ; les stingers et
    # melee_din réutilisent la banque AU1 (sfx/), créditée par ailleurs (CREDITS.md).
    for relative in config["layers"].values():
        if "music/battle_layers/" not in relative:
            continue
        path = GAME / relative
        assert path.name in _source_md_credits(path), (
            f"couche non créditée : {relative}"
        )


def test_battle_layers_states_cover_the_intensity_ladder() -> None:
    """Cinq états d'intensité, couches déclarées.

    approach/engagement/critical/victory/defeat sont tous définis pour chaque couche déclarée.
    """
    config = _battle_layers()
    states = config["states"]
    assert set(states) == {"approach", "engagement", "critical", "victory", "defeat"}
    layer_names = set(config["layers"])
    for state, preset in states.items():
        undeclared = set(preset["layers"]) - layer_names
        assert not undeclared, f"{state} : couche(s) non déclarée(s) : {undeclared}"
