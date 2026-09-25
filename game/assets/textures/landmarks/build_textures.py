"""Shared material atlas of the landmark cities (lot L3).

Usage: ``uv run --with pillow --with numpy python build_textures.py <downloads>``

``<downloads>`` holds the Poly Haven 1k files ``<id>_diff_1k.jpg`` and ``<id>_nor_gl_1k.jpg``
(https://api.polyhaven.com/files/<id>, CC0). Writes next to this script:

* ``landmark_albedo_array.jpg``: ``SLICE``-pixel layers stacked vertically (imported as a
  Texture2DArray, order = ``LAYERS``, same indices as ``MATERIAL_LAYER`` in
  ``tools/blender_scripts/landmark_city.py`` and ``landmark.gdshader``). Photographic layers are
  *detail* maps: each channel divided by its mean and halved, so that the shader multiplies the
  palette colour by ``2 × texel`` and every city keeps its own stone colour (blond Paris
  limestone, Caen stone, Avignon stone, Bruges brick) from the same few photographs;
* ``landmark_normal_array.jpg``: the matching OpenGL normal maps;
* the ``raw`` layers keep their own colour (stained glass) or hold masks (``aging``: R blotches,
  G rain streaks, B moss speckle, all tileable).
"""

import sys
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).parent
SLICE = 512

# (name, Poly Haven id or procedural key)
LAYERS = [
    ("ashlar", "medieval_blocks_03"),  # 0 dressed limestone (cathedrals, palaces)
    ("rubble", "castle_wall_varriation"),  # 1 rubble walls, towers, quays
    ("brick", "medieval_red_brick"),  # 2 Flemish brick
    ("slate", "roof_slates_02"),  # 3 slate roofs
    ("tile", "clay_roof_tiles_02"),  # 4 clay tiles
    ("tile_old", "roof_tiles_14"),  # 5 old weathered tiles
    ("plaster", "clay_plaster"),  # 6 lime plaster, render
    ("timber", "@timber"),  # 7 half-timbered facade (procedural on plaster and planks)
    ("wood", "old_planks_02"),  # 8 planks, doors, hoardings
    ("thatch", "reed_roof_04"),  # 9 thatch
    ("paving", "cobblestone_floor_08"),  # 10 streets, squares
    ("lead", "@lead"),  # 11 lead sheets with standing seams (spires, roofs)
    ("glass", "@glass"),  # 12 stained glass (raw colour)
    ("soil", "@soil"),  # 13 grass, gardens, dirt (noise)
    ("flat", "@flat"),  # 14 no texture (water, gold, dark openings)
    ("aging", "@aging"),  # 15 masks: blotches, streaks, moss (raw)
]

# Share of the photo's own hue kept per layer (clay tiles vary from red to yellow: luminance only).
HUE = {"tile": 0.0, "tile_old": 0.1, "brick": 0.35, "rubble": 0.3}


def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    """sRGB [0, 1] to linear."""
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c: np.ndarray) -> np.ndarray:
    """Linear [0, 1] to sRGB."""
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def load(path: Path) -> np.ndarray:
    """Image resized to the slice, float RGB in [0, 1]."""
    image = Image.open(path).convert("RGB").resize((SLICE, SLICE), Image.LANCZOS)
    return np.asarray(image, dtype=np.float64) / 255.0


def detail(srgb: np.ndarray, hue: float = 0.25) -> np.ndarray:
    """Detail map: linear channels divided by their mean, halved (mean 0.5), back to sRGB.

    ``hue`` is the share of the photo's own colour variation kept (the rest is its luminance).
    """
    lin = srgb_to_linear(srgb)
    mean = lin.reshape(-1, 3).mean(axis=0)
    luma = (lin * np.array([0.2126, 0.7152, 0.0722])).sum(axis=2, keepdims=True)
    lin = hue * lin / mean + (1.0 - hue) * luma / luma.mean()
    return linear_to_srgb(lin * 0.5)


def periodic_noise(rng: np.random.Generator, scale: float, aspect=(1.0, 1.0)) -> np.ndarray:
    """Tileable noise in [0, 1]: white noise low-passed in the Fourier domain."""
    white = rng.standard_normal((SLICE, SLICE))
    fy = np.fft.fftfreq(SLICE)[:, None] * aspect[1]
    fx = np.fft.fftfreq(SLICE)[None, :] * aspect[0]
    radius = np.sqrt(fx * fx + fy * fy) * SLICE
    spectrum = np.fft.fft2(white) * np.exp(-((radius / scale) ** 2))
    field = np.real(np.fft.ifft2(spectrum))
    field -= field.min()
    return field / max(field.max(), 1e-9)


def fbm(rng: np.random.Generator, base: float, octaves: int = 4) -> np.ndarray:
    """Sum of periodic noises, octaves halving in amplitude."""
    total = np.zeros((SLICE, SLICE))
    amp, norm = 1.0, 0.0
    for k in range(octaves):
        total += amp * periodic_noise(rng, base * 2**k)
        norm += amp
        amp *= 0.5
    return total / norm


def normal_from_height(height: np.ndarray, strength: float) -> np.ndarray:
    """OpenGL normal map (RGB [0, 1]) from a tileable height field."""
    dx = (np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1)) * strength
    dy = (np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)) * strength
    n = np.stack([-dx, dy, np.ones_like(height)], axis=2)
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    return n * 0.5 + 0.5


FLAT_NORMAL = np.full((SLICE, SLICE, 3), (0.5, 0.5, 1.0))


def timber(src: Path, rng: np.random.Generator):
    """Half-timbered facade: plaster panels in a frame of posts, rails and braces (1 tile = 4 m)."""
    plaster = detail(load(src / "clay_plaster_diff_1k.jpg")) * 1.25
    wood = detail(load(src / "old_planks_02_diff_1k.jpg")) * 0.32
    u = (np.arange(SLICE) + 0.5) / SLICE
    x, y = np.meshgrid(u, u)
    beam = 0.028
    mask = np.zeros((SLICE, SLICE), dtype=bool)
    for px in (0.0, 0.25, 0.5, 0.75, 1.0):
        mask |= np.abs(x - px) < beam
    for py in (0.0, 0.5, 1.0):
        mask |= np.abs(y - py) < beam * 1.2
    # Saint Andrew's crosses in every other bay, braces elsewhere.
    for bay in range(4):
        x0 = bay * 0.25
        for storey in range(2):
            y0 = storey * 0.5
            lx, ly = (x - x0) / 0.25, (y - y0) / 0.5
            inside = (lx >= 0) & (lx <= 1) & (ly >= 0) & (ly <= 1)
            d1 = np.abs(ly - lx) * 0.25 / np.sqrt(2)
            d2 = np.abs(ly - (1 - lx)) * 0.25 / np.sqrt(2)
            if (bay + storey) % 2 == 0:
                mask |= inside & ((d1 < beam * 0.7) | (d2 < beam * 0.7))
            else:
                mask |= inside & (d1 < beam * 0.7)
    albedo = np.where(mask[..., None], wood, plaster)
    height = np.where(mask, 1.0, 0.0) + fbm(rng, 40.0) * 0.3
    return np.clip(albedo, 0, 1), normal_from_height(height, 3.0)


def lead(src: Path, rng: np.random.Generator):
    """Lead sheets: dull grey with white oxidation, standing seams every 0.25 tile."""
    luma = (0.42 + 0.16 * fbm(rng, 60.0))[..., None]
    blotch = fbm(rng, 12.0)[..., None]
    albedo = luma * (0.85 + 0.6 * np.clip(blotch - 0.45, 0, 1))
    u = (np.arange(SLICE) + 0.5) / SLICE
    x, _ = np.meshgrid(u, u)
    seam = np.exp(-(((x * 4) % 1.0 - 0.5) ** 2) / 0.0015)
    albedo = albedo * (1.0 - 0.25 * seam[..., None]) + 0.08 * seam[..., None]
    height = seam + fbm(rng, 30.0) * 0.15
    return np.clip(np.repeat(albedo, 3, axis=2), 0, 1), normal_from_height(height, 4.0)


def glass(rng: np.random.Generator):
    """Stained glass (raw colour, dark): diamond panes of blue, red and green in lead cames."""
    u = (np.arange(SLICE) + 0.5) / SLICE
    x, y = np.meshgrid(u, u)
    n = 8
    a, b = (x + y) * n, (x - y) * n
    ia, ib = np.floor(a).astype(int), np.floor(b).astype(int)
    fa, fb = a - ia, b - ib
    came = (np.minimum(np.minimum(fa, 1 - fa), np.minimum(fb, 1 - fb))) < 0.06
    palette = np.array(
        [
            (0.10, 0.18, 0.55),
            (0.08, 0.13, 0.42),
            (0.55, 0.10, 0.08),
            (0.12, 0.35, 0.18),
            (0.55, 0.42, 0.12),
            (0.14, 0.22, 0.60),
        ]
    )
    pick = (ia * 7 + ib * 13) % len(palette)
    colour = palette[pick] * (0.75 + 0.5 * fbm(rng, 30.0)[..., None])
    albedo = np.where(came[..., None], 0.02, colour)
    height = np.where(came, 1.0, 0.0)
    return linear_to_srgb(albedo), normal_from_height(height, 2.0)


def soil(rng: np.random.Generator):
    """Ground detail (grass, gardens, dirt): soft mottling around a mean of 0.5."""
    field = 0.5 * fbm(rng, 20.0) + 0.5 * fbm(rng, 90.0)
    lin = 0.5 * (0.7 + 0.6 * field)
    albedo = linear_to_srgb(np.repeat(lin[..., None], 3, axis=2))
    return albedo, normal_from_height(field, 2.0)


def aging(rng: np.random.Generator):
    """Masks of the ageing: R blotches (dirt, lichen), G rain streaks (vertical), B moss speckle."""
    blotch = fbm(rng, 6.0, 5)
    streak = periodic_noise(rng, 60.0, aspect=(1.0, 14.0))
    streak = np.clip((streak - 0.35) * 1.8, 0, 1)
    moss = fbm(rng, 25.0, 4)
    return np.stack([blotch, streak, moss], axis=2), FLAT_NORMAL


def build(src: Path) -> None:
    """Assemble both arrays."""
    rng = np.random.default_rng(1337)
    albedos, normals = [], []
    for name, source in LAYERS:
        if source == "@timber":
            albedo, normal = timber(src, rng)
        elif source == "@lead":
            albedo, normal = lead(src, rng)
        elif source == "@glass":
            albedo, normal = glass(rng)
        elif source == "@soil":
            albedo, normal = soil(rng)
        elif source == "@flat":
            albedo, normal = np.full((SLICE, SLICE, 3), linear_to_srgb(np.array(0.5))), FLAT_NORMAL
        elif source == "@aging":
            albedo, normal = aging(rng)
        else:
            albedo = detail(load(src / f"{source}_diff_1k.jpg"), HUE.get(name, 0.25))
            normal = load(src / f"{source}_nor_gl_1k.jpg")
        albedos.append(albedo)
        normals.append(normal)
        print(f"layer {len(albedos) - 1:2d} {name}")
    for out, stack in (("landmark_albedo_array.jpg", albedos), ("landmark_normal_array.jpg", normals)):
        sheet = np.concatenate(stack, axis=0)
        Image.fromarray((np.clip(sheet, 0, 1) * 255 + 0.5).astype(np.uint8)).save(
            HERE / out, quality=90
        )
        print("wrote", out)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit("usage: build_textures.py <downloads>")
    build(Path(sys.argv[1]))
