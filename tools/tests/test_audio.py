"""Tests for the procedural audio generator."""

import hashlib
import wave
from pathlib import Path

import numpy as np
import pytest

from cent_ans_tools import audio


def _digest(samples: np.ndarray) -> str:
    return hashlib.sha256(audio.to_pcm16(samples)).hexdigest()


@pytest.mark.parametrize("name", sorted(audio.SFX))
def test_sfx_deterministic_and_bounded(name: str) -> None:
    """Each effect renders identically twice, is short, non-silent and unclipped."""
    first = audio.render_sfx(name)
    assert _digest(first) == _digest(audio.render_sfx(name))
    assert 0.03 < len(first) / audio.SAMPLE_RATE < 4.0
    assert np.max(np.abs(first)) <= 0.9
    assert np.sqrt(np.mean(first**2)) > 0.01


@pytest.mark.parametrize("name", sorted(audio.PIECES))
def test_music_deterministic(name: str) -> None:
    """A shortened render of each piece is deterministic and audible."""
    first = audio.render_music(name, duration=6)
    assert _digest(first) == _digest(audio.render_music(name, duration=6))
    assert len(first) == 6 * audio.SAMPLE_RATE
    assert np.sqrt(np.mean(first**2)) > 0.02


def test_music_lengths_follow_spec() -> None:
    """Full pieces last between 60 and 90 seconds, in Dorian or Mixolydian."""
    for piece in audio.PIECES.values():
        assert 60 <= piece.duration <= 90
        assert piece.mode in ("dorian", "mixolydian")


def test_wav_fallback(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Without an OGG encoder, clips are written as mono 16-bit WAV."""
    monkeypatch.setattr(audio, "ogg_encoder", lambda: None)
    path = audio.write_clip(tmp_path, "ui_click", audio.render_sfx("ui_click"), "96k")
    assert path.suffix == ".wav"
    with wave.open(str(path)) as handle:
        assert handle.getnchannels() == 1
        assert handle.getsampwidth() == 2
        assert handle.getframerate() == audio.SAMPLE_RATE
