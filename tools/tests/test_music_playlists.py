"""Validates data/audio/music.json (music playlists per context)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_music_matches_schema() -> None:
    """The playlist file matches its schema."""
    schema = _load("schemas/music.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_load("audio/music.json")))
    assert not errors, [error.message for error in errors]


def test_music_tracks_exist_and_are_credited() -> None:
    """Every track exists; third-party tracks appear in their folder's SOURCE.md."""
    for context, playlist in _load("audio/music.json")["playlists"].items():
        # DA4: {primary, fallback}; the fallback tier (MIDI renders, Kevin MacLeod) still varies.
        tracks = playlist["primary"] + playlist["fallback"]
        assert len(tracks) >= 3, f"{context}: playlist too short to vary"
        for track in tracks:
            path = ROOT / "game" / track
            assert path.is_file(), track
            if "third_party" in track:
                source = (path.parent / "SOURCE.md").read_text(encoding="utf-8")
                assert path.name in source, f"{track} missing from SOURCE.md"
