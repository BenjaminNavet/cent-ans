"""Validates data/audio/music.json (music playlists per context)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_music_tracks_exist_and_are_credited() -> None:
    """Every track exists; third-party tracks appear in their folder's SOURCE.md."""
    for context, playlist in _load("audio/music.json")["playlists"].items():
        # DA4: {primary, fallback}; the fallback tier (MIDI renders, Kevin MacLeod) still varies.
        # RX audio (ADR 0247): `battle` has Kevin MacLeod as primary and no fallback tier.
        tracks = playlist["primary"] + playlist.get("fallback", [])
        assert len(tracks) >= 3, f"{context}: playlist too short to vary"
        for track in tracks:
            path = ROOT / "game" / track
            assert path.is_file(), track
            if "third_party" in track:
                source = (path.parent / "SOURCE.md").read_text(encoding="utf-8")
                assert path.name in source, f"{track} missing from SOURCE.md"


def test_every_third_party_track_has_in_game_credit() -> None:
    """Chaque piste tierce jouée figure dans data/audio/credits.json (écran Crédits, CC BY)."""
    referenced = {
        track
        for playlist in _load("audio/music.json")["playlists"].values()
        for track in playlist["primary"] + playlist.get("fallback", [])
        if "third_party" in track
    }
    credited = {entry["file"] for entry in _load("audio/credits.json")["tracks"]}
    assert referenced - credited == set(), "pistes jouées sans crédit en jeu"
    assert credited - referenced == set(), "crédits pour des pistes non jouées"


def test_kevin_macleod_credits_carry_cc_by_attribution() -> None:
    """Kevin MacLeod (CC BY 4.0) : chaque piste a son titre, l'auteur et le texte de licence."""
    macleod = [
        entry
        for entry in _load("audio/credits.json")["tracks"]
        if "kevin_macleod" in entry["file"]
    ]
    assert len(macleod) == 17
    for entry in macleod:
        assert entry["creators"] == "Kevin MacLeod (incompetech.com)"
        assert entry["licence"] == "CC BY 4.0"
        assert entry["licence_url"] == "https://creativecommons.org/licenses/by/4.0/"
