"""Procedural sound effects and modal music (numpy only, deterministic).

Every clip is a pure function of its name (fixed random seeds), rendered as mono
float samples at 44.1 kHz, written as 16-bit WAV and converted to OGG Vorbis with
``ffmpeg`` when available (Godot reads both).

Synthesis building blocks:

- **Karplus-Strong** plucked string (lute): a noise burst circulates in a delay
  line of one period, averaged with its neighbour each pass (low-pass + decay).
- **Bowed string** (vièle): band-limited sawtooth with slow attack and vibrato.
- **Portative organ**: a few sine harmonics with a gentle tremolo.
- **Choir**: detuned sawtooth voices through vowel formant resonators.
- **Bell**: inharmonic partials with exponential decays.
- A small feedback-comb reverb glues the music together.

Music uses medieval church modes (Dorian, Mixolydian) over a drone (tonic + fifth).
"""

from __future__ import annotations

import shutil
import subprocess
import wave
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy import signal

REPO_DIR = Path(__file__).resolve().parents[2]
AUDIO_DIR = REPO_DIR / "game" / "assets" / "audio"
SFX_DIR = AUDIO_DIR / "sfx"
MUSIC_DIR = AUDIO_DIR / "music"

SAMPLE_RATE = 44100

# Semitone offsets of the church modes from their final.
MODES = {
    "dorian": [0, 2, 3, 5, 7, 9, 10],
    "mixolydian": [0, 2, 4, 5, 7, 9, 10],
}


# --- Primitives ---------------------------------------------------------------------


def _time(duration: float) -> np.ndarray:
    return np.arange(int(duration * SAMPLE_RATE)) / SAMPLE_RATE


def midi_to_hz(note: float) -> float:
    """Convert a MIDI note number to a frequency (A4 = 69 = 440 Hz)."""
    return 440.0 * 2.0 ** ((note - 69) / 12)


def envelope(
    length: int, attack: float, release: float, sustain: float = 1.0
) -> np.ndarray:
    """Linear attack, flat sustain, linear release (seconds), ``length`` samples."""
    env = np.full(length, sustain)
    attack_n = min(length, int(attack * SAMPLE_RATE))
    release_n = min(length - attack_n, int(release * SAMPLE_RATE))
    if attack_n:
        env[:attack_n] = np.linspace(0, sustain, attack_n)
    if release_n:
        env[length - release_n :] *= np.linspace(1, 0, release_n)
    return env


def lowpass(samples: np.ndarray, cutoff: float, order: int = 2) -> np.ndarray:
    """Butterworth low-pass filter."""
    sos = signal.butter(order, cutoff, "low", fs=SAMPLE_RATE, output="sos")
    return signal.sosfilt(sos, samples)


def bandpass(samples: np.ndarray, low: float, high: float) -> np.ndarray:
    """Butterworth band-pass filter."""
    sos = signal.butter(2, [low, high], "band", fs=SAMPLE_RATE, output="sos")
    return signal.sosfilt(sos, samples)


def sawtooth(
    freq: np.ndarray | float, duration: float, harmonics: int = 12
) -> np.ndarray:
    """Additive (band-limited) sawtooth; ``freq`` may be a per-sample array."""
    t = _time(duration)
    freq = np.broadcast_to(np.asarray(freq, dtype=float), t.shape)
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    out = np.zeros_like(t)
    for k in range(1, harmonics + 1):
        mask = (freq * k) < SAMPLE_RATE / 2.2
        out += mask * np.sin(k * phase) / k
    return out * 0.6


def karplus_strong(
    freq: float, duration: float, rng: np.random.Generator, damping: float = 0.996
) -> np.ndarray:
    """Plucked string: a noise burst recirculated through an averaging delay line."""
    period = max(2, int(SAMPLE_RATE / freq))
    total = int(duration * SAMPLE_RATE)
    out = np.zeros(total + period)
    out[:period] = rng.uniform(-1, 1, period)
    out[:period] = lowpass(out[:period], min(8000, freq * 12))
    position = period
    while position < len(out):
        end = min(position + period, len(out))
        previous = out[position - period : end - period]
        shifted = out[position - period + 1 : end - period + 1]
        if len(shifted) < len(previous):
            shifted = np.append(shifted, shifted[-1:])
        out[position:end] = damping * 0.5 * (previous + shifted)
        position = end
    return out[period:] * 0.8


def bowed(freq: float, duration: float, vibrato: float = 5.0) -> np.ndarray:
    """Vièle-like bowed string: sawtooth with vibrato, slow attack, body filter."""
    t = _time(duration)
    depth = np.clip(t / 0.4, 0, 1) * 0.006
    tone = sawtooth(freq * (1 + depth * np.sin(2 * np.pi * vibrato * t)), duration, 16)
    tone = lowpass(tone, min(3500, freq * 8))
    return tone * envelope(len(t), 0.12, 0.15)


def organ(freq: float, duration: float) -> np.ndarray:
    """Portative organ pipe: sine harmonics with a slight tremolo and breath noise."""
    t = _time(duration)
    tone = sum(
        weight * np.sin(2 * np.pi * freq * k * t)
        for k, weight in ((1, 1.0), (2, 0.5), (3, 0.25), (4, 0.12))
    )
    tone *= 1 + 0.03 * np.sin(2 * np.pi * 4.5 * t)
    return tone * envelope(len(t), 0.05, 0.08) * 0.5


def bell(freq: float, duration: float) -> np.ndarray:
    """Church bell: inharmonic partials, the higher ones decaying faster."""
    t = _time(duration)
    partials = [
        (0.5, 1.0, 0.6),
        (1.0, 0.8, 1.2),
        (1.19, 0.5, 1.6),
        (1.56, 0.4, 2.0),
        (2.0, 0.35, 2.4),
        (2.74, 0.25, 3.5),
        (3.76, 0.15, 5.0),
    ]
    out = sum(
        amp * np.sin(2 * np.pi * freq * ratio * t) * np.exp(-t * decay)
        for ratio, amp, decay in partials
    )
    return out / 2.5


def noise(duration: float, rng: np.random.Generator) -> np.ndarray:
    """White noise in -1..1."""
    return rng.uniform(-1, 1, int(duration * SAMPLE_RATE))


def reverb(samples: np.ndarray, mix: float = 0.25, decay: float = 1.0) -> np.ndarray:
    """Parallel feedback combs (Schroeder style) for a small stone-hall ambience."""
    wet = np.zeros(len(samples))
    for delay_ms, gain in ((29.7, 0.8), (37.1, 0.78), (41.1, 0.76), (43.7, 0.74)):
        delay = int(delay_ms * SAMPLE_RATE / 1000)
        denominator = np.zeros(delay + 1)
        denominator[0] = 1
        denominator[delay] = -gain * decay
        wet += signal.lfilter([1.0], denominator, samples)
    wet = lowpass(wet / 4, 5000)
    return (1 - mix) * samples + mix * wet


def mix_at(
    target: np.ndarray, clip: np.ndarray, start: float, gain: float = 1.0
) -> None:
    """Add ``clip`` into ``target`` at ``start`` seconds (clipped to the end)."""
    offset = max(0, int(start * SAMPLE_RATE))
    if offset >= len(target):
        return
    end = min(len(target), offset + len(clip))
    target[offset:end] += gain * clip[: end - offset]


def normalize(samples: np.ndarray, peak: float = 0.89) -> np.ndarray:
    """Scale to ``peak`` and fade the first/last 5 ms to avoid clicks."""
    samples = np.asarray(samples, dtype=float)
    top = np.max(np.abs(samples)) or 1.0
    samples = samples / top * peak
    fade = min(len(samples) // 2, int(0.005 * SAMPLE_RATE))
    if fade:
        samples[:fade] *= np.linspace(0, 1, fade)
        samples[-fade:] *= np.linspace(1, 0, fade)
    return samples


# --- Sound effects ------------------------------------------------------------------


def sfx_ui_click(rng: np.random.Generator) -> np.ndarray:
    """Short wooden tick."""
    t = _time(0.06)
    tone = np.sin(2 * np.pi * 1800 * t) * np.exp(-t * 90)
    return tone + 0.3 * bandpass(noise(0.06, rng), 2000, 6000) * np.exp(-t * 150)


def sfx_page_turn(rng: np.random.Generator) -> np.ndarray:
    """Parchment swish: band-passed noise swelling then crackling."""
    duration = 0.55
    t = _time(duration)
    swish = (
        bandpass(noise(duration, rng), 1500, 7000) * np.sin(np.pi * t / duration) ** 2
    )
    crackle = (rng.random(len(t)) > 0.997) * rng.uniform(-1, 1, len(t))
    return swish + 0.6 * lowpass(crackle, 5000)


def sfx_turn_bell(rng: np.random.Generator) -> np.ndarray:
    """Two strokes of a church bell."""
    out = np.zeros(int(3.2 * SAMPLE_RATE))
    mix_at(out, bell(midi_to_hz(57), 3.2), 0.0)
    mix_at(out, bell(midi_to_hz(52), 2.6), 0.6, 0.8)
    return reverb(out, 0.3)


def _brass(freq: float, duration: float) -> np.ndarray:
    t = _time(duration)
    brightness = np.clip(t / 0.08, 0, 1)
    tone = sawtooth(freq, duration, 20)
    tone = lowpass(tone, 900 + 2500 * float(np.mean(brightness)))
    return tone * envelope(len(t), 0.04, 0.1)


def sfx_fanfare(rng: np.random.Generator) -> np.ndarray:
    """Victory fanfare: trumpet arpeggio ending on a held chord."""
    out = np.zeros(int(2.6 * SAMPLE_RATE))
    notes = [
        (60, 0.0, 0.18),
        (60, 0.2, 0.18),
        (64, 0.4, 0.18),
        (67, 0.6, 0.35),
        (64, 1.0, 0.18),
        (67, 1.2, 1.3),
    ]
    for note, start, length in notes:
        mix_at(out, _brass(midi_to_hz(note), length), start)
    mix_at(out, _brass(midi_to_hz(72), 1.3), 1.2, 0.6)
    mix_at(out, _brass(midi_to_hz(48), 1.3), 1.2, 0.5)
    return reverb(out, 0.3)


def _drum_hit(
    rng: np.random.Generator, pitch: float = 90, length: float = 0.35
) -> np.ndarray:
    t = _time(length)
    freq = pitch * (1 + 1.5 * np.exp(-t * 30))
    body = np.sin(2 * np.pi * np.cumsum(freq) / SAMPLE_RATE) * np.exp(-t * 9)
    skin = lowpass(noise(length, rng), 2500) * np.exp(-t * 35)
    return body + 0.4 * skin


def sfx_march_drum(rng: np.random.Generator) -> np.ndarray:
    """Marching tabor: long-short-short pattern, twice."""
    out = np.zeros(int(2.2 * SAMPLE_RATE))
    for bar in range(2):
        for offset, gain in ((0.0, 1.0), (0.4, 0.6), (0.6, 0.6)):
            mix_at(out, _drum_hit(rng), bar * 1.0 + offset, gain)
    return out


def _metal_hit(rng: np.random.Generator, freq: float) -> np.ndarray:
    t = _time(0.6)
    ring = sum(
        np.sin(2 * np.pi * freq * ratio * t) * np.exp(-t * (8 + 4 * ratio))
        for ratio in (1.0, 2.76, 5.4, 8.93)
    )
    scrape = bandpass(noise(0.6, rng), 3000, 9000) * np.exp(-t * 40)
    return 0.4 * ring + scrape


def sfx_sword_clash(rng: np.random.Generator) -> np.ndarray:
    """Two steel blades meeting, then a third glancing blow."""
    out = np.zeros(int(1.2 * SAMPLE_RATE))
    mix_at(out, _metal_hit(rng, 1250), 0.0)
    mix_at(out, _metal_hit(rng, 1480), 0.22, 0.8)
    mix_at(out, _metal_hit(rng, 1100), 0.5, 0.5)
    return out


def sfx_arrow_volley(rng: np.random.Generator) -> np.ndarray:
    """Dozens of arrows whistling past, then thuds."""
    out = np.zeros(int(2.4 * SAMPLE_RATE))
    for _ in range(40):
        length = rng.uniform(0.25, 0.5)
        t = _time(length)
        center = rng.uniform(2500, 5000)
        whoosh = (
            bandpass(noise(length, rng), center * 0.7, center * 1.3)
            * np.sin(np.pi * t / length) ** 3
        )
        mix_at(out, whoosh, rng.uniform(0.0, 0.9), rng.uniform(0.2, 0.5))
    for _ in range(18):
        mix_at(
            out,
            _drum_hit(rng, rng.uniform(140, 220), 0.12),
            rng.uniform(1.0, 1.9),
            rng.uniform(0.2, 0.45),
        )
    return out


def sfx_gallop(rng: np.random.Generator) -> np.ndarray:
    """Horse gallop: three-beat hoof pattern on soft ground."""
    out = np.zeros(int(2.4 * SAMPLE_RATE))
    for stride in range(6):
        base = stride * 0.38
        for offset in (0.0, 0.08, 0.16):
            hoof = lowpass(_drum_hit(rng, rng.uniform(70, 95), 0.12), 1500)
            mix_at(
                out,
                hoof,
                base + offset + rng.uniform(-0.01, 0.01),
                rng.uniform(0.6, 1.0),
            )
    return out * np.linspace(0.6, 1.0, len(out))


def sfx_war_horn(rng: np.random.Generator) -> np.ndarray:
    """Low war horn with a rising attack glide."""
    duration = 2.4
    t = _time(duration)
    freq = (
        midi_to_hz(43)
        * (1 - 0.06 * np.exp(-t * 6))
        * (1 + 0.004 * np.sin(2 * np.pi * 5 * t))
    )
    tone = lowpass(sawtooth(freq, duration, 24), 1200)
    tone *= envelope(len(t), 0.25, 0.6)
    return reverb(tone, 0.35)


VOWEL_AH = ((700, 1.0), (1150, 0.5), (2800, 0.25))


def _choir_voice(freq: float, duration: float, rng: np.random.Generator) -> np.ndarray:
    t = _time(duration)
    detune = (
        1
        + rng.uniform(-0.004, 0.004)
        + 0.003 * np.sin(2 * np.pi * rng.uniform(4, 6) * t)
    )
    source = sawtooth(freq * detune, duration, 30) + 0.05 * noise(duration, rng)
    voice = sum(
        gain * bandpass(source, formant * 0.85, formant * 1.15)
        for formant, gain in VOWEL_AH
    )
    return voice * envelope(len(t), 0.35, 0.8)


def sfx_choir(rng: np.random.Generator) -> np.ndarray:
    """Short plainchant chord (open fifth then octave), several voices."""
    out = np.zeros(int(3.2 * SAMPLE_RATE))
    for note in (50, 57, 62):
        for _ in range(3):
            mix_at(
                out, _choir_voice(midi_to_hz(note), 3.0, rng), rng.uniform(0, 0.05), 0.4
            )
    return reverb(out, 0.4)


SFX: dict[str, Callable[[np.random.Generator], np.ndarray]] = {
    "ui_click": sfx_ui_click,
    "page_turn": sfx_page_turn,
    "turn_bell": sfx_turn_bell,
    "fanfare": sfx_fanfare,
    "march_drum": sfx_march_drum,
    "sword_clash": sfx_sword_clash,
    "arrow_volley": sfx_arrow_volley,
    "gallop": sfx_gallop,
    "war_horn": sfx_war_horn,
    "choir": sfx_choir,
}


# --- Music --------------------------------------------------------------------------


@dataclass(frozen=True)
class Piece:
    """Parameters of one procedural modal piece."""

    name: str
    final: int  # MIDI note of the mode's final (tonic)
    mode: str
    tempo: float  # beats per minute
    duration: float  # seconds
    melody: str  # "lute" | "viele" | "organ"
    drone: str  # "organ" | "viele"
    drum: bool
    seed: int


PIECES: dict[str, Piece] = {
    "campaign": Piece("campaign", 62, "dorian", 72, 80, "lute", "organ", False, 1337),
    "war": Piece("war", 57, "dorian", 96, 72, "viele", "viele", True, 1346),
    "court": Piece("court", 55, "mixolydian", 84, 76, "lute", "organ", False, 1364),
}


def _phrase(rng: np.random.Generator, beats: int) -> list[tuple[int, float]]:
    """Random walk over scale degrees, ending on the final: [(degree, beats)]."""
    notes: list[tuple[int, float]] = []
    degree = int(rng.choice([0, 2, 4]))
    remaining = beats
    while remaining > 0:
        length = float(
            rng.choice([0.5, 1.0, 1.0, 1.5, 2.0], p=[0.3, 0.35, 0.15, 0.1, 0.1])
        )
        length = min(length, remaining)
        notes.append((degree, length))
        remaining -= length
        degree = int(np.clip(degree + rng.choice([-2, -1, -1, 1, 1, 2, 3]), -2, 9))
    notes[-1] = (0, notes[-1][1])
    return notes


def _degree_to_midi(final: int, mode: str, degree: int) -> int:
    scale = MODES[mode]
    octave, index = divmod(degree, len(scale))
    return final + 12 * octave + scale[index]


def _voice(
    kind: str, freq: float, duration: float, rng: np.random.Generator
) -> np.ndarray:
    if kind == "lute":
        return karplus_strong(freq, duration + 0.6, rng)
    if kind == "viele":
        return bowed(freq, duration)
    return organ(freq, duration)


def render_music(name: str, duration: float | None = None) -> np.ndarray:
    """Render one piece (``campaign``, ``war``, ``court``); ``duration`` overrides length."""
    piece = PIECES[name]
    rng = np.random.default_rng(piece.seed)
    total = duration or piece.duration
    beat = 60.0 / piece.tempo
    out = np.zeros(int((total + 2) * SAMPLE_RATE))

    # Drone: final + fifth, re-articulated every 8 beats.
    drone_length = 8 * beat
    position = 0.0
    while position < total:
        for interval, gain in ((0, 0.18), (7, 0.12)):
            freq = midi_to_hz(piece.final - 12 + interval)
            mix_at(
                out, _voice(piece.drone, freq, drone_length + 0.2, rng), position, gain
            )
        position += drone_length

    # Melody: phrases of 8 beats (A A' B A'' form from four generated phrases).
    phrases = [_phrase(rng, 8) for _ in range(4)]
    form = [0, 1, 2, 0, 3, 1, 2, 3]
    position = beat * 2
    index = 0
    while position < total - 2 * beat:
        phrase = phrases[form[index % len(form)]]
        octave_shift = 12 if index % 4 == 2 else 0
        for degree, length in phrase:
            note = _degree_to_midi(piece.final, piece.mode, degree) + octave_shift
            note_duration = length * beat
            gain = 0.35 if piece.melody == "lute" else 0.25
            mix_at(
                out,
                _voice(piece.melody, midi_to_hz(note), note_duration * 0.95, rng),
                position,
                gain,
            )
            # Lute: add a lower-fifth counterpoint on strong beats.
            if piece.melody == "lute" and length >= 1.0:
                mix_at(
                    out,
                    karplus_strong(midi_to_hz(note - 7), note_duration + 0.4, rng),
                    position,
                    0.15,
                )
            position += note_duration
        position += beat  # breath between phrases
        index += 1

    if piece.drum:
        position = 0.0
        while position < total:
            for offset, gain in (
                (0.0, 0.5),
                (1.0, 0.25),
                (1.5, 0.25),
                (2.0, 0.4),
                (3.0, 0.25),
            ):
                mix_at(out, _drum_hit(rng, 85, 0.4), position + offset * beat, gain)
            position += 4 * beat

    out = reverb(out[: int(total * SAMPLE_RATE)], 0.3)
    fade = int(1.5 * SAMPLE_RATE)
    out[:fade] *= np.linspace(0, 1, fade)
    out[-fade:] *= np.linspace(1, 0, fade)
    return normalize(out, 0.8)


# --- Output -------------------------------------------------------------------------


def render_sfx(name: str) -> np.ndarray:
    """Render one sound effect by name (deterministic)."""
    seed = sum(ord(char) for char in name)
    return normalize(SFX[name](np.random.default_rng(seed)))


def to_pcm16(samples: np.ndarray) -> bytes:
    """Convert float samples in -1..1 to little-endian 16-bit PCM bytes."""
    return (np.clip(samples, -1, 1) * 32767).astype("<i2").tobytes()


def write_wav(path: Path, samples: np.ndarray) -> Path:
    """Write mono 16-bit WAV."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(SAMPLE_RATE)
        handle.writeframes(to_pcm16(samples))
    return path


def ogg_encoder() -> str | None:
    """Path to ``ffmpeg`` if it can encode Vorbis, else None."""
    ffmpeg = shutil.which("ffmpeg")
    if ffmpeg is None:
        return None
    probe = subprocess.run(
        [ffmpeg, "-hide_banner", "-encoders"],
        capture_output=True,
        text=True,
        check=False,
    )
    return ffmpeg if "vorbis" in probe.stdout else None


def write_clip(out_dir: Path, name: str, samples: np.ndarray, bitrate: str) -> Path:
    """Write ``<name>.ogg`` (ffmpeg Vorbis) or fall back to ``<name>.wav``."""
    wav_path = write_wav(out_dir / f"{name}.wav", samples)
    encoder = ogg_encoder()
    if encoder is None:
        return wav_path
    ogg_path = out_dir / f"{name}.ogg"
    # ffmpeg's native Vorbis encoder is flagged experimental; libvorbis is used if present.
    codec = (
        "libvorbis"
        if "libvorbis"
        in subprocess.run(
            [encoder, "-hide_banner", "-encoders"],
            capture_output=True,
            text=True,
            check=False,
        ).stdout
        else "vorbis"
    )
    subprocess.run(
        [
            encoder,
            "-y",
            "-loglevel",
            "error",
            "-i",
            str(wav_path),
            "-c:a",
            codec,
            "-strict",
            "-2",
            "-ac",
            "2" if codec == "vorbis" else "1",
            "-b:a",
            bitrate,
            "-map_metadata",
            "-1",
            "-fflags",
            "+bitexact",
            "-flags:a",
            "+bitexact",
            str(ogg_path),
        ],
        check=True,
    )
    wav_path.unlink()
    return ogg_path


def build(out_dir: Path = AUDIO_DIR, music: bool = True) -> list[Path]:
    """Render every sound effect (and the three music pieces) into ``out_dir``."""
    written = [
        write_clip(out_dir / "sfx", name, render_sfx(name), "96k") for name in SFX
    ]
    if music:
        written += [
            write_clip(out_dir / "music", name, render_music(name), "112k")
            for name in PIECES
        ]
    return written
