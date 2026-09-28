"""CC0 PBR textures of the campaign terrain (Poly Haven) and the sea normal map.

Layer identity (order, Poly Haven asset) lives in
``data/fx/campaign_terrain_textures.json`` (lot GA4, schema
``fx_campaign_terrain_textures.schema.json``); the order is the fixed index
contract of ``terrain.gdshader``.

Outputs in ``game/assets/textures/terrain/``:

``terrain_albedo_array.jpg`` / ``terrain_normal_array.jpg`` (lot GA4)
    Layers stacked vertically (imported by Godot as VRAM-compressed
    ``Texture2DArray``), ``albedo_size`` / ``normal_size`` pixels per layer.
    Normal array: R, G = OpenGL normal X, Y (``nor_gl``), B = roughness
    (``Rough``); the shader rebuilds Z.
``water_normal.png`` (lot GA4)
    Tileable procedural sea normal map (own work, CC0): sum of wave trains with
    integer wave vectors, so the tile wraps exactly.
``<layer>_albedo.jpg`` / ``<layer>_normal_rough.jpg`` (legacy 1k)
    Kept for the ``--no-ga4`` A/B path only.

The linear mean of each 2k albedo is written back into the data file
(``mean_linear``): the shader tints the detail around it.

API: ``https://api.polyhaven.com/files/<asset id>`` (free, CC0, no key).
"""

from __future__ import annotations

import io
import json
import re
from pathlib import Path

import httpx
import numpy as np
from PIL import Image

from cent_ans_tools.geo import download

REPO_DIR = download.TOOLS_DIR.parent
TEXTURE_DIR = REPO_DIR / "game" / "assets" / "textures" / "terrain"
SPEC_PATH = REPO_DIR / "data" / "fx" / "campaign_terrain_textures.json"
CACHE_DIR = download.RAW_DIR / "polyhaven"
API_URL = "https://api.polyhaven.com/files/{asset}"
LEGACY_RESOLUTION = "1k"
LEGACY_SIZE = 1024
JPEG_QUALITY = 92
ARRAY_JPEG_QUALITY = 90


def load_spec(path: Path = SPEC_PATH) -> dict:
    """Read the campaign terrain texture data file."""
    return json.loads(path.read_text(encoding="utf-8"))


# Ordre = index de couche dans les Texture2DArray du shader (terrain.gdshader).
LAYERS: dict[str, str] = {
    layer["id"]: layer["poly_haven_id"] for layer in load_spec()["layers"]
}


def pack_normal_rough(normal_rgb: np.ndarray, rough: np.ndarray) -> np.ndarray:
    """Pack normal X/Y and roughness into one ``uint8`` RGB image.

    Args:
        normal_rgb: ``(h, w, 3)`` OpenGL normal map (``uint8``).
        rough: ``(h, w)`` roughness (``uint8``).
    """
    packed = normal_rgb[..., :3].copy()
    packed[..., 2] = rough
    return packed


def srgb_to_linear(value: np.ndarray) -> np.ndarray:
    """Convert sRGB values in [0, 1] to linear."""
    return np.where(value <= 0.04045, value / 12.92, ((value + 0.055) / 1.055) ** 2.4)


def linear_mean(albedo: Image.Image) -> list[float]:
    """Linear mean colour of an sRGB albedo (Godot: last mipmap, then ``srgb_to_linear``)."""
    mean_srgb = np.asarray(albedo.convert("RGB"), dtype=np.float64).mean(axis=(0, 1))
    return [round(float(v), 4) for v in srgb_to_linear(mean_srgb / 255.0)]


def water_normal(size: int = 1024, seed: int = 1337) -> np.ndarray:
    """Tileable sea normal map (``uint8`` RGB, OpenGL convention).

    Sum of wave trains whose wave vectors are integer multiples of the tile
    frequency (exact wrap), amplitude falling with frequency, with a dominant
    wind direction; normals from the analytic gradient.
    """
    rng = np.random.default_rng(seed)
    coords = np.arange(size, dtype=np.float64) / size
    x, y = np.meshgrid(coords, coords)
    grad_x = np.zeros((size, size))
    grad_y = np.zeros((size, size))
    wind = np.array([0.8, 0.6])
    for _ in range(64):
        while True:
            k = rng.integers(-24, 25, size=2)
            magnitude = float(np.hypot(*k))
            if 2.0 <= magnitude <= 24.0:
                break
        alignment = abs(float(np.dot(k / magnitude, wind)))
        amplitude = (0.35 + 0.65 * alignment**2) / magnitude**1.6
        phase = rng.uniform(0.0, 2.0 * np.pi)
        arg = 2.0 * np.pi * (k[0] * x + k[1] * y) + phase
        derivative = -amplitude * 2.0 * np.pi * np.sin(arg)
        grad_x += derivative * k[0]
        grad_y += derivative * k[1]
    scale = 1.0 / max(float(np.abs(grad_x).max()), float(np.abs(grad_y).max()), 1e-9)
    nx, ny = -grad_x * scale, -grad_y * scale
    nz = np.ones_like(nx)
    length = np.sqrt(nx * nx + ny * ny + nz * nz)
    normal = np.stack([nx / length, ny / length, nz / length], axis=-1)
    return np.clip((normal * 0.5 + 0.5) * 255.0 + 0.5, 0, 255).astype(np.uint8)


def _fetch(client: httpx.Client, url: str) -> Image.Image:
    response = client.get(url, follow_redirects=True, timeout=120.0)
    response.raise_for_status()
    return Image.open(io.BytesIO(response.content))


def _cached(
    client: httpx.Client, files: dict, asset: str, key: str, res: str
) -> Image.Image:
    """Poly Haven map ``key`` at ``res``, cached under ``geo/raw/polyhaven``."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    target = CACHE_DIR / f"{asset}_{key}_{res}.jpg"
    if not target.exists():
        response = client.get(
            files[key][res]["jpg"]["url"], follow_redirects=True, timeout=120.0
        )
        response.raise_for_status()
        target.write_bytes(response.content)
    return Image.open(target)


def build_arrays(
    spec_path: Path = SPEC_PATH, texture_dir: Path = TEXTURE_DIR
) -> list[Path]:
    """Build the stacked 2k arrays and the sea normal map, write layer means back."""
    spec = load_spec(spec_path)
    albedo_size = int(spec["albedo_size"])
    normal_size = int(spec["normal_size"])
    layers = spec["layers"]
    albedo_sheet = Image.new("RGB", (albedo_size, albedo_size * len(layers)))
    normal_sheet = Image.new("RGB", (normal_size, normal_size * len(layers)))
    res = f"{max(albedo_size, normal_size) // 1024}k"
    with httpx.Client() as client:
        for index, layer in enumerate(layers):
            asset = layer["poly_haven_id"]
            response = client.get(API_URL.format(asset=asset), timeout=60.0)
            response.raise_for_status()
            files = response.json()
            albedo = _cached(client, files, asset, "Diffuse", res).convert("RGB")
            normal = _cached(client, files, asset, "nor_gl", res).convert("RGB")
            rough = _cached(client, files, asset, "Rough", res).convert("L")
            albedo = albedo.resize((albedo_size, albedo_size), Image.Resampling.LANCZOS)
            layer["mean_linear"] = linear_mean(albedo)
            albedo_sheet.paste(albedo, (0, index * albedo_size))
            normal = normal.resize((normal_size, normal_size), Image.Resampling.LANCZOS)
            rough = rough.resize((normal_size, normal_size), Image.Resampling.LANCZOS)
            packed = pack_normal_rough(np.asarray(normal), np.asarray(rough))
            normal_sheet.paste(
                Image.fromarray(packed, mode="RGB"), (0, index * normal_size)
            )
    albedo_path = texture_dir / "terrain_albedo_array.jpg"
    normal_path = texture_dir / "terrain_normal_array.jpg"
    albedo_sheet.save(albedo_path, quality=ARRAY_JPEG_QUALITY, subsampling=0)
    normal_sheet.save(normal_path, quality=ARRAY_JPEG_QUALITY, subsampling=0)
    water_path = texture_dir / "water_normal.png"
    water_size = int(spec["water"]["normal_size"])
    Image.fromarray(water_normal(water_size), mode="RGB").save(water_path, optimize=True)
    spec_path.write_text(format_spec(spec), encoding="utf-8")
    return [albedo_path, normal_path, water_path, spec_path]


def format_spec(spec: dict) -> str:
    """Data file text: two-space indent, one line per layer (as hand-written)."""
    head = {key: value for key, value in spec.items() if key != "layers"}
    lines = ["{"]
    for key in ("albedo_size", "normal_size", "tile_screen_px"):
        lines.append(f'  "{key}": {json.dumps(head.pop(key))},')
    lines.append('  "layers": [')
    layer_lines = [
        "    " + json.dumps(layer, ensure_ascii=False) for layer in spec["layers"]
    ]
    lines.append(",\n".join(layer_lines))
    lines.append("  ],")
    rest = json.dumps(head, indent=2, ensure_ascii=False)[1:-1].strip("\n")
    # Listes de nombres sur une ligne, comme le fichier écrit à la main.
    rest = re.sub(
        r"\[\s*([-0-9.e, \n]+?)\s*\]",
        lambda m: "[" + ", ".join(v.strip() for v in m.group(1).split(",")) + "]",
        rest,
    )
    lines.append(rest)
    lines.append("}")
    return "\n".join(lines) + "\n"


def build(force: bool = False, texture_dir: Path = TEXTURE_DIR) -> list[Path]:
    """Legacy 1k layers (``--no-ga4``, skipped if present unless ``force``) + GA4 arrays."""
    texture_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    size = (LEGACY_SIZE, LEGACY_SIZE)
    with httpx.Client() as client:
        for layer, asset in LAYERS.items():
            albedo_path = texture_dir / f"{layer}_albedo.jpg"
            packed_path = texture_dir / f"{layer}_normal_rough.jpg"
            if albedo_path.exists() and packed_path.exists() and not force:
                written += [albedo_path, packed_path]
                continue
            response = client.get(API_URL.format(asset=asset), timeout=60.0)
            response.raise_for_status()
            maps = response.json()
            urls = {
                key: maps[key][LEGACY_RESOLUTION]["jpg"]["url"]
                for key in ("Diffuse", "nor_gl", "Rough")
            }
            albedo = _fetch(client, urls["Diffuse"]).convert("RGB")
            normal = _fetch(client, urls["nor_gl"]).convert("RGB")
            rough = _fetch(client, urls["Rough"]).convert("L")
            albedo = albedo.resize(size, Image.Resampling.LANCZOS)
            normal = normal.resize(size, Image.Resampling.LANCZOS)
            rough = rough.resize(size, Image.Resampling.LANCZOS)
            albedo.save(albedo_path, quality=JPEG_QUALITY, subsampling=0)
            packed = pack_normal_rough(np.asarray(normal), np.asarray(rough))
            Image.fromarray(packed, mode="RGB").save(
                packed_path, quality=JPEG_QUALITY, subsampling=0
            )
            written += [albedo_path, packed_path]
    return written + build_arrays(texture_dir=texture_dir)
