"""Colour helpers shared by the texture factory stages (T1b)."""

from __future__ import annotations

import numpy as np

# Rec. 709 luma weights (linear RGB).
LUMA = np.array([0.2126, 0.7152, 0.0722])


def srgb_to_linear(values: np.ndarray) -> np.ndarray:
    """SRGB (0..1) to linear light."""
    return np.where(
        values <= 0.04045, values / 12.92, ((values + 0.055) / 1.055) ** 2.4
    )


def linear_to_srgb(values: np.ndarray) -> np.ndarray:
    """Linear light to sRGB (0..1), clipped."""
    values = np.clip(values, 0.0, 1.0)
    return np.where(
        values <= 0.0031308, values * 12.92, 1.055 * values ** (1 / 2.4) - 0.055
    )
