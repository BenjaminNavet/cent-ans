"""DA7a tests: France and Italy play real (non-MIDI) Ars nova recordings, all freely licensed."""

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
GAME = REPO / "game"
MUSIC_JSON = REPO / "data" / "audio" / "music.json"

# Tracks realised from MIDI files (DA4): never counted as a real recording.
MIDI_RENDERS = {
    "machaut_douce_dame_jolie.mp3",
    "machaut_riches_damour.mp3",
    "solage_fumeux_fume.mp3",
    "landini_ecco_la_primavera.mp3",
    "landini_si_dolce_non_sono.mp3",
    "binchois_triste_plaisir.mp3",
    "binchois_dueil_angoisseux.mp3",
}

# Project-made procedural music (AU1/DA4), not third-party: no external licence to check.
OWN_MUSIC_DIR = "assets/audio/music/"

ALLOWED_LICENCE = re.compile(
    r"\b(Public domain|Domaine public|CC0|CC BY(?:-SA)? \d\.\d)\b", re.IGNORECASE
)
FORBIDDEN_LICENCE = re.compile(
    r"\bCC BY[\w-]*-(?:NC|ND)\b|\bNonCommercial\b|\bNoDerivs?\b"
)


def _playlists() -> dict:
    return json.loads(MUSIC_JSON.read_text(encoding="utf-8"))["playlists"]


def _all_tracks() -> set[str]:
    tracks: set[str] = set()
    for playlist in _playlists().values():
        tracks.update(playlist.get("primary", []))
        tracks.update(playlist.get("fallback", []))
    return tracks


def _licence_text(track: str) -> str:
    """The licence stated for `track`: its `SOURCE.md` table row, else the folder licence."""
    path = GAME / track
    source = (path.parent / "SOURCE.md").read_text(encoding="utf-8")
    for line in source.splitlines():
        is_row = f"`{path.name}`" in line and line.startswith("|")
        if is_row and (ALLOWED_LICENCE.search(line) or FORBIDDEN_LICENCE.search(line)):
            return line
    # Single-licence folders (Kevin MacLeod) state it once in a "**Licence** :" line.
    folder = [line for line in source.splitlines() if line.startswith("- **Licence**")]
    return "\n".join(folder)


def _is_real_recording(track: str) -> bool:
    return (
        "third_party/music/" in track
        and track.rsplit("/", 1)[-1] not in MIDI_RENDERS
        and "kevin_macleod" not in track
    )


def test_france_and_italy_play_a_real_ars_nova_recording() -> None:
    """DA7a: campaign_france and campaign_italy still list a real DA7a performance.

    ADR 0166 moved the medieval pieces to `fallback` (calm lute and viol pieces lead the
    `primary` lists) without dropping them: they must stay listed in either list.
    """
    playlists = _playlists()
    for context in ("campaign_france", "campaign_italy"):
        listed = playlists[context]["primary"] + playlists[context].get("fallback", [])
        ars_nova = [t for t in listed if "third_party/music/ars_nova/" in t]
        assert ars_nova, (
            f"{context} : aucun enregistrement réel d'Ars nova (DA7a) listé"
        )
        assert all(_is_real_recording(t) for t in ars_nova)


def test_ars_nova_tracks_are_not_midi_renders() -> None:
    """No DA7a track is a MIDI realisation, and the DA4 MIDI renders stay fallback-only."""
    for context, playlist in _playlists().items():
        for track in playlist["primary"]:
            assert track.rsplit("/", 1)[-1] not in MIDI_RENDERS, (context, track)
    ars_nova_source = (
        GAME / "assets" / "third_party" / "music" / "ars_nova" / "SOURCE.md"
    ).read_text(encoding="utf-8")
    assert (
        "MIDI"
        not in ars_nova_source.split("## Licences des sources")[0].split("| Fichier |")[
            1
        ]
    ), "une piste DA7a est décrite comme MIDI"


def test_every_track_has_an_allowed_licence() -> None:
    """Every third-party track states an allowed licence (PD, CC0, CC BY, CC BY-SA; no NC/ND)."""
    problems = []
    for track in sorted(_all_tracks()):
        if track.startswith(OWN_MUSIC_DIR):
            continue
        assert "third_party/" in track, f"piste d'origine inconnue : {track}"
        text = _licence_text(track)
        if not ALLOWED_LICENCE.search(text) or FORBIDDEN_LICENCE.search(text):
            problems.append(f"{track}: {text.strip()[:120]!r}")
    assert not problems, "licence absente ou refusée :\n" + "\n".join(problems)


def test_forbidden_licence_pattern_rejects_nc_and_nd() -> None:
    """Sanity check of the licence filter itself."""
    assert FORBIDDEN_LICENCE.search("| x | CC BY-NC-SA 3.0 |")
    assert FORBIDDEN_LICENCE.search("| x | CC BY-ND 4.0 |")
    assert not FORBIDDEN_LICENCE.search("| x | CC BY-SA 4.0 |")
    assert ALLOWED_LICENCE.search("| x | Public domain |")
