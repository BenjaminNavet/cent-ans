"""Height, normal, roughness and micro-detail maps from albedo luminance (T1b, T1e)."""

from __future__ import annotations

import numpy as np
from scipy.ndimage import gaussian_filter

from cent_ans_tools.texture_factory.color import LUMA
from cent_ans_tools.texture_factory.seamless import low_pass


def flatten_lighting(
    linear: np.ndarray, sigma: float, strength: float = 0.7
) -> np.ndarray:
    """Divide out large-scale luminance variation (wrap-around blur), keeping chroma."""
    luminance = linear @ LUMA
    low = low_pass(luminance, sigma, "wrap")
    ratio = (low.mean() / np.maximum(low, 1e-4)) ** strength
    return linear * ratio[..., np.newaxis]


def flatten_gradients(linear: np.ndarray, sigma: float) -> np.ndarray:
    """Remove large-scale colour gradients of a non-tiling image (vignetting, haze).

    Each channel is divided by its own mirrored-edge blur (times its mean), so the regions
    the seam cuts bring together share the same low-frequency colour and the cuts leave no
    visible band.
    """
    low = low_pass(linear, sigma, "reflect")
    means = linear.reshape(-1, linear.shape[-1]).mean(axis=0)
    return linear * (means / np.maximum(low, 1e-4))


def equalize_luminance(linear: np.ndarray, target: float) -> np.ndarray:
    """Scale linear RGB so its mean luminance equals ``target``."""
    mean = float((linear @ LUMA).mean())
    return np.clip(linear * (target / max(mean, 1e-4)), 0.0, 1.0)


def derive_normal_rough(
    linear: np.ndarray, strength: float, base_roughness: float
) -> np.ndarray:
    """Normal (R, G, OpenGL) + roughness (B) from the high-passed luminance, uint8."""
    luminance = linear @ LUMA
    height = luminance - gaussian_filter(luminance, 8.0, mode="wrap")
    height = gaussian_filter(height, 0.8, mode="wrap")
    scale = max(float(np.percentile(np.abs(height), 98)), 1e-5)
    height = np.clip(height / scale, -1.0, 1.0)
    dx = (np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1)) * 0.5
    dy = (np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)) * 0.5
    nx, ny = -dx * strength, dy * strength  # OpenGL: +Y up, image rows go down
    nz = np.ones_like(nx)
    norm = np.sqrt(nx * nx + ny * ny + nz * nz)
    rough = np.clip(base_roughness + 0.1 * (-height), 0.0, 1.0)
    packed = np.stack([nx / norm * 0.5 + 0.5, ny / norm * 0.5 + 0.5, rough], axis=-1)
    return np.clip(np.rint(packed * 255), 0, 255).astype(np.uint8)
