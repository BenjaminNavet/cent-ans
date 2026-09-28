"""GA: AI-generated tileable materials and their derived maps.

Pipeline: prompt (``data/art/materials.yaml``) -> 1024x1024 image via
:func:`cent_ans_tools.openrouter.generate_image` (budget-guarded) -> tileable
(half offset + seam blend) -> derived maps (height from high-passed luminance,
OpenGL normal, roughness) -> contact sheet for review.
"""

from __future__ import annotations

from decimal import Decimal
from pathlib import Path
from typing import Any

import numpy as np
import yaml
from PIL import Image, ImageDraw, ImageFont
from scipy.ndimage import gaussian_filter

from cent_ans_tools import budget

ROOT = Path(__file__).resolve().parents[2]
MATERIALS_PATH = ROOT / "data" / "art" / "materials.yaml"

# Ledger section every GA spend must land in (``docs/budget.md``).
GA_SECTION_PREFIX = "Assets générés GA"
# Per-lot ceiling (spec: GA1 <= 4 $), checked against the ledger rows of the lot.
LOT_CAPS = {"GA1": Decimal("4.00")}
# Rec. 709 luma weights.
_LUMA = np.array([0.2126, 0.7152, 0.0722], dtype=np.float64)
# Largest review sheet kept in the repository.
SHEET_MAX_BYTES = 1_000_000


# --- data -----------------------------------------------------------------------


def load_materials(path: Path = MATERIALS_PATH) -> dict[str, Any]:
    """Return the parsed ``materials.yaml`` document."""
    return yaml.safe_load(Path(path).read_text(encoding="utf-8"))


def material_entry(material_id: str, path: Path = MATERIALS_PATH) -> dict[str, Any]:
    """Return one material entry of ``materials.yaml`` by id."""
    for entry in load_materials(path)["materials"]:
        if entry["id"] == material_id:
            return entry
    raise KeyError(f"Matière inconnue dans {path} : {material_id}")


# --- image processing ------------------------------------------------------------


def _seam_weight(length: int, blend_width: int) -> np.ndarray:
    """Weight 1 on the seam of a half-rolled axis, falling to 0 at ``blend_width``."""
    seam = length // 2 - 0.5  # between rolled indices length//2 - 1 and length//2
    distance = np.abs(np.arange(length, dtype=np.float64) - seam)
    ramp = np.clip(1.0 - distance / blend_width, 0.0, 1.0)
    return ramp * ramp * (3.0 - 2.0 * ramp)  # smoothstep


def make_tileable(image: np.ndarray, blend_width: int) -> np.ndarray:
    """Return a seamlessly tileable copy of ``image`` (H x W x C, uint8).

    The image is rolled by half on both axes, so its borders become pairs of pixels
    that were neighbours in the source (hence wrap seamlessly), while the original
    borders meet on a central cross. That cross is cross-faded, over ``blend_width``
    pixels, with copies rolled on one axis only (continuous across the seam they
    cover, still wrapping on the other axis) and with the unrolled source at the
    centre of the cross.
    """
    if image.ndim not in (2, 3):
        raise ValueError(f"Image H×W ou H×W×C attendue, forme {image.shape}")
    height, width = image.shape[:2]
    if not 1 <= blend_width < min(height, width) // 2:
        raise ValueError(f"blend_width hors bornes : {blend_width}")
    source = image.astype(np.float64)
    half_y, half_x = height // 2, width // 2
    rolled_xy = np.roll(source, (half_y, half_x), axis=(0, 1))
    rolled_y = np.roll(source, half_y, axis=0)  # continuous across the vertical seam
    rolled_x = np.roll(source, half_x, axis=1)  # continuous across the horizontal seam
    weight_x = _seam_weight(width, blend_width)[np.newaxis, :]
    weight_y = _seam_weight(height, blend_width)[:, np.newaxis]
    if source.ndim == 3:
        weight_x = weight_x[..., np.newaxis]
        weight_y = weight_y[..., np.newaxis]
    blended = (
        rolled_xy * (1 - weight_x) * (1 - weight_y)
        + rolled_y * weight_x * (1 - weight_y)
        + rolled_x * (1 - weight_x) * weight_y
        + source * weight_x * weight_y
    )
    return np.clip(np.rint(blended), 0, 255).astype(np.uint8)


def _luminance(albedo: np.ndarray) -> np.ndarray:
    """Luminance in 0..1 of an H x W (x C) uint8 image."""
    pixels = albedo.astype(np.float64) / 255.0
    if pixels.ndim == 2:
        return pixels
    if pixels.shape[2] == 1:
        return pixels[..., 0]
    return pixels[..., :3] @ _LUMA


def derive_maps(
    albedo: np.ndarray,
    *,
    height_strength: float = 1.0,
    roughness_bias: float = 0.0,
) -> dict[str, np.ndarray]:
    """Derive ``height``, ``normal`` (OpenGL) and ``roughness`` maps from an albedo.

    - ``height`` (H x W, uint8): luminance high-passed by a wrapped Gaussian blur
      (removes lighting gradients, stays tileable), centred on 128.
    - ``normal`` (H x W x 3, uint8): tangent-space normal, OpenGL convention (green = Y+,
      up in the image), from wrapped central differences of the height.
    - ``roughness`` (H x W, uint8): high and matte by default, rougher in cavities and
      dark areas, shifted by ``roughness_bias`` (-1..1).
    """
    luminance = _luminance(albedo)
    size = min(luminance.shape)
    denoised = gaussian_filter(luminance, sigma=0.7, mode="wrap")
    low_pass = gaussian_filter(denoised, sigma=max(size / 32.0, 2.0), mode="wrap")
    detail = denoised - low_pass
    spread = float(np.percentile(np.abs(detail), 98)) or 1.0
    height = np.clip(0.5 + 0.5 * detail / spread, 0.0, 1.0)

    # Slopes in height units per pixel, scaled so a full-range step over ~1/64 of the
    # tile tilts the normal by about 45 degrees at strength 1.
    scale = height_strength * size / 64.0
    d_dx = (np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1)) * 0.5 * scale
    d_drow = (np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)) * 0.5 * scale
    # Image rows grow downwards while OpenGL's V (green) grows upwards: dh/dv = -dh/drow.
    normal = np.stack([-d_dx, d_drow, np.ones_like(height)], axis=-1)
    normal /= np.linalg.norm(normal, axis=-1, keepdims=True)

    base_roughness = 0.78 + roughness_bias
    roughness = base_roughness + 0.25 * (0.5 - height) + 0.10 * (0.5 - luminance)
    roughness = np.clip(roughness, 0.0, 1.0)

    def to_u8(values: np.ndarray) -> np.ndarray:
        return np.clip(np.rint(values * 255.0), 0, 255).astype(np.uint8)

    return {
        "height": to_u8(height),
        "normal": to_u8(normal * 0.5 + 0.5),
        "roughness": to_u8(roughness),
    }


# --- generation ------------------------------------------------------------------


def _check_ledger(budget_path: Path, lot: str) -> None:
    """Refuse a paid call unless the ledger's last section is GA and the lot has room."""
    ledger = budget.BudgetLedger(budget_path)
    title = ledger.current_session.title or ""
    if not title.startswith(GA_SECTION_PREFIX):
        raise RuntimeError(
            f"La dernière section de {budget_path} n'est pas « {GA_SECTION_PREFIX} » "
            f"(trouvé : « {title} ») : la dépense serait mal consignée."
        )
    cap = LOT_CAPS.get(lot)
    if cap is None:
        return
    spent = sum(
        (
            entry.actual
            for entry in ledger.current_session.entries
            if entry.subject.startswith(lot)
        ),
        Decimal("0.00"),
    )
    if spent >= cap:
        raise budget.BudgetExceeded(
            f"Plafond du lot {lot} atteint : {spent} $ / {cap} $"
        )


def _square(image: Image.Image, size: int) -> Image.Image:
    """Centre-crop ``image`` to a square and resize it to ``size``."""
    side = min(image.size)
    left = (image.width - side) // 2
    top = (image.height - side) // 2
    square = image.crop((left, top, left + side, top + side))
    if side != size:
        square = square.resize((size, size), Image.Resampling.LANCZOS)
    return square


def process(
    material_id: str,
    raw_path: Path,
    out_dir: Path,
    *,
    materials_path: Path = MATERIALS_PATH,
) -> dict[str, Path]:
    """Turn a raw generated image into the albedo tile and its derived maps (no paid call).

    Writes ``<id>_albedo.png``, ``<id>_height.png``, ``<id>_normal.png`` and
    ``<id>_roughness.png`` (all ``tile_size``²) in ``out_dir``.
    """
    document = load_materials(materials_path)
    entry = material_entry(material_id, materials_path)
    size, tile_size = int(document["size"]), int(document["tile_size"])
    raw = _square(Image.open(raw_path).convert("RGB"), size)
    tileable = make_tileable(np.asarray(raw), int(entry.get("blend_width", size // 8)))
    # Downscaling a periodic image with a plain filter is not exactly periodic at the
    # borders: resize a 3x3 mosaic and keep its centre to stay seamless.
    mosaic = Image.fromarray(np.tile(tileable, (3, 3, 1)))
    mosaic = mosaic.resize((tile_size * 3, tile_size * 3), Image.Resampling.LANCZOS)
    albedo = np.asarray(mosaic)[tile_size : 2 * tile_size, tile_size : 2 * tile_size]
    maps = derive_maps(albedo, **_map_params(entry))
    out_dir.mkdir(parents=True, exist_ok=True)
    paths = {"albedo": out_dir / f"{material_id}_albedo.png"}
    Image.fromarray(albedo).save(paths["albedo"])
    for name, values in maps.items():
        paths[name] = out_dir / f"{material_id}_{name}.png"
        Image.fromarray(values).save(paths[name])
    return paths


def generate(
    material_id: str,
    out_dir: Path,
    *,
    lot: str = "GA1",
    materials_path: Path = MATERIALS_PATH,
    budget_path: Path = budget.DEFAULT_BUDGET_PATH,
) -> Path:
    """Generate one material (paid unless its raw image exists) and return its albedo tile.

    The raw image is kept as ``<out_dir>/<id>_raw.png``; an existing raw image is
    reused without any call, so an interrupted batch never pays twice. The spend is
    recorded by :func:`openrouter.generate_image` in the GA section of the ledger.
    """
    from cent_ans_tools import openrouter

    out_dir = Path(out_dir)
    document = load_materials(materials_path)
    entry = material_entry(material_id, materials_path)
    raw_path = out_dir / f"{material_id}_raw.png"
    if not raw_path.exists():
        _check_ledger(Path(budget_path), lot)
        model = document["model"]
        openrouter.generate_image(
            model,
            entry["prompt"],
            raw_path,
            subject=f"{lot} : matière {material_id} ({model})",
            budget_path=budget_path,
        )
    return process(material_id, raw_path, out_dir, materials_path=materials_path)[
        "albedo"
    ]


# --- review sheet ----------------------------------------------------------------


# Latin font with accents (Pillow's built-in default font has none).
SHEET_FONT = (
    ROOT
    / "game"
    / "assets"
    / "third_party"
    / "fonts"
    / "eb_garamond"
    / "EBGaramond-VariableFont_wght.ttf"
)


def _font(size: int) -> ImageFont.ImageFont | ImageFont.FreeTypeFont:
    try:
        return ImageFont.truetype(str(SHEET_FONT), size)
    except OSError:
        return ImageFont.load_default(size=size)


def _map_params(entry: dict[str, Any]) -> dict[str, float]:
    """``derive_maps`` keyword arguments of a ``materials.yaml`` entry."""
    return {
        "height_strength": float(entry.get("height_strength", 1.0)),
        "roughness_bias": float(entry.get("roughness_bias", 0.0)),
    }


def contact_sheet(
    tiles: dict[str, np.ndarray],
    out_path: Path,
    *,
    panel: int = 320,
    max_bytes: int = SHEET_MAX_BYTES,
    params: dict[str, dict[str, Any]] | None = None,
) -> Path:
    """Write a labelled review sheet of ``tiles`` and return its path.

    ``tiles`` maps a material id to its albedo tile (H x W x 3 uint8). One row per
    material: albedo repeated 2x2 (seams visible if any), derived normal, roughness.
    ``params`` maps an id to its ``materials.yaml`` entry, so the derived maps use
    the material's own ``height_strength``/``roughness_bias`` (defaults otherwise).
    Falls back to a 256-colour palette PNG, then shrinks it, while it exceeds
    ``max_bytes``.
    """
    label_width, header, gap = 150, 30, 8
    columns = ("albédo 2×2", "normale", "rugosité")
    width = label_width + len(columns) * (panel + gap)
    height = header + len(tiles) * (panel + gap)
    sheet = Image.new("RGB", (width, height), (40, 38, 34))
    draw = ImageDraw.Draw(sheet)
    font = _font(18)
    for column, title in enumerate(columns):
        draw.text(
            (label_width + column * (panel + gap) + 6, 6),
            title,
            fill=(230, 220, 200),
            font=font,
        )
    for row, (material_id, albedo) in enumerate(tiles.items()):
        top = header + row * (panel + gap)
        draw.text((8, top + 8), material_id, fill=(230, 220, 200), font=font)
        maps = derive_maps(albedo, **_map_params((params or {}).get(material_id, {})))
        panels = [
            Image.fromarray(np.tile(albedo[..., :3], (2, 2, 1))),
            Image.fromarray(maps["normal"]),
            Image.fromarray(maps["roughness"]).convert("RGB"),
        ]
        for column, image in enumerate(panels):
            resized = image.resize((panel, panel), Image.Resampling.LANCZOS)
            sheet.paste(resized, (label_width + column * (panel + gap), top))
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, optimize=True)
    # Too heavy: 256-colour palette, then shrink until it fits.
    while out_path.stat().st_size > max_bytes:
        sheet.quantize(colors=256, method=Image.Quantize.MEDIANCUT).save(
            out_path, optimize=True
        )
        if out_path.stat().st_size <= max_bytes or sheet.width < 400:
            break
        sheet = sheet.resize(
            (int(sheet.width * 0.85), int(sheet.height * 0.85)),
            Image.Resampling.LANCZOS,
        )
    return out_path


# --- fine figure arrays (GA1) ----------------------------------------------------

FINE_TEXTURES_DIR = ROOT / "game" / "assets" / "models" / "battle_fine" / "textures"
DETAIL_ARRAY = "fine_detail_ga1.png"
ALBEDO_ARRAY = "fine_detail_albedo.png"
# Detail albedo layers are smaller than the RG/B/A tiles (memory: GA1 adds <= 4 MB).
ALBEDO_LAYER_SIZE = 256


def centred_albedo(albedo: np.ndarray, saturation: float) -> np.ndarray:
    """Albedo factor tile: hue kept at ``saturation``, mean luminance 0.5 (x2 in the shader).

    The shader multiplies the vertex colour by ``2 * texel``: a mean of 0.5 keeps the
    data-driven colour (livery, heraldry) on average and only adds the material's detail.
    """
    rgb = albedo[..., :3].astype(np.float64) / 255.0
    luminance = rgb @ _LUMA
    mixed = luminance[..., np.newaxis] + saturation * (rgb - luminance[..., np.newaxis])
    mean = float(luminance.mean()) or 1.0
    centred = np.clip(mixed * (0.5 / mean), 0.0, 1.0)
    return np.clip(np.rint(centred * 255.0), 0, 255).astype(np.uint8)


def build_fine_arrays(
    tile_dir: Path,
    out_dir: Path = FINE_TEXTURES_DIR,
    *,
    materials_path: Path = MATERIALS_PATH,
) -> dict[str, Path]:
    """Assemble the GA1 texture arrays of the fine figures from processed tiles.

    Reads ``<id>_albedo.png`` / ``_normal.png`` / ``_height.png`` / ``_roughness.png``
    of every material (``materials.yaml`` order = layer) and writes two vertical strips
    imported by Godot as ``Texture2DArray`` (one slice per material):

    - ``fine_detail_ga1.png``: ``tile_size``² RGBA, RG normal (OpenGL), B relief
      (height), A roughness -- same packing as the FG3 tiles of ADR 0088;
    - ``fine_detail_albedo.png``: ``ALBEDO_LAYER_SIZE``² RGB, centred albedo factor.
    """
    entries = load_materials(materials_path)["materials"]
    detail_layers = []
    albedo_layers = []
    for entry in entries:
        material_id = entry["id"]

        def read(name: str, material_id: str = material_id) -> np.ndarray:
            return np.asarray(Image.open(Path(tile_dir) / f"{material_id}_{name}.png"))

        normal, height, roughness = read("normal"), read("height"), read("roughness")
        detail_layers.append(
            np.dstack([normal[..., 0], normal[..., 1], height, roughness])
        )
        albedo = centred_albedo(
            read("albedo"), float(entry.get("detail_saturation", 0.0))
        )
        small = Image.fromarray(np.tile(albedo, (3, 3, 1))).resize(
            (ALBEDO_LAYER_SIZE * 3, ALBEDO_LAYER_SIZE * 3), Image.Resampling.LANCZOS
        )
        albedo_layers.append(
            np.asarray(small)[
                ALBEDO_LAYER_SIZE : 2 * ALBEDO_LAYER_SIZE,
                ALBEDO_LAYER_SIZE : 2 * ALBEDO_LAYER_SIZE,
            ]
        )
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    paths = {"detail": out_dir / DETAIL_ARRAY, "albedo": out_dir / ALBEDO_ARRAY}
    Image.fromarray(np.concatenate(detail_layers), "RGBA").save(
        paths["detail"], optimize=True
    )
    Image.fromarray(np.concatenate(albedo_layers), "RGB").save(
        paths["albedo"], optimize=True
    )
    return paths
