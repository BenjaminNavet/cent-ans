"""GA/SR: tileable materials of the fine figures and their maps.

Generated (GA1): prompt (``data/art/materials.yaml``) -> 1024x1024 image via
:func:`cent_ans_tools.openrouter.generate_image` (budget-guarded) -> tileable
(half offset + seam blend) -> derived maps (height from high-passed luminance,
OpenGL normal, roughness) -> contact sheet for review.

Scanned (SR1): ``source: ambientcg:<Id>`` -> CC0 1K-JPG maps downloaded once into
``~/dev/cent-ans-raw/sr1/`` -> tile at physical scale (``scan_m``/``tile_m``) with the
scan's own normal, roughness and displacement.
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
from cent_ans_tools.paths import REPO_DIR as ROOT

MATERIALS_PATH = ROOT / "data" / "art" / "materials.yaml"

# Ledger section every GA spend must land in (``docs/budget.md``).
GA_SECTION_PREFIX = "Assets générés GA"
# Per-lot ceiling (spec: GA1 <= 4 $), checked against the ledger rows of the lot.
LOT_CAPS = {"GA1": Decimal("4.00")}
# Rec. 709 luma weights.
_LUMA = np.array([0.2126, 0.7152, 0.0722], dtype=np.float64)
# Largest review sheet kept in the repository.
SHEET_MAX_BYTES = 1_000_000

# SR1: CC0 PBR scans from ambientCG (``source: ambientcg:<AssetId>``).
AMBIENTCG_API = "https://ambientcg.com/api/v2/full_json"
AMBIENTCG_SOURCE_PREFIX = "ambientcg:"
AMBIENTCG_RESOLUTION = "1K-JPG"
# Generic client identification only: no personal data leaves the machine.
AMBIENTCG_USER_AGENT = "cent-ans-sr/0.1"
# Raw downloads stay outside the repository (never deleted by the pipeline).
SCAN_CACHE_DIR = Path.home() / "dev" / "cent-ans-raw" / "sr1"
# Map suffixes of an ambientCG material zip used by the pipeline.
SCAN_MAPS = ("Color", "NormalGL", "Roughness", "Displacement")


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


def budget_settings(document: dict[str, Any]) -> dict[str, Any]:
    """Ledger settings of a materials document: section prefix, cap, lot, image estimate.

    Without a ``budget`` block (``materials.yaml``, GA1) the GA section and its lot caps
    apply, as before. A block (``water_materials.yaml``, RC5) names its own section, its
    own ceiling and a per-image estimate, so the run is capped without any network call.
    """
    block = document.get("budget")
    if not block:
        return {
            "section": GA_SECTION_PREFIX,
            "cap": None,
            "lot": None,
            "estimate": None,
        }
    return {
        "section": block["section"],
        "cap": Decimal(str(block["cap"])),
        "lot": block.get("lot"),
        "estimate": Decimal(str(block["estimate_per_image"])),
    }


def _check_ledger(
    budget_path: Path,
    lot: str,
    *,
    section: str = GA_SECTION_PREFIX,
    cap: Decimal | None = None,
    estimate: Decimal = Decimal("0.00"),
) -> None:
    """Refuse a paid call unless the ledger's last section is ``section`` and has room.

    With ``cap`` (RC5 envelope) the whole section's cumulative spend plus ``estimate``
    must stay within it; otherwise the per-lot ceiling of ``LOT_CAPS`` (GA1) applies.
    """
    ledger = budget.BudgetLedger(budget_path)
    title = ledger.current_session.title or ""
    if not title.startswith(section):
        raise RuntimeError(
            f"La dernière section de {budget_path} n'est pas « {section} » "
            f"(trouvé : « {title} ») : la dépense serait mal consignée."
        )
    if cap is not None:
        spent_total = ledger.total()
        if spent_total + estimate > cap:
            raise budget.BudgetExceeded(
                f"Plafond de la section « {section} » : {spent_total} $ dépensés "
                f"+ {estimate} $ estimés > {cap} $"
            )
        return
    lot_cap = LOT_CAPS.get(lot)
    if lot_cap is None:
        return
    spent = sum(
        (
            entry.actual
            for entry in ledger.current_session.entries
            if entry.subject.startswith(lot)
        ),
        Decimal("0.00"),
    )
    if spent >= lot_cap:
        raise budget.BudgetExceeded(
            f"Plafond du lot {lot} atteint : {spent} $ / {lot_cap} $"
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
    lot: str | None = None,
    materials_path: Path = MATERIALS_PATH,
    budget_path: Path = budget.DEFAULT_BUDGET_PATH,
    envelope: Decimal | None = None,
    local: bool = False,
) -> Path:
    """Generate one material (paid unless its raw image exists) and return its albedo tile.

    A scanned material (``source: ambientcg:<Id>``) is fetched and cut instead (free).

    The raw image is kept as ``<out_dir>/<id>_raw.png``; an existing raw image is
    reused without any call, so an interrupted batch never pays twice (delete it to
    retry a bad draw). The spend is recorded by :func:`openrouter.generate_image` in
    the ledger. ``envelope`` lowers the section cap of a ``budget`` block (RC5).
    ``local`` renders with the free mflux model (square 1:1, no ledger check or row,
    ADR 0190).
    """
    from cent_ans_tools import local_art, openrouter

    out_dir = Path(out_dir)
    document = load_materials(materials_path)
    entry = material_entry(material_id, materials_path)
    if scan_asset_id(entry) is not None:
        return process_scan(material_id, out_dir, materials_path=materials_path)[
            "albedo"
        ]
    settings = budget_settings(document)
    lot = lot or settings["lot"] or "GA1"
    raw_path = out_dir / f"{material_id}_raw.png"
    if not raw_path.exists() and local:
        out_dir.mkdir(parents=True, exist_ok=True)
        raw_path.write_bytes(
            local_art.render_image(entry["prompt"], aspect_ratio="1:1")
        )
    elif not raw_path.exists():
        cap = settings["cap"]
        if cap is not None and envelope is not None:
            cap = min(cap, envelope)
        _check_ledger(
            Path(budget_path),
            lot,
            section=settings["section"],
            cap=cap,
            estimate=settings["estimate"] or Decimal("0.00"),
        )
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


# --- scanned materials (SR1, ambientCG CC0) ---------------------------------------


def scan_asset_id(entry: dict[str, Any]) -> str | None:
    """AmbientCG asset id of a ``source: ambientcg:<Id>`` entry, ``None`` if generated."""
    source = entry.get("source")
    if not source:
        return None
    if not source.startswith(AMBIENTCG_SOURCE_PREFIX):
        raise ValueError(f"Source de matière inconnue : {source}")
    return source[len(AMBIENTCG_SOURCE_PREFIX) :]


def _http_get(url: str) -> bytes:
    """GET ``url`` with a generic User-Agent (no personal data in the headers)."""
    import urllib.request

    request = urllib.request.Request(url, headers={"User-Agent": AMBIENTCG_USER_AGENT})
    with urllib.request.urlopen(request, timeout=120) as response:  # noqa: S310
        return response.read()


def _scan_map_paths(asset_dir: Path, asset_id: str) -> dict[str, Path]:
    return {
        name: asset_dir / f"{asset_id}_{AMBIENTCG_RESOLUTION}_{name}.jpg"
        for name in SCAN_MAPS
    }


def ambientcg_download_url(metadata: dict[str, Any], asset_id: str) -> str:
    """URL of the ``AMBIENTCG_RESOLUTION`` zip in an API v2 ``full_json`` answer."""
    for asset in metadata.get("foundAssets", []):
        if asset.get("assetId", "").lower() != asset_id.lower():
            continue
        folders = asset.get("downloadFolders", {})
        for folder in folders.values():
            categories = folder.get("downloadFiletypeCategories", {})
            for download in categories.get("zip", {}).get("downloads", []):
                if download.get("attribute") == AMBIENTCG_RESOLUTION:
                    return download["fullDownloadPath"]
    raise KeyError(
        f"Pas d'archive {AMBIENTCG_RESOLUTION} pour {asset_id} sur ambientCG"
    )


def fetch_ambientcg(
    asset_id: str,
    cache_dir: Path = SCAN_CACHE_DIR,
    *,
    http_get: Any = None,
) -> dict[str, Path]:
    """Return the Color/NormalGL/Roughness/Displacement maps of an ambientCG asset.

    Downloads the ``1K-JPG`` zip once (API v2) into ``<cache_dir>/<asset_id>/`` and
    extracts the four maps; a complete cache is reused without any network access.
    """
    import io
    import json
    import zipfile

    http_get = http_get or _http_get
    asset_dir = Path(cache_dir) / asset_id
    paths = _scan_map_paths(asset_dir, asset_id)
    if all(path.exists() for path in paths.values()):
        return paths
    asset_dir.mkdir(parents=True, exist_ok=True)
    zip_path = asset_dir / f"{asset_id}_{AMBIENTCG_RESOLUTION}.zip"
    if not zip_path.exists():
        metadata = json.loads(
            http_get(f"{AMBIENTCG_API}?id={asset_id}&include=downloadData")
        )
        (asset_dir / "metadata.json").write_text(json.dumps(metadata, indent=1))
        zip_path.write_bytes(http_get(ambientcg_download_url(metadata, asset_id)))
    with zipfile.ZipFile(io.BytesIO(zip_path.read_bytes())) as archive:
        for path in paths.values():
            path.write_bytes(archive.read(path.name))
    return paths


def _decode_normal(normal: np.ndarray) -> np.ndarray:
    vectors = normal[..., :3].astype(np.float64) / 127.5 - 1.0
    return vectors / np.maximum(np.linalg.norm(vectors, axis=-1, keepdims=True), 1e-6)


def _encode_normal(vectors: np.ndarray) -> np.ndarray:
    vectors = vectors / np.maximum(
        np.linalg.norm(vectors, axis=-1, keepdims=True), 1e-6
    )
    return np.clip(np.rint((vectors * 0.5 + 0.5) * 255.0), 0, 255).astype(np.uint8)


def _periodic_resize(image: np.ndarray, size: int) -> np.ndarray:
    """Resize a tileable image to ``size``² and keep it seamless (3x3 mosaic centre)."""
    if image.shape[0] == size and image.shape[1] == size:
        return image
    reps = (3, 3, 1) if image.ndim == 3 else (3, 3)
    mosaic = Image.fromarray(np.tile(image, reps))
    mosaic = mosaic.resize((size * 3, size * 3), Image.Resampling.LANCZOS)
    return np.asarray(mosaic)[size : 2 * size, size : 2 * size]


def scan_tile(
    maps: dict[str, np.ndarray],
    *,
    scan_m: float,
    tile_m: float,
    tile_size: int,
    blend_width: int = 64,
    crop_centre: tuple[float, float] = (0.5, 0.5),
    normal_strength: float = 1.0,
    roughness_bias: float = 0.0,
) -> dict[str, np.ndarray]:
    """Cut a tileable ``tile_m`` tile out of a ``scan_m`` scan (physical scale kept).

    ``maps`` holds the scan's ``albedo`` (H x W x 3), ``normal`` (OpenGL, H x W x 3),
    ``roughness`` and ``height`` (H x W) as uint8. A tile covering the whole scan keeps
    the scan's own seamless wrap; a smaller one is a square crop around ``crop_centre``
    (fractions of the scan) made tileable with the same blend on every map, normals
    renormalised after blending and resizing. ``blend_width`` is in output pixels.
    Returns ``albedo``, ``normal``, ``height`` (centred on 128) and ``roughness``, all
    ``tile_size``².
    """
    if not 0 < tile_m <= scan_m * 1.0001:
        raise ValueError(f"tile_m ({tile_m}) doit être dans ]0, scan_m = {scan_m}]")
    height_px, width_px = maps["albedo"].shape[:2]
    scan_px = min(height_px, width_px)
    side = min(scan_px, int(round(scan_px * tile_m / scan_m)))
    whole = side >= scan_px and height_px == width_px
    tiles: dict[str, np.ndarray] = {}
    for name in ("albedo", "normal", "roughness", "height"):
        image = maps[name]
        if not whole:
            centre_x = int(round(crop_centre[0] * width_px))
            centre_y = int(round(crop_centre[1] * height_px))
            left = int(np.clip(centre_x - side // 2, 0, width_px - side))
            top = int(np.clip(centre_y - side // 2, 0, height_px - side))
            image = image[top : top + side, left : left + side]
            blend = int(round(blend_width * side / tile_size))
            image = make_tileable(image, int(np.clip(blend, 1, side // 2 - 1)))
        tiles[name] = image
    vectors = _decode_normal(tiles["normal"])
    vectors[..., :2] *= normal_strength
    normal = _encode_normal(vectors)
    normal = _encode_normal(_decode_normal(_periodic_resize(normal, tile_size)))
    albedo = _periodic_resize(tiles["albedo"][..., :3], tile_size)
    roughness = _periodic_resize(tiles["roughness"], tile_size).astype(np.float64)
    roughness = np.clip(roughness / 255.0 + roughness_bias, 0.0, 1.0)
    relief = _periodic_resize(tiles["height"], tile_size).astype(np.float64)
    relief -= relief.mean()
    spread = float(np.percentile(np.abs(relief), 98)) or 1.0
    relief = np.clip(0.5 + 0.5 * relief / spread, 0.0, 1.0)

    def to_u8(values: np.ndarray) -> np.ndarray:
        return np.clip(np.rint(values * 255.0), 0, 255).astype(np.uint8)

    return {
        "albedo": albedo,
        "normal": normal,
        "height": to_u8(relief),
        "roughness": to_u8(roughness),
    }


def _read_scan(paths: dict[str, Path]) -> dict[str, np.ndarray]:
    return {
        "albedo": np.asarray(Image.open(paths["Color"]).convert("RGB")),
        "normal": np.asarray(Image.open(paths["NormalGL"]).convert("RGB")),
        "roughness": np.asarray(Image.open(paths["Roughness"]).convert("L")),
        "height": np.asarray(Image.open(paths["Displacement"]).convert("L")),
    }


def process_scan(
    material_id: str,
    out_dir: Path,
    *,
    materials_path: Path = MATERIALS_PATH,
    cache_dir: Path = SCAN_CACHE_DIR,
    http_get: Any = None,
) -> dict[str, Path]:
    """Fetch (cached) and cut the scan of a ``source: ambientcg`` entry (no paid call).

    Writes ``<id>_albedo.png``, ``_normal.png``, ``_height.png`` and ``_roughness.png``
    (``tile_size``²) in ``out_dir``, like :func:`process` for a generated material.
    """
    document = load_materials(materials_path)
    entry = material_entry(material_id, materials_path)
    asset_id = scan_asset_id(entry)
    if asset_id is None:
        raise ValueError(f"{material_id} n'est pas une matière scannée")
    maps = _read_scan(fetch_ambientcg(asset_id, cache_dir, http_get=http_get))
    tiles = scan_tile(
        maps,
        scan_m=float(entry["scan_m"]),
        tile_m=float(entry["tile_m"]),
        tile_size=int(document["tile_size"]),
        blend_width=int(entry.get("blend_width", 64)),
        crop_centre=tuple(entry.get("crop_centre", (0.5, 0.5))),
        normal_strength=float(entry.get("normal_strength", 1.0)),
        roughness_bias=float(entry.get("roughness_bias", 0.0)),
    )
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    paths = {}
    for name, values in tiles.items():
        paths[name] = out_dir / f"{material_id}_{name}.png"
        Image.fromarray(values).save(paths[name])
    return paths


def plan(
    ids: list[str] | None,
    out_dir: Path,
    *,
    materials_path: Path = MATERIALS_PATH,
    local: bool = False,
) -> dict[str, Any]:
    """Dry run: prompts and estimated cost of a batch, without any API call.

    Returns ``{"model", "items": [{"id", "prompt", "reuse", "cost"}], "total", "cap"}``;
    an image whose raw file already exists in ``out_dir`` costs nothing.
    """
    from cent_ans_tools import local_art

    local_model = local_art.MODEL_ID
    document = load_materials(materials_path)
    settings = budget_settings(document)
    per_image = Decimal("0.00") if local else settings["estimate"] or Decimal("0.00")
    wanted = ids or [entry["id"] for entry in document["materials"]]
    items = []
    for material_id in wanted:
        entry = material_entry(material_id, materials_path)
        reuse = (Path(out_dir) / f"{material_id}_raw.png").exists()
        items.append(
            {
                "id": material_id,
                "prompt": entry["prompt"],
                "reuse": reuse,
                "cost": Decimal("0.00") if reuse else per_image,
            }
        )
    return {
        "model": local_model if local else document["model"],
        "items": items,
        "total": sum((item["cost"] for item in items), Decimal("0.00")),
        "cap": settings["cap"],
    }


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


def _previous_layers(
    out_dir: Path, count: int
) -> tuple[list[np.ndarray], list[np.ndarray]] | None:
    """Layers of the arrays already in ``out_dir`` if both exist with ``count`` layers."""
    detail_path, albedo_path = out_dir / DETAIL_ARRAY, out_dir / ALBEDO_ARRAY
    if not (detail_path.exists() and albedo_path.exists()):
        return None
    detail = np.asarray(Image.open(detail_path).convert("RGBA"))
    albedo = np.asarray(Image.open(albedo_path).convert("RGB"))
    if detail.shape[0] % count or albedo.shape[0] % count:
        raise ValueError(f"Tableaux existants incompatibles avec {count} couches")
    return (
        np.split(detail, count),
        np.split(albedo, count),
    )


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

    A material without tiles in ``tile_dir`` keeps its layer of the arrays already in
    ``out_dir`` (same order), copied verbatim: kept layers are never regenerated (SR1).
    """
    entries = load_materials(materials_path)["materials"]
    previous = _previous_layers(Path(out_dir), len(entries))
    detail_layers = []
    albedo_layers = []
    for index, entry in enumerate(entries):
        material_id = entry["id"]

        def read(name: str, material_id: str = material_id) -> np.ndarray:
            return np.asarray(Image.open(Path(tile_dir) / f"{material_id}_{name}.png"))

        if not (Path(tile_dir) / f"{material_id}_normal.png").exists():
            if previous is None:
                raise FileNotFoundError(
                    f"Ni tuiles de {material_id} dans {tile_dir} ni tableaux existants"
                )
            detail_layers.append(previous[0][index])
            albedo_layers.append(previous[1][index])
            continue
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


def layers_sheet(
    out_path: Path,
    textures_dir: Path = FINE_TEXTURES_DIR,
    *,
    materials_path: Path = MATERIALS_PATH,
    cell: int = 150,
    max_bytes: int = 400_000,
) -> Path:
    """Write a JPEG review sheet of the built arrays: per layer, albedo 2x2 and normal.

    Reads ``fine_detail_albedo.png`` and ``fine_detail_ga1.png`` (all layers, kept ones
    included), four layers per row; lowers the JPEG quality until under ``max_bytes``.
    """
    entries = load_materials(materials_path)["materials"]
    count = len(entries)
    detail = np.split(
        np.asarray(Image.open(Path(textures_dir) / DETAIL_ARRAY).convert("RGBA")), count
    )
    albedo = np.split(
        np.asarray(Image.open(Path(textures_dir) / ALBEDO_ARRAY).convert("RGB")), count
    )
    per_row, gap, label = 4, 10, 26
    pair = 2 * cell + 4
    rows = (count + per_row - 1) // per_row
    sheet = Image.new(
        "RGB", (per_row * (pair + gap) + gap, rows * (cell + label + gap)), (40, 38, 34)
    )
    draw = ImageDraw.Draw(sheet)
    font = _font(18)
    for index, entry in enumerate(entries):
        left = gap + (index % per_row) * (pair + gap)
        top = (index // per_row) * (cell + label + gap)
        source = entry.get("source", "généré GA1")
        caption = f"{index} {entry['id']} — {source} — {entry['tile_m']} m"
        draw.text((left, top + 3), caption, fill=(230, 220, 200), font=font)
        tiled = Image.fromarray(np.tile(albedo[index], (2, 2, 1)))
        normal_rgb = detail[index][..., :2]
        vectors = normal_rgb.astype(np.float64) / 127.5 - 1.0
        z = np.sqrt(np.clip(1.0 - (vectors**2).sum(axis=-1), 0.0, 1.0))
        normal = np.dstack(
            [normal_rgb, np.rint((z * 0.5 + 0.5) * 255).astype(np.uint8)]
        )
        for column, image in enumerate((tiled, Image.fromarray(normal))):
            resized = image.resize((cell, cell), Image.Resampling.LANCZOS)
            sheet.paste(resized, (left + column * (cell + 4), top + label))
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    for quality in (85, 75, 65, 55, 45):
        sheet.save(out_path, "JPEG", quality=quality, optimize=True)
        if out_path.stat().st_size <= max_bytes:
            break
    return out_path
