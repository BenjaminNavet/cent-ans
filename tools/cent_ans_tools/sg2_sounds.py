"""SG2 procedural siege sounds: boiling oil poured from the machicolations.

Reproducible synthesis (numpy/scipy, fixed seeds, no external sample), run with::

    uv run --project tools --with soundfile python -m cent_ans_tools.sg2_sounds

Each ``battle/boiling_oil_<n>.ogg`` layers, over about three seconds:

1. the **pour**: a heavy liquid gush, low-passed noise with a slow flutter;
2. the **splash** on armour and earth, a short band-passed burst when the oil lands;
3. the **sizzle**: high-passed hiss with sparse crackles, decaying as the oil cools;
4. a few **bubbles** (short falling sine chirps) in the puddle.

Files are mono (3D sounds), peak-normalised to -1 dBFS and encoded Ogg Vorbis like the AU1
bank (``audio_bank.write_ogg``); ``data/audio/sound_bank.json`` references them as the
``boiling_oil`` event. ``SOURCE.md`` lists them in its SG2 section.
"""

from __future__ import annotations

import numpy as np
from scipy import signal

from cent_ans_tools.audio_bank import AUDIO_DIR, PEAK_DB, SAMPLE_RATE, peak_normalize, write_ogg

CLIP_NAMES = ["battle/boiling_oil_1", "battle/boiling_oil_2"]
DURATION_S = 3.2
LAND_S = 0.85  # the oil reaches the assailants' heads (pour from ~8 m)


def _envelope(n: int, attack: float, decay: float, start: float = 0.0) -> np.ndarray:
    t = np.arange(n) / SAMPLE_RATE - start
    env = np.where(t < 0.0, 0.0, 1.0 - np.exp(-np.maximum(t, 0.0) / max(attack, 1e-4)))
    return env * np.exp(-np.maximum(t, 0.0) / decay)


def _filtered(rng: np.random.Generator, n: int, kind: str, freq) -> np.ndarray:
    sos = signal.butter(4, freq, btype=kind, fs=SAMPLE_RATE, output="sos")
    return signal.sosfilt(sos, rng.standard_normal(n))


def boiling_oil(seed: int) -> np.ndarray:
    """One pot of boiling oil: pour, splash, sizzle and bubbles (mono float32)."""
    rng = np.random.default_rng(seed)
    n = int(DURATION_S * SAMPLE_RATE)
    t = np.arange(n) / SAMPLE_RATE
    flutter = 1.0 + 0.35 * np.sin(2 * np.pi * (7.0 + rng.uniform(-1, 1)) * t)
    pour = _filtered(rng, n, "lowpass", 900.0) * flutter
    pour *= np.clip(t / 0.08, 0.0, 1.0) * np.clip((LAND_S + 0.25 - t) / 0.3, 0.0, 1.0)
    splash = _filtered(rng, n, "bandpass", [300.0, 3200.0]) * _envelope(n, 0.01, 0.18, LAND_S)
    hiss = _filtered(rng, n, "highpass", 3500.0) * _envelope(n, 0.05, 1.1, LAND_S)
    crackle = np.zeros(n)
    count = int(rng.integers(90, 130))
    for _ in range(count):
        at = LAND_S + rng.exponential(0.7)
        i = int(at * SAMPLE_RATE)
        if i >= n - 200:
            continue
        width = int(rng.integers(20, 120))
        crackle[i : i + width] += rng.uniform(0.3, 1.0) * np.hanning(width) * rng.choice([-1.0, 1.0])
    crackle = signal.sosfilt(signal.butter(2, 1800.0, btype="highpass", fs=SAMPLE_RATE, output="sos"), crackle)
    bubbles = np.zeros(n)
    for _ in range(int(rng.integers(6, 11))):
        at = LAND_S + rng.uniform(0.1, 1.6)
        i = int(at * SAMPLE_RATE)
        length = int(0.06 * SAMPLE_RATE)
        if i + length >= n:
            continue
        tt = np.arange(length) / SAMPLE_RATE
        f0 = rng.uniform(350.0, 700.0)
        phase = 2 * np.pi * np.cumsum(f0 * (1.0 + 2.5 * tt / tt[-1])) / SAMPLE_RATE
        bubbles[i : i + length] += np.sin(phase) * np.exp(-tt / 0.02) * rng.uniform(0.2, 0.5)
    mix = 0.55 * pour + 0.9 * splash + 0.35 * hiss + 0.5 * crackle + 0.4 * bubbles
    fade = np.clip((DURATION_S - t) / 0.4, 0.0, 1.0)
    return peak_normalize((mix * fade).astype(np.float32), PEAK_DB)


def main() -> None:
    """Writes the SG2 clips into ``game/assets/audio/``."""
    for k, name in enumerate(CLIP_NAMES):
        write_ogg(AUDIO_DIR / f"{name}.ogg", boiling_oil(1356 + k * 17), 1)
        print("wrote", name)


if __name__ == "__main__":
    main()
