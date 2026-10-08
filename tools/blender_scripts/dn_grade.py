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


def _apply_gamma(rgb: np.ndarray, gamma: float, saturation_cap: float) -> np.ndarray:
    """Apply ``x ** (1 / gamma)`` then re-cap saturation (brightening desaturates nothing, darkening can raise S)."""
    out = rgb ** (1.0 / gamma) if gamma != 1.0 else rgb
    hsv = rgb_to_hsv(out)
    hsv[..., 1] = np.minimum(hsv[..., 1], saturation_cap)
    return hsv_to_rgb(hsv)


def auto_gamma(
    rgb: np.ndarray, luma_range: tuple[float, float], saturation_cap: float = 0.40
) -> float:
    """Gamma (>1 brightens) bringing mean luminance into the window; 1.0 when already inside.

    Bisects so the mean lands 10 % of the window width inside the nearest edge.
    """
    low, high = luma_range
    mean = float((rgb @ LUMA).mean())
    if low <= mean <= high or mean <= 0:
        return 1.0
    goal = low + 0.1 * (high - low) if mean < low else high - 0.1 * (high - low)
    lo_g, hi_g = 0.2, 6.0
    for _ in range(40):
        mid = (lo_g + hi_g) / 2
        if float((_apply_gamma(rgb, mid, saturation_cap) @ LUMA).mean()) < goal:
            lo_g = mid
        else:
            hi_g = mid
    return round((lo_g + hi_g) / 2, 4)


def grade_albedo(
    rgb: np.ndarray,
    saturation_cap: float = 0.40,
    luma_range: tuple[float, float] | None = None,
    gamma: float | None = None,
) -> tuple[np.ndarray, dict]:
    """Cap saturation at ``saturation_cap`` and fit mean luminance into ``luma_range``.

    ``gamma`` None = automatic (needs ``luma_range``), a number = forced. Saturation above
    the cap is clipped (hue and value kept) before and after gamma. Returns the graded pixels
    and a report (``before`` / ``after`` statistics and the gamma used).
    """
    before = albedo_stats(rgb)
    out = _apply_gamma(np.clip(rgb, 0.0, 1.0), 1.0, saturation_cap)
    if gamma is None:
        gamma = auto_gamma(out, luma_range, saturation_cap) if luma_range else 1.0
    out = _apply_gamma(out, gamma, saturation_cap)
    return out, {"before": before, "after": albedo_stats(out), "gamma": gamma}
