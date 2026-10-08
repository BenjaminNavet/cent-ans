"""Albedo grading to the world palette (bible 3.3): saturation cap and mean-luminance window.

Pure numpy, no Blender dependency, so it is unit-tested (``tools/tests/test_dn_ingest.py``) and
imported by ``dn_ingest.py`` inside Blender. Pixels are float RGB arrays in [0, 1], shape (..., 3).
"""

from __future__ import annotations

import numpy as np

LUMA = np.array([0.2126, 0.7152, 0.0722])


def rgb_to_hsv(rgb: np.ndarray) -> np.ndarray:
    """Vectorised RGB -> HSV (all channels in [0, 1])."""
    maxc = rgb.max(axis=-1)
    minc = rgb.min(axis=-1)
    delta = maxc - minc
    safe = np.where(delta == 0, 1.0, delta)
    saturation = np.where(maxc == 0, 0.0, delta / np.where(maxc == 0, 1.0, maxc))
    red, green, blue = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    hue = np.where(
        maxc == red,
        ((green - blue) / safe) % 6,
        np.where(maxc == green, (blue - red) / safe + 2, (red - green) / safe + 4),
    )
    hue = np.where(delta == 0, 0.0, hue / 6.0)
    return np.stack([hue, saturation, maxc], axis=-1)


def hsv_to_rgb(hsv: np.ndarray) -> np.ndarray:
    """Vectorised HSV -> RGB."""
    hue, saturation, value = hsv[..., 0], hsv[..., 1], hsv[..., 2]
    sector = hue * 6.0

    def channel(offset: float) -> np.ndarray:
        k = (offset + sector) % 6.0
        return value - value * saturation * np.clip(np.minimum(k, 4.0 - k), 0.0, 1.0)

    return np.stack([channel(5.0), channel(3.0), channel(1.0)], axis=-1)


def albedo_stats(rgb: np.ndarray) -> dict[str, float]:
    """Mean luminance, mean saturation and 95th percentile of saturation."""
    saturation = rgb_to_hsv(rgb)[..., 1]
    return {
        "mean_luma": round(float((rgb @ LUMA).mean()), 4),
        "mean_s": round(float(saturation.mean()), 4),
        "p95_s": round(float(np.percentile(saturation, 95)), 4),
    }


def grade_albedo(
    rgb: np.ndarray,
    saturation_cap: float = 0.40,
    luma_range: tuple[float, float] | None = None,
    gamma: float = 1.0,
) -> tuple[np.ndarray, dict]:
    """Cap saturation at ``saturation_cap`` and pull mean luminance into ``luma_range``.

    Returns the graded pixels and a report (``before`` / ``after`` statistics and the
    gain applied). Saturation above the cap is clipped (hue and value kept); the luminance
    window is reached by a single multiplicative gain, never beyond what keeps values <= 1.
    """
    before = albedo_stats(rgb)
    out = np.clip(rgb, 0.0, 1.0)
    if gamma != 1.0:
        out = out ** (1.0 / gamma)
    hsv = rgb_to_hsv(out)
    hsv[..., 1] = np.minimum(hsv[..., 1], saturation_cap)
    out = hsv_to_rgb(hsv)
    gain = 1.0
    if luma_range is not None:
        mean = float((out @ LUMA).mean())
        low, high = luma_range
        if mean > 0 and not low <= mean <= high:
            gain = (low if mean < low else high) / mean
            headroom = 1.0 / max(float(out.max()), 1e-6)
            gain = min(gain, headroom)
            out = np.clip(out * gain, 0.0, 1.0)
            mean = float((out @ LUMA).mean())
            if (
                mean < low and 0 < mean < 1
            ):  # gain was limited by headroom: lift mid-tones
                out = out ** (np.log(low) / np.log(mean))
    return out, {
        "before": before,
        "after": albedo_stats(out),
        "gain": round(gain, 4),
        "gamma": gamma,
    }
