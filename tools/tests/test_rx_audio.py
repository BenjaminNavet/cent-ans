"""Tests des données du lot RX audio (ADR 0247) : gains, listes de lecture, langues de répliques."""

import json
from pathlib import Path

import pytest

from cent_ans_tools import audio_mastering, barks_languages

REPO = Path(__file__).resolve().parents[2]
MUSIC = json.loads((REPO / "data" / "audio" / "music.json").read_text(encoding="utf-8"))
BANK = json.loads(
    (REPO / "data" / "audio" / "sound_bank.json").read_text(encoding="utf-8")
)
BARKS = json.loads((REPO / "data" / "voice" / "barks.json").read_text(encoding="utf-8"))
GAME = REPO / "game"


def _tracks(context: str) -> set[str]:
    playlist = MUSIC["playlists"][context]
    return set(playlist.get("primary", [])) | set(playlist.get("fallback", []))


def test_every_present_track_has_a_bounded_gain() -> None:
    """Chaque morceau présent sur disque a un gain, dans les bornes de l'outil."""
    gains = MUSIC["track_gain_db"]
    for track in audio_mastering.playlist_tracks(MUSIC):
        if (GAME / track).exists():
            assert track in gains, track
            assert (
                audio_mastering.MAX_CUT_DB
                <= gains[track]
                <= audio_mastering.MAX_BOOST_DB
            )
    assert gains.keys() <= set(audio_mastering.playlist_tracks(MUSIC))
    assert MUSIC["crossfade_seconds"] > 0
    assert MUSIC["fallback_every"] > 0


def test_menu_war_battle_share_no_piece() -> None:
    """Une même pièce ne sert pas à la fois au menu, à la guerre et à la bataille."""
    menu, war, battle = _tracks("menu"), _tracks("war"), _tracks("battle")
    assert not menu & war
    assert not menu & battle
    assert not war & battle
    assert not any(
        "agincourt_carol" in t
        for p in MUSIC["playlists"].values()
        for t in p["primary"]
    )


def test_gain_for_respects_peak_headroom_and_bounds() -> None:
    """Un morceau très faible n'est pas remonté au-delà de ses crêtes ni de +9 dB."""
    assert audio_mastering.gain_for(-30.0, -10.0, -16.0) == 9.0
    assert audio_mastering.gain_for(-20.0, -2.0, -16.0) == 1.0
    assert audio_mastering.gain_for(-10.0, -1.0, -16.0) == -6.0
    assert audio_mastering.gain_for(-2.0, 0.0, -16.0) == -12.0


def test_variant_gains_trim_only_the_extremes() -> None:
    """Dans la bande ± 3 dB : rien ; au-delà : retour à la bande, sans écrêter."""
    levels = {
        "a": (-20.0, -6.0),
        "b": (-21.0, -6.0),
        "c": (-30.0, -20.0),
        "d": (-12.0, -3.0),
    }
    gains = audio_mastering.variant_gains(levels)
    assert "a" not in gains
    assert gains["c"] > 0
    assert gains["d"] < 0


def test_bank_variant_gains_target_known_files() -> None:
    """`file_gain_db` ne vise que des variantes de l'événement, dans ± 12 dB."""
    graded = 0
    for name, entry in BANK["events"].items():
        for file, gain in entry.get("file_gain_db", {}).items():
            graded += 1
            assert file in entry["files"], (name, file)
            assert -12 <= gain <= 12
    assert graded >= 5


def test_faction_language_covers_the_voiced_cultures() -> None:
    """Toute faction dont la culture a une langue proche en reçoit une, existante."""
    by_faction = BARKS["faction_language"]
    for faction, culture in barks_languages.faction_cultures().items():
        if culture in barks_languages.CULTURE_LANGUAGE:
            assert faction in by_faction, faction
    assert set(by_faction.values()) <= set(BARKS["languages"])
    assert by_faction["fac_castile"] == "oc"
    assert by_faction["fac_france"] == "fr"
    assert len(barks_languages.faction_cultures()) == 177


@pytest.mark.parametrize("culture", sorted(barks_languages.CULTURE_LANGUAGE))
def test_culture_language_is_a_known_language(culture: str) -> None:
    """La langue associée à chaque culture existe dans `barks.json`."""
    assert barks_languages.CULTURE_LANGUAGE[culture] in BARKS["languages"]
