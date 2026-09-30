"""NB: Nano Banana 2 redraw of the illuminated UI kit, guided by shape and style.

Pipeline (spec ``docs/superpowers/specs/2026-09-30-nb-nano-banana-interface-design.md``):
each piece of ``game/assets/ui/illumination/kit.json`` is sent to the image model with
two references, the style anchor (``data/art/style/anchor.png``) and the current
procedural texture upscaled on a green background (shape guide). The raw answer is
keyed out, fitted back to the piece geometry, its stretched bands made seamless, and
written to ``game/assets/ui/illumination/nb/<id>.png`` where ``ui_illumination.build``
picks it up instead of the procedural drawing.
"""

from __future__ import annotations

import io
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path
from typing import Any

import numpy as np
import yaml
from PIL import Image, ImageDraw, ImageFont

from cent_ans_tools import openrouter

ROOT = Path(__file__).resolve().parents[2]
ORNAMENTS_PATH = ROOT / "data" / "art" / "ui_ornaments.yaml"
ANCHOR_PATH = ROOT / "data" / "art" / "style" / "anchor.png"
KIT_DIR = ROOT / "game" / "assets" / "ui" / "illumination"
NB_DIR = KIT_DIR / "nb"
RAW_DIR = ROOT / "tools" / "nb_raw" / "ui"

# Ledger section every NB spend lands in, and its own envelope (spec § 2).
NB_SECTION = "Nano Banana NB"
NB_CAP = Decimal("10.00")
# Pure green used as the keyable background of generated pieces.
KEY_GREEN = (0, 255, 0)


@dataclass(frozen=True)
class OrnamentRequest:
    """One paid generation: a kit piece (or decor) variant with its references."""

    piece_id: str
    variant: int
    prompt: str
    references: tuple[bytes, ...]
    image_config: dict[str, str]
    seed: int


# Fixed prompt suffixes (spec § 6.2).
GUIDE_SUFFIX = (
    " Image 1 is the style reference, Image 2 is the shape guide: keep its exact "
    "geometry, border thickness and inner area; plain vellum interior; pure #00FF00 "
    "flat background outside the frame; no text."
)
DECOR_SUFFIX = (
    " Image 1 is the style reference. Single isolated ornament on a pure #00FF00 "
    "flat background; no text."
)
# Alpha ramp of the chroma key: G excess over max(R, B) below KEY_LOW is opaque,
# above KEY_HIGH is fully transparent.
KEY_LOW = 20.0
KEY_HIGH = 110.0
PARCHMENT = (232, 220, 190)


def load_ornaments(path: Path = ORNAMENTS_PATH) -> dict[str, Any]:
    """Return the parsed ``ui_ornaments.yaml`` document."""
    return yaml.safe_load(Path(path).read_text(encoding="utf-8"))


def shape_guide(piece_png: Path, scale: int) -> bytes:
    """Nearest-neighbour upscale of a kit texture over pure green, as PNG bytes."""
    with Image.open(piece_png) as opened:
        piece = opened.convert("RGBA")
    piece = piece.resize((piece.width * scale, piece.height * scale), Image.NEAREST)
    background = Image.new("RGBA", piece.size, (*KEY_GREEN, 255))
    background.alpha_composite(piece)
    buffer = io.BytesIO()
    background.convert("RGB").save(buffer, format="PNG")
    return buffer.getvalue()


def build_requests(
    kit: dict[str, Any],
    anchor: bytes,
    config: dict[str, Any],
    kit_dir: Path = KIT_DIR,
) -> list[OrnamentRequest]:
    """Build every request from the kit geometry, the anchor and the YAML config."""
    requests: list[OrnamentRequest] = []
    guide_scale = int(config.get("guide_scale", 6))
    for section in ("pieces", "decor"):
        is_piece = section == "pieces"
        for entry in config.get(section) or []:
            piece_id = entry["id"]
            if is_piece:
                if piece_id not in kit:
                    continue
                references = (
                    anchor,
                    shape_guide(kit_dir / f"{piece_id}.png", guide_scale),
                )
                suffix = GUIDE_SUFFIX
            else:
                references = (anchor,)
                suffix = DECOR_SUFFIX
            for variant in range(int(entry["variants"])):
                requests.append(
                    OrnamentRequest(
                        piece_id=piece_id,
                        variant=variant,
                        prompt=entry["prompt"].rstrip() + suffix,
                        references=references,
                        image_config={
                            "aspect_ratio": entry["aspect_ratio"],
                            "image_size": entry["image_size"],
                        },
                        seed=int(entry.get("seed", 1337)) + variant,
                    )
                )
    return requests


def generate(
    requests: list[OrnamentRequest],
    raw_dir: Path = RAW_DIR,
    dry_run: bool = False,
    model: str | None = None,
) -> list[Path]:
    """Paid calls (budget-guarded); raw images already in ``raw_dir`` are reused."""
    model = model or load_ornaments()["model"]
    if not dry_run:
        raw_dir.mkdir(parents=True, exist_ok=True)
    paths: list[Path] = []
    total = Decimal("0")
    for request in requests:
        path = raw_dir / f"{request.piece_id}_v{request.variant}.png"
        paths.append(path)
        if path.exists():
            continue
        if dry_run:
            price = openrouter.estimate_price(
                model, image_size=request.image_config.get("image_size")
            )
            total += price
            print(f"{request.piece_id} v{request.variant} : ~{price} $")
            continue
        openrouter.generate_image(
            model,
            request.prompt,
            path,
            subject=f"NB1 : {request.piece_id} v{request.variant} ({model})",
            images=list(request.references),
            image_config=request.image_config,
            seed=request.seed,
            cap=NB_CAP,
        )
    if dry_run:
        print(f"Total estimé : ~{total} $")
    return paths


def key_out(image: np.ndarray) -> np.ndarray:
    """Return RGBA with the green background removed and green fringes decontaminated."""
    rgb = image[..., :3].astype(np.float64)
    red, green, blue = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    excess = green - np.maximum(red, blue)
    ramp = np.clip((excess - KEY_LOW) / (KEY_HIGH - KEY_LOW), 0.0, 1.0)
    ramp = ramp * ramp * (3 - 2 * ramp)
    alpha = 1.0 - ramp
    if image.shape[2] == 4:
        alpha = alpha * (image[..., 3].astype(np.float64) / 255.0)
    green = np.where(excess > 0, np.minimum(green, np.maximum(red, blue)), green)
    out = np.dstack([red, green, blue, alpha * 255.0])
    return np.clip(np.rint(out), 0, 255).astype(np.uint8)


def fit_to_piece(image: np.ndarray, piece: dict[str, Any]) -> np.ndarray:
    """Crop to the opaque bounds and resize to the piece ``size`` of ``kit.json``."""
    rows, cols = np.nonzero(image[..., 3] > 8)
    if rows.size:
        image = image[rows.min() : rows.max() + 1, cols.min() : cols.max() + 1]
    width, height = piece["size"]
    resized = Image.fromarray(image, "RGBA").resize((width, height), Image.LANCZOS)
    return np.asarray(resized).copy()


def _seamless_band(band: np.ndarray) -> np.ndarray:
    """Make a band (axis 1 is the stretched axis) wrap: first and last columns match."""
    length = band.shape[1]
    if length < 8:
        return band
    source = band.astype(np.float64)
    blend = max(2, min(length // 4, 24))
    rolled = np.roll(source, length // 2, axis=1)
    # 1 at the band edges (rolled copy is continuous there), 0 towards the centre.
    position = np.minimum(np.arange(length), np.arange(length)[::-1]).astype(float)
    weight = np.clip(1.0 - position / blend, 0.0, 1.0)[np.newaxis, :, np.newaxis]
    blended = rolled * weight + source * (1 - weight)
    # Residual mismatch between the two edges is spread over the last columns.
    gap = blended[:, -1] - blended[:, 0]
    ramp = np.clip((np.arange(length) - (length - 1 - blend)) / blend, 0.0, 1.0)
    blended = blended - gap[:, np.newaxis, :] * ramp[np.newaxis, :, np.newaxis]
    return np.clip(np.rint(blended), 0, 255).astype(np.uint8)


def seam_fix(image: np.ndarray, piece: dict[str, Any]) -> np.ndarray:
    """Make the stretched central bands (between 9-slice margins) seamless."""
    left, top, right, bottom = piece["margins"]
    height, width = image.shape[:2]
    result = image.copy()
    if width - left - right >= 8:
        band = result[:, left : width - right]
        result[:, left : width - right] = _seamless_band(band)
    if height - top - bottom >= 8:
        band = np.swapaxes(result[top : height - bottom], 0, 1)
        fixed = np.swapaxes(_seamless_band(np.ascontiguousarray(band)), 0, 1)
        result[top : height - bottom] = fixed
    # Corners and margins stay exactly as fitted.
    result[:top, :left] = image[:top, :left]
    result[:top, width - right :] = image[:top, width - right :]
    result[height - bottom :, :left] = image[height - bottom :, :left]
    result[height - bottom :, width - right :] = image[
        height - bottom :, width - right :
    ]
    return result


def process(raw_path: Path, piece: dict[str, Any]) -> np.ndarray:
    """Raw model image -> keyed, fitted, seamless RGBA ready for the kit."""
    with Image.open(raw_path) as opened:
        raw = np.asarray(opened.convert("RGB"))
    return seam_fix(fit_to_piece(key_out(raw), piece), piece)


def install(
    selected: dict[str, int],
    kit: dict[str, Any] | None = None,
    raw_dir: Path = RAW_DIR,
    nb_dir: Path = NB_DIR,
) -> list[Path]:
    """Process the selected variant of each piece into ``nb_dir/<id>.png``."""
    if kit is None:
        import json

        kit = json.loads((KIT_DIR / "kit.json").read_text(encoding="utf-8"))
    nb_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for piece_id, variant in selected.items():
        result = process(raw_dir / f"{piece_id}_v{variant}.png", kit[piece_id])
        target = nb_dir / f"{piece_id}.png"
        Image.fromarray(result, "RGBA").save(target, optimize=True)
        written.append(target)
    return written


def contact_sheet(before: list[Path], after: list[Path], out_path: Path) -> Path:
    """Write a before/after review sheet of the kit pieces."""
    font = ImageFont.load_default()
    scale = 3
    images = []
    for before_path, after_path in zip(before, after, strict=True):
        pair = []
        for path in (before_path, after_path):
            with Image.open(path) as opened:
                tile = opened.convert("RGBA")
            pair.append(
                tile.resize((tile.width * scale, tile.height * scale), Image.NEAREST)
            )
        images.append((before_path.stem, pair))
    label_width, gap = 150, 10
    column = max((p.width for _, pair in images for p in pair), default=1)
    while True:
        rows = [max(p.height for p in pair) for _, pair in images]
        sheet = Image.new(
            "RGBA",
            (label_width + 2 * column + 3 * gap, sum(rows) + gap * (len(rows) + 1)),
            (*PARCHMENT, 255),
        )
        draw = ImageDraw.Draw(sheet)
        y = gap
        for (name, pair), row_height in zip(images, rows, strict=True):
            draw.text((gap, y), name, fill=(40, 30, 20, 255), font=font)
            for index, tile in enumerate(pair):
                x = label_width + gap + index * (column + gap)
                sheet.alpha_composite(tile, (x, y))
            y += row_height + gap
        buffer = io.BytesIO()
        sheet.convert("RGB").save(buffer, format="PNG", optimize=True)
        if buffer.tell() < 900_000 or scale == 1:
            break
        scale -= 1
        images = [
            (
                name,
                [
                    t.resize(
                        (
                            t.width * scale // (scale + 1),
                            t.height * scale // (scale + 1),
                        ),
                        Image.NEAREST,
                    )
                    for t in pair
                ],
            )
            for name, pair in images
        ]
        column = max(p.width for _, pair in images for p in pair)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(buffer.getvalue())
    return out_path
