"""Procedural fire and smoke flipbooks (lot V3, A1-13).

Writes two 8x8 flipbook sheets (128 px frames, 1024 px sheets) used by the siege fire particles:

- `flame_flipbook.png`: a looping flame tongue. RGB = heat (0 cold edge, 1 white core), A = coverage.
  The shader maps heat to a black-body ramp; the 64 frames loop seamlessly (cross-faded noise).
- `smoke_flipbook.png`: a smoke puff over its life (frame 0 = young and dense, 63 = spread and
  thin). R = density, G = self-shadowed lighting from above (fake volumetric shading), A = density.

Deterministic (fixed seed), no dependency beyond numpy and Pillow.

Usage: uv run --project tools python -m cent_ans_tools.fire_flipbooks [output_dir]
"""

from __future__ import annotations

import sys
from pathlib import Path

import numpy as np
from PIL import Image

FRAME = 128
GRID = 8
FRAMES = GRID * GRID
DEFAULT_OUT = (
    Path(__file__).resolve().parents[2] / "game" / "assets" / "textures" / "fx"
)


class ValueNoise3D:
    """Smooth 3D value noise on a hashed lattice (vectorised)."""

    def __init__(self, seed: int, size: int = 256) -> None:
        """Builds the permutation table."""
        rng = np.random.default_rng(seed)
        self.size = size
        self.perm = np.concatenate([rng.permutation(size)] * 2)
        self.values = rng.random(size).astype(np.float32)

    def _hash(self, x: np.ndarray, y: np.ndarray, z: np.ndarray) -> np.ndarray:
        s = self.size - 1
        return self.values[self.perm[self.perm[self.perm[x & s] + (y & s)] + (z & s)]]

    def __call__(self, x: np.ndarray, y: np.ndarray, z: np.ndarray) -> np.ndarray:
        """Noise in [0, 1] at the given coordinates (same shapes)."""
        xi, yi, zi = (np.floor(v).astype(np.int64) for v in (x, y, z))
        xf, yf, zf = x - xi, y - yi, z - zi
        u, v, w = (f * f * (3.0 - 2.0 * f) for f in (xf, yf, zf))
        result = 0.0
        for dx in (0, 1):
            for dy in (0, 1):
                for dz in (0, 1):
                    weight = (
                        (u if dx else 1.0 - u)
                        * (v if dy else 1.0 - v)
                        * (w if dz else 1.0 - w)
                    )
                    result = result + weight * self._hash(xi + dx, yi + dy, zi + dz)
        return result

    def fbm(
        self,
        x: np.ndarray,
        y: np.ndarray,
        z: np.ndarray,
        octaves: int = 5,
        lacunarity: float = 2.0,
        gain: float = 0.5,
    ) -> np.ndarray:
        """Fractal sum normalised to about [0, 1]."""
        total = np.zeros_like(x)
        amplitude, frequency, norm = 1.0, 1.0, 0.0
        for octave in range(octaves):
            total += amplitude * self(
                x * frequency + octave * 17.3,
                y * frequency + octave * 5.1,
                z * frequency + octave * 3.7,
            )
            norm += amplitude
            amplitude *= gain
            frequency *= lacunarity
        return total / norm


def _smoothstep(edge0: float, edge1: float, x: np.ndarray) -> np.ndarray:
    t = np.clip((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def _grid() -> tuple[np.ndarray, np.ndarray]:
    coords = (np.arange(FRAME, dtype=np.float32) + 0.5) / FRAME
    x, y = np.meshgrid(coords, coords[::-1])  # y = 0 at the bottom of the frame
    return x, y


def flame_frame(noise: ValueNoise3D, phase: float) -> tuple[np.ndarray, np.ndarray]:
    """One looping flame frame: (heat, alpha) in [0, 1]. `phase` in [0, 1)."""
    x, y = _grid()

    def field(time: float) -> np.ndarray:
        rise = time * 6.0
        warp = noise.fbm(x * 3.0, y * 2.5 - rise, np.full_like(x, 1.3), octaves=3)
        sway = (warp - 0.5) * (0.12 + 0.35 * y)
        detail = noise.fbm(
            x * 8.0 + sway * 4.0, y * 4.0 - rise * 1.6, np.full_like(x, 7.7), octaves=4
        )
        return sway, detail

    # Seamless loop: cross-fade the field at t and t - 1.
    sway_a, detail_a = field(phase)
    sway_b, detail_b = field(phase - 1.0)
    blend = phase
    sway = sway_a * (1.0 - blend) + sway_b * blend
    detail = detail_a * (1.0 - blend) + detail_b * blend
    half_width = (
        0.3
        * np.power(np.clip(1.0 - y, 0.0, 1.0), 0.75)
        * _smoothstep(0.0, 0.12, y + 0.02)
    )
    distance = np.abs(x - 0.5 + sway)
    body = _smoothstep(half_width + 0.02, half_width * 0.25, distance)
    # Erosion by noise, stronger towards the tip: tongues detach and break up.
    erosion = (detail - 0.32) * (0.5 + 2.2 * y)
    mask = np.clip(body - np.maximum(erosion, 0.0) * 1.1, 0.0, 1.0)
    mask *= _smoothstep(1.0, 0.72, y)
    heat = np.clip(np.power(mask, 2.0) * (1.05 - 0.75 * y), 0.0, 1.0)
    alpha = _smoothstep(0.02, 0.35, mask)
    return heat, alpha


def smoke_frame(noise: ValueNoise3D, age: float) -> tuple[np.ndarray, np.ndarray]:
    """One smoke puff frame at `age` in [0, 1]: (density, lighting)."""
    x, y = _grid()
    cx, cy = x - 0.5, y - 0.5
    radius = 0.3 + 0.16 * np.sqrt(age)
    swirl = age * 1.5
    warp_x = noise.fbm(x * 3.0, y * 3.0, np.full_like(x, swirl), octaves=3) - 0.5
    warp_y = noise.fbm(x * 3.0 + 9.1, y * 3.0, np.full_like(x, swirl), octaves=3) - 0.5
    dx = cx + warp_x * 0.22
    dy = cy + warp_y * 0.22
    distance = np.sqrt(dx * dx + dy * dy)
    billow = noise.fbm(
        x * 5.0 + warp_x,
        y * 5.0 + warp_y,
        np.full_like(x, 3.0 + swirl * 0.7),
        octaves=5,
    )
    shape = _smoothstep(radius, radius * 0.35, distance) * (0.55 + 0.9 * billow)
    fade = (1.0 - age) ** 1.3
    density = np.clip(shape * (0.3 + 0.7 * fade), 0.0, 1.0)
    density *= _smoothstep(0.0, 0.25, shape)
    # Fake self-shadowing: density sampled towards the light (up and slightly left) darkens.
    shift = 6
    towards_light = np.roll(np.roll(density, shift, axis=0), shift // 2, axis=1)
    lighting = np.clip(1.0 - towards_light * 0.9 + (billow - 0.5) * 0.5, 0.0, 1.0)
    lighting = 0.25 + 0.75 * lighting
    return density, lighting


def _sheet(frames: list[np.ndarray]) -> np.ndarray:
    channels = frames[0].shape[-1]
    sheet = np.zeros((FRAME * GRID, FRAME * GRID, channels), dtype=np.float32)
    for index, frame in enumerate(frames):
        row, column = divmod(index, GRID)
        sheet[
            row * FRAME : (row + 1) * FRAME, column * FRAME : (column + 1) * FRAME
        ] = frame
    return sheet


def build(out_dir: Path) -> list[Path]:
    """Writes both sheets into `out_dir` and returns their paths."""
    out_dir.mkdir(parents=True, exist_ok=True)
    flame_noise = ValueNoise3D(1337)
    smoke_noise = ValueNoise3D(2002)
    flames = []
    smokes = []
    for index in range(FRAMES):
        heat, alpha = flame_frame(flame_noise, index / FRAMES)
        flames.append(np.stack([heat, heat, heat, alpha], axis=-1))
        density, lighting = smoke_frame(smoke_noise, index / (FRAMES - 1))
        smokes.append(np.stack([density, lighting, lighting, density], axis=-1))
    paths = []
    for name, frames in (
        ("flame_flipbook.png", flames),
        ("smoke_flipbook.png", smokes),
    ):
        sheet = (np.clip(_sheet(frames), 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)
        path = out_dir / name
        Image.fromarray(sheet, "RGBA").save(path, optimize=True)
        paths.append(path)
    return paths


if __name__ == "__main__":
    target = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    for written in build(target):
        print(written)
