"""Tests of the AU1 sound bank: data file, generated assets, licence check, DSP helpers."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator

from cent_ans_tools import audio_bank

REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "data"
AUDIO = REPO / "game" / "assets" / "audio"


def _bank() -> dict:
    return json.loads((DATA / "audio" / "sound_bank.json").read_text(encoding="utf-8"))


def test_sound_bank_matches_schema() -> None:
    """``data/audio/sound_bank.json`` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "sound_bank.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_bank()))
    assert not errors, [error.message for error in errors]


def test_every_bank_file_exists_and_is_generated() -> None:
    """Each referenced file exists and is produced by a clip of the pipeline."""
    bank = _bank()
    files = [f for entry in bank["events"].values() for f in entry.get("files", [])]
    files += [entry["file"] for entry in bank["beds"].values()]
    files += [entry["file"] for entry in bank["ambience"].values()]
    clip_names = {clip.name for clip in audio_bank.CLIPS}
    for name in files:
        assert (AUDIO / f"{name}.ogg").exists(), name
        assert name in clip_names, name


def test_layers_reference_known_events() -> None:
    """Composite events only layer events that have files."""
    events = _bank()["events"]
    for entry in events.values():
        for layer in entry.get("layers", []):
            assert events[layer].get("files"), layer


def test_every_clip_source_is_declared() -> None:
    """Clips only use declared Freesound sources, all credited in SOURCE.md."""
    source_md = (AUDIO / "SOURCE.md").read_text(encoding="utf-8")
    for clip in audio_bank.CLIPS:
        for sound_id in clip.sources:
            assert sound_id in audio_bank.SOURCES, (clip.name, sound_id)
            assert f"[{sound_id}]" in source_md, (clip.name, sound_id)


def test_bank_weight_under_budget() -> None:
    """The AU1 bank stays well under the 60 MB budget."""
    total = sum(
        p.stat().st_size
        for d in ("battle", "ambience")
        for p in (AUDIO / d).glob("*.ogg")
    )
    assert total < 20 * 1024 * 1024


def test_verify_licence_accepts_only_cc0() -> None:
    """Only a page linking the CC0 deed (and no other licence) is accepted."""
    cc0 = '<a href="https://creativecommons.org/publicdomain/zero/1.0/">'
    by = '<a href="https://creativecommons.org/licenses/by/4.0/">'
    by_nc = '<a href="http://creativecommons.org/licenses/by-nc/3.0/">'
    assert audio_bank.verify_licence(cc0)
    assert not audio_bank.verify_licence(by)
    assert not audio_bank.verify_licence(cc0 + by_nc)
    assert not audio_bank.verify_licence("<html>no licence</html>")


def test_parse_page() -> None:
    """Author, title (which may contain " by ") and HQ preview are read from a page."""
    page = (
        "<title>Freesound - Arrows fly by fast by someone</title>"
        '<a href="/people/someone/sounds/12345/">x</a>'
        "https://cdn.freesound.org/previews/12/12345_678-hq.mp3"
    )
    info = audio_bank.parse_page(12345, page)
    assert info.user == "someone"
    assert info.title == "Arrows fly by fast"
    assert info.preview.endswith("12345_678-hq.mp3")
    assert info.url == "https://freesound.org/people/someone/sounds/12345/"


def test_loopify_is_seamless() -> None:
    """The loop point joins the tail to the head without a jump."""
    t = np.arange(audio_bank.SAMPLE_RATE * 4) / audio_bank.SAMPLE_RATE
    x = (np.sin(2 * np.pi * 3.3 * t) * 0.5).astype(np.float32)
    loop = audio_bank.loopify(x, 1.0)
    assert len(loop) == len(x) - audio_bank.SAMPLE_RATE
    assert abs(float(loop[-1]) - float(loop[0])) < 0.01


def test_onsets_find_attacks_in_order() -> None:
    """Three clicks are found at their positions, in time order."""
    x = np.zeros(audio_bank.SAMPLE_RATE * 3, dtype=np.float32)
    for at in (0.5, 1.3, 2.2):
        start = int(at * audio_bank.SAMPLE_RATE)
        x[start : start + 2000] = np.hanning(4000)[2000:] * 0.9
    marks = audio_bank.onsets(x, 3, 0.3)
    assert [round(m / audio_bank.SAMPLE_RATE, 1) for m in marks] == [0.5, 1.3, 2.2]


def test_loudest_window_picks_the_busy_part() -> None:
    """The loudest window starts where the signal is loud."""
    x = np.zeros(audio_bank.SAMPLE_RATE * 10, dtype=np.float32)
    x[audio_bank.SAMPLE_RATE * 6 : audio_bank.SAMPLE_RATE * 8] = 0.5
    window = audio_bank.loudest_window(x, 2.0)
    assert np.mean(np.abs(window)) > 0.4
