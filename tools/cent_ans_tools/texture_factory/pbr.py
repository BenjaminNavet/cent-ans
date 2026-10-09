"""Height, normal, roughness and micro-detail maps from albedo luminance (T1b, T1e)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter

from cent_ans_tools.texture_factory.color import LUMA, srgb_to_linear
from cent_ans_tools.texture_factory.seamless import low_pass, make_seamless


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


MICRO_STD = 0.12


def _micro_height(src_images: list[Path], size: int) -> np.ndarray:
    """Seamless height tile in [0, 1] (mean 0.5, std ~0.12) from high-passed luminance."""
    if not src_images:
        raise ValueError("aucune image source pour le micro-détail")
    accumulated = np.zeros((size, size))
    for path in src_images:
        image = Image.open(path).convert("RGB").resize((size, size), Image.LANCZOS)
        luminance = srgb_to_linear(np.asarray(image) / 255.0) @ LUMA
        detail = luminance - gaussian_filter(luminance, 6.0, mode="wrap")
        accumulated += detail / max(float(detail.std()), 1e-6)
    accumulated /= len(src_images)
    half_band = max(16, min(size // 8, size // 2 - 1))
    tile = make_seamless(accumulated[..., np.newaxis].repeat(3, axis=-1), half_band)[
        ..., 0
    ]
    tile = (tile - tile.mean()) / max(float(tile.std()), 1e-6)
    return np.clip(0.5 + MICRO_STD * tile, 0.0, 1.0)


def micro_detail(src_images: list[Path], dst: Path, size: int = 2048) -> Path:
    """Seamless grayscale fine-grain height tile (8-bit PNG) from the inputs' fine detail."""
    height = _micro_height(src_images, size)
    dst = Path(dst)
    dst.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(np.rint(height * 255).astype(np.uint8), "L").save(dst)
    return dst


def micro_detail_normal(
    src_images: list[Path], dst: Path, size: int = 2048, strength: float = 1.0
) -> Path:
    """Normal-map (RGB, OpenGL; blue holds roughness) variant of :func:`micro_detail`."""
    height = _micro_height(src_images, size)
    packed = derive_normal_rough(
        np.repeat(height[..., np.newaxis], 3, axis=-1), strength, 0.5
    )
    dst = Path(dst)
    dst.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(packed, "RGB").save(dst)
    return dst
