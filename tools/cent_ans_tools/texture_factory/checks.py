"""Automatic quality checks: seam error, luminance and saturation ranges, card scores (T1b)."""

from __future__ import annotations

import numpy as np

from cent_ans_tools.texture_factory.color import LUMA


def seam_ratio(image: np.ndarray) -> float:
    """Wrap seam jump relative to the typical jump between neighbouring pixels.

    ~1 means the seam is as smooth as the texture interior; a hard seam scores several.
    """
    data = image.astype(np.float64)
    interior = np.concatenate(
        [np.abs(np.diff(data, axis=0)).ravel(), np.abs(np.diff(data, axis=1)).ravel()]
    ).mean()
    wrap = np.concatenate(
        [np.abs(data[0] - data[-1]).ravel(), np.abs(data[:, 0] - data[:, -1]).ravel()]
    ).mean()
    return float(wrap / max(interior, 1e-6))


def blotch_score(image: np.ndarray, blocks: int = 4) -> float:
    """Coefficient of variation of block-mean luminances (a dominant blotch scores high)."""
    luminance = image.astype(np.float64)
    if luminance.ndim == 3:
        luminance = luminance[..., :3] @ LUMA
    size = luminance.shape[0] // blocks
    means = (
        luminance[: size * blocks, : size * blocks]
        .reshape(blocks, size, blocks, size)
        .mean(axis=(1, 3))
    )
    return float(means.std() / max(means.mean(), 1e-6))


def _mean_in_range(value: float, low: float, high: float) -> bool:
    return bool(low <= value <= high)


def luminance_in_range(image: np.ndarray, low: float, high: float) -> bool:
    """True if the mean luma of an RGB image (uint8 or 0..1 float) lies in [low, high]."""
    data = _unit_range(image)
    luminance = data[..., :3] @ LUMA if data.ndim == 3 else data
    return _mean_in_range(float(luminance.mean()), low, high)


def saturation_in_range(image: np.ndarray, low: float, high: float) -> bool:
    """True if the mean HSV saturation of an RGB image lies in [low, high]."""
    data = _unit_range(image)[..., :3]
    peak = data.max(axis=-1)
    saturation = (peak - data.min(axis=-1)) / np.maximum(peak, 1e-6)
    return _mean_in_range(float(saturation.mean()), low, high)


def _unit_range(image: np.ndarray) -> np.ndarray:
    data = image.astype(np.float64)
    return data / 255.0 if image.dtype == np.uint8 else data
