"""Procedural tileable water detail textures (lot DN nature, extends RC5 / ADR 0141).

Writes ``<id>_normal.png`` (tangent-space normal, +Z up, OpenGL green) and ``<id>_albedo.png``
into ``game/assets/textures/water/`` for the ids of ``data/fx/water_detail.json``. No third-party
source: the height field is spectral noise (inverse FFT of a shaped random spectrum), hence
periodic by construction (no seam, no blending). Run::

    uv run --project tools python -m cent_ans_tools.water_procedural [--out DIR] [--size 512]

Channels read by ``shaders/water_detail.gdshaderinc``:

* normal: ripples elongated along texture x (the flow axis of the river shaders: x = downstream);
* albedo RGB: mid-grey tint relief (the shader divides by the mean, so only the variation is
  kept), cooler in troughs, warmer on crests;
* albedo A: foam streak mask (sparse crests, stretched along the flow), used for bank foam.

Deterministic (fixed seed per id). Parameters live in ``MATERIALS`` below: shape of the spectrum
only (stretch, slope, bump), never game rules.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from PIL import Image
from cent_ans_tools.paths import REPO_DIR as REPO

DEFAULT_OUT = REPO / "game" / "assets" / "textures" / "water"

# stretch: elongation along x (flow); slope: spectral falloff (higher = smoother);
# bump: normal strength; foam: fraction of the tile covered by foam streaks.
MATERIALS: dict[str, dict[str, float]] = {
    "river_large": {
        "stretch": 5.0,
        "slope": 2.3,
        "bump": 1.0,
        "foam": 0.10,
        "seed": 11,
    },
    "river_small": {
        "stretch": 3.2,
        "slope": 1.9,
        "bump": 1.0,
        "foam": 0.16,
        "seed": 12,
    },
    "sea": {"stretch": 1.4, "slope": 2.2, "bump": 0.9, "foam": 0.05, "seed": 13},
    "ocean": {"stretch": 1.2, "slope": 2.6, "bump": 0.7, "foam": 0.04, "seed": 14},
}


def spectral_field(size: int, stretch: float, slope: float, seed: int) -> np.ndarray:
    """Periodic random field in [-1, 1], features elongated ``stretch`` times along x."""
    rng = np.random.default_rng(seed)
    kx = np.fft.fftfreq(size)[None, :] * size
    ky = np.fft.fftfreq(size)[:, None] * size
    radius = np.sqrt((kx * stretch) ** 2 + ky**2)
    radius[0, 0] = 1.0
    amplitude = radius ** (-slope / 2.0) * (radius < size * 0.45)
    amplitude[0, 0] = 0.0
    phase = np.fft.fft2(rng.standard_normal((size, size)))
    field = np.fft.ifft2(phase * amplitude).real
    field -= field.mean()
    return field / max(float(np.abs(field).max()), 1e-9)


def periodic_gradient(field: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Central differences with wrap-around (keeps the tile periodic)."""
    gx = (np.roll(field, -1, axis=1) - np.roll(field, 1, axis=1)) * 0.5
    gy = (np.roll(field, -1, axis=0) - np.roll(field, 1, axis=0)) * 0.5
    return gx, gy


def build(params: dict[str, float], size: int) -> tuple[np.ndarray, np.ndarray]:
    """Return (normal RGB uint8, albedo RGBA uint8) for one material."""
    seed = int(params["seed"])
    height = spectral_field(size, params["stretch"], params["slope"], seed)
    fine = spectral_field(
        size, params["stretch"] * 0.6, params["slope"] - 0.6, seed + 100
    )
    surface = height + 0.35 * fine
    gx, gy = periodic_gradient(surface * size / 64.0)
    nx, ny = (
        -gx * params["bump"],
        gy * params["bump"],
    )  # +Y green = OpenGL (see inc comment)
    normal = np.stack([nx, ny, np.ones_like(nx)], axis=-1)
    normal /= np.linalg.norm(normal, axis=-1, keepdims=True)
    normal_rgb = np.round((normal * 0.5 + 0.5) * 255).astype(np.uint8)

    relief = surface / max(float(np.abs(surface).max()), 1e-9)
    tint = 0.5 + 0.18 * relief
    albedo = np.stack([tint * 1.02, tint, tint * 0.97], axis=-1)
    streaks = spectral_field(size, params["stretch"] * 1.8, 1.7, seed + 200)
    threshold = np.quantile(streaks, 1.0 - params["foam"])
    foam = np.clip((streaks - threshold) / 0.12, 0.0, 1.0)
    foam *= np.clip(0.6 + 0.8 * (height - height.min()) / np.ptp(height), 0.0, 1.0)
    rgba = np.concatenate([albedo, foam[..., None]], axis=-1)
    return normal_rgb, np.round(np.clip(rgba, 0, 1) * 255).astype(np.uint8)


def main() -> None:
    """Write the textures of every material."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--size", type=int, default=512)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    for name, params in MATERIALS.items():
        normal, albedo = build(params, args.size)
        Image.fromarray(normal, "RGB").save(args.out / f"{name}_normal.png")
        Image.fromarray(albedo, "RGBA").save(args.out / f"{name}_albedo.png")
        print(f"{name}: {args.size}px")


if __name__ == "__main__":
    main()
