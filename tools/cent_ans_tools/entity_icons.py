"""DA5b: entity icons as small painted miniatures in a common frame.

The bible (``docs/design/2026-09-25-bible-da.md`` § 8) wants entity icons (units,
buildings, technologies, skills, resources, diets) to be small painted miniatures
sharing one frame, never flat pictograms. The catalogue ``data/ui/entity_icons.json``
lists one miniature per entity:

- **derived** entries (``source``) reuse an existing painted illustration
  (``game/assets/illustrations/`` 16:9 miniatures): a square is cut on the subject
  (explicit ``crop`` or an automatic, deterministic saliency search), reduced and
  sharpened for small sizes — free;
- **generated** entries (``subject``) have no illustration: one square miniature is
  generated once with OpenRouter through :func:`cent_ans_tools.portraits.generate`
  (lot envelope, global cap of ``docs/budget.md``, one ledger row per batch). The
  prompt is the shared style block of the catalogue followed by the subject; the raw
  output is kept downscaled in ``tools/da5b_raw/<id>.jpg`` and never regenerated.

The free ``build`` step paints the common frame procedurally (ink outline, burnished
gold fillet, azure fillet — the spirit of the illuminated UI kit, ADR 0050, and of the
DA5 medallions) and writes ``game/assets/icons/entity/<id>.png`` (128 px, RGBA) plus
``index.json`` (game id -> file) read by ``IconLibrary`` before the SVG table.
"""

from __future__ import annotations

import io
import json
from collections.abc import Iterable
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageEnhance, ImageFilter
from scipy import ndimage

from cent_ans_tools.ink_icons import ensure_import_settings
from cent_ans_tools.portraits import PortraitJob

REPO_DIR = Path(__file__).resolve().parents[2]
CATALOG_PATH = REPO_DIR / "data" / "ui" / "entity_icons.json"
RAW_DIR = REPO_DIR / "tools" / "da5b_raw"
OUT_DIR = REPO_DIR / "game" / "assets" / "icons" / "entity"
BUDGET_SUBJECT = "DA5b : icônes d'entité en miniatures peintes"
LOT_PREFIX = "DA5b"
# Raw generated sources are stored downscaled (JPEG) to keep the repository small.
RAW_SIZE = 512
# Supersampling factor of the frame drawing (antialiasing).
SUPERSAMPLE = 4


@dataclass(frozen=True)
class Crop:
    """Square cut in a source picture: centre (0-1 of width/height), side / shorter side."""

    x: float
    y: float
    scale: float


@dataclass(frozen=True)
class Entry:
    """One catalogue miniature."""

    id: str
    label: str
    group: str
    targets: tuple[str, ...]
    source: str | None = None
    subject: str | None = None
    crop: Crop | None = None

    @property
    def generated(self) -> bool:
        """True when the picture comes from a paid generation (no illustration)."""
        return self.source is None

    @property
    def raw_path(self) -> Path:
        """Downscaled raw model output (``tools/da5b_raw/<id>.jpg``)."""
        return RAW_DIR / f"{self.id}.jpg"

    @property
    def out_path(self) -> Path:
        """Game asset written by :func:`build`."""
        return OUT_DIR / f"{self.id}.png"


def load_catalog(path: Path = CATALOG_PATH) -> dict:
    """The parsed catalogue."""
    return json.loads(path.read_text(encoding="utf-8"))


def entries(catalog: dict, group: str | None = None) -> list[Entry]:
    """Catalogue entries (optionally one ``group`` only), in catalogue order."""
    result = []
    for item in catalog["icons"]:
        if group not in (None, item["group"]):
            continue
        crop = item.get("crop")
        result.append(
            Entry(
                id=item["id"],
                label=item["label"],
                group=item["group"],
                targets=tuple(item.get("targets", [item["id"]])),
                source=item.get("source"),
                subject=item.get("subject"),
                crop=Crop(crop["x"], crop["y"], crop["scale"]) if crop else None,
            )
        )
    return result


# --- Generation ------------------------------------------------------------------------


def build_prompt(catalog: dict, entry: Entry) -> str:
    """Prompt of a generated miniature: lead and subject, then the shared style block.

    The prompt ends with the common style block, as every illumination tool does
    (bible § 5).
    """
    generation = catalog["generation"]
    return f"{generation['lead']} {entry.subject}.\n{generation['prompt']}"


def plan(
    catalog: dict,
    *,
    only: Iterable[str] | None = None,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Paid jobs still to run: generated entries without a raw file (``only`` filters ids)."""
    wanted = set(only or [])
    reference = catalog["generation"].get("reference")
    jobs: list[PortraitJob] = []
    for entry in entries(catalog):
        if not entry.generated or entry.raw_path.exists():
            continue
        if wanted and entry.id not in wanted:
            continue
        jobs.append(
            PortraitJob(
                character_id=entry.id,
                prompt=build_prompt(catalog, entry),
                out_path=entry.raw_path,
                reference=REPO_DIR / reference if reference else None,
            )
        )
    return jobs[:limit] if limit is not None else jobs


def to_raw(image_bytes: bytes) -> bytes:
    """Raw generated miniature kept in ``tools/da5b_raw`` (centre square, 512 px JPEG)."""
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    side = min(image.size)
    left = (image.size[0] - side) // 2
    top = (image.size[1] - side) // 2
    image = image.crop((left, top, left + side, top + side))
    image = image.resize((RAW_SIZE, RAW_SIZE), Image.Resampling.LANCZOS)
    out = io.BytesIO()
    image.save(out, format="JPEG", quality=92)
    return out.getvalue()


def lot_spent(ledger) -> Decimal:  # noqa: ANN001
    """Real spend already recorded for the lot (rows whose subject starts with ``DA5b``)."""
    return sum(
        (
            entry.actual
            for entry in ledger.entries
            if entry.subject.startswith(LOT_PREFIX)
        ),
        Decimal("0.00"),
    )


# --- Crop on the subject ---------------------------------------------------------------


def saliency(image: Image.Image) -> np.ndarray:
    """Subject map in [0, 1] of a small copy of ``image`` (deterministic).

    Detail (gradient energy) and colour saturation mark figures and buildings; the
    flat or diapered azure / gold backgrounds of the miniatures are damped (a
    repeated fleur-de-lis pattern has edges but is not a subject); a soft central
    bias keeps the cut near the composition's centre.
    """
    rgb = np.asarray(image.convert("RGB"), dtype=np.float32) / 255.0
    hsv = np.asarray(image.convert("HSV"), dtype=np.float32) / 255.0
    gray = rgb.mean(axis=-1)
    gx = ndimage.sobel(gray, axis=1)
    gy = ndimage.sobel(gray, axis=0)
    detail = ndimage.gaussian_filter(np.hypot(gx, gy), sigma=2.0)
    detail /= max(float(detail.max()), 1e-6)
    hue, sat, val = hsv[..., 0], hsv[..., 1], hsv[..., 2]
    # Azure background: blue hue (~210-240 deg), saturated, not bright.
    azure = (hue > 0.55) & (hue < 0.70) & (sat > 0.35) & (val < 0.75)
    azure_share = ndimage.uniform_filter(azure.astype(np.float32), size=9)
    colour = ndimage.gaussian_filter(sat * val, sigma=3.0)
    score = (0.65 * detail + 0.35 * colour) * (1.0 - 0.8 * azure_share)
    h, w = score.shape
    ys = (np.arange(h) - (h - 1) / 2.0) / h
    xs = (np.arange(w) - (w - 1) / 2.0) / w
    bias = np.exp(-(xs[None, :] ** 2) / (2 * 0.30**2)) * np.exp(
        -(ys[:, None] ** 2) / (2 * 0.45**2)
    )
    score = score * (0.35 + 0.65 * bias)
    return score / max(float(score.max()), 1e-6)


def auto_crop(image: Image.Image, scale: float) -> Crop:
    """Square of side ``scale`` × shorter side holding the most subject (first best)."""
    width, height = image.size
    work_w = 160
    work_h = max(1, round(height * work_w / width))
    small = image.convert("RGB").resize((work_w, work_h), Image.Resampling.BILINEAR)
    score = saliency(small)
    side = max(1, min(round(min(work_w, work_h) * scale), work_w, work_h))
    integral = np.pad(score.cumsum(0).cumsum(1), ((1, 0), (1, 0)))
    sums = (
        integral[side:, side:]
        - integral[:-side, side:]
        - integral[side:, :-side]
        + integral[:-side, :-side]
    )
    top, left = np.unravel_index(int(np.argmax(sums)), sums.shape)
    return Crop(
        x=round((left + side / 2) / work_w, 3),
        y=round((top + side / 2) / work_h, 3),
        scale=scale,
    )


def cut_square(image: Image.Image, crop: Crop) -> Image.Image:
    """The square ``crop`` of ``image``, clamped inside the picture."""
    width, height = image.size
    side = min(round(min(width, height) * crop.scale), width, height)
    left = min(max(round(crop.x * width - side / 2), 0), width - side)
    top = min(max(round(crop.y * height - side / 2), 0), height - side)
    return image.crop((left, top, left + side, top + side))


# --- Common frame ----------------------------------------------------------------------


def _rgb(hex_colour: str) -> tuple[int, int, int]:
    value = hex_colour.lstrip("#")
    return tuple(int(value[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]


def frame_inset(frame: dict, size: int) -> int:
    """Pixels between the icon edge and the painting, at ``size``."""
    widths = frame["ink_px"] + frame["gold_px"] + frame["azure_px"] + frame["ink_px"]
    return round(widths * size / 128)


def _rounded(size: int, inset: float, radius: float) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (inset, inset, size - 1 - inset, size - 1 - inset),
        radius=max(radius, 0.0),
        fill=255,
    )
    return mask


def frame_layers(frame: dict, size: int) -> tuple[Image.Image, Image.Image]:
    """The painted frame (RGBA) and the mask of the picture window, at ``size``."""
    big = size * SUPERSAMPLE
    unit = big / 128.0
    radius = frame["corner_px"] * unit
    ink = _rgb(frame["ink"])
    gold_dark = np.array(_rgb(frame["gold_dark"]), dtype=np.float32)
    gold_light = np.array(_rgb(frame["gold_light"]), dtype=np.float32)
    azure = _rgb(frame["azure"])
    rings = [
        (0.0, ink),
        (frame["ink_px"] * unit, None),  # gold (gradient)
        ((frame["ink_px"] + frame["gold_px"]) * unit, azure),
        ((frame["ink_px"] + frame["gold_px"] + frame["azure_px"]) * unit, ink),
    ]
    window_inset = (frame["ink_px"] * 2 + frame["gold_px"] + frame["azure_px"]) * unit
    # Burnished gold: light towards the top-left, dark towards the bottom-right.
    ramp = np.add.outer(np.arange(big), np.arange(big)).astype(np.float32) / (
        2 * (big - 1)
    )
    ripple = 0.5 + 0.5 * np.cos(ramp * np.pi * 3.0)
    t = np.clip(0.15 + 0.55 * ramp + 0.3 * (1 - ripple) * 0.5, 0, 1)[..., None]
    gold = (gold_light * (1 - t) + gold_dark * t).astype(np.uint8)
    canvas = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    for inset, colour in rings:
        mask = _rounded(big, inset, radius - inset)
        layer = (
            Image.fromarray(np.dstack([gold, np.full((big, big), 255, np.uint8)]))
            if colour is None
            else Image.new("RGBA", (big, big), (*colour, 255))
        )
        canvas.paste(layer, (0, 0), mask)
    window = _rounded(big, window_inset, radius - window_inset)
    return (
        canvas.resize((size, size), Image.Resampling.LANCZOS),
        window.resize((size, size), Image.Resampling.LANCZOS),
    )


def finish_picture(picture: Image.Image, side: int, style: dict) -> Image.Image:
    """Painting reduced to ``side`` px, with contrast and sharpening for small sizes."""
    picture = picture.convert("RGB").resize((side, side), Image.Resampling.LANCZOS)
    picture = ImageEnhance.Contrast(picture).enhance(style.get("contrast", 1.0))
    picture = ImageEnhance.Color(picture).enhance(style.get("saturation", 1.0))
    return picture.filter(
        ImageFilter.UnsharpMask(
            radius=1.2, percent=style.get("sharpen", 0), threshold=2
        )
    )


def compose(
    picture: Image.Image, catalog: dict, size: int | None = None
) -> Image.Image:
    """Framed miniature: painting under the window, then the common frame around it."""
    size = size or catalog["size"]
    frame = catalog["frame"]
    frame_image, window = frame_layers(frame, size)
    inner = finish_picture(picture, size, catalog.get("picture", {}))
    result = frame_image.copy()
    result.paste(inner, (0, 0), window)
    return result


# --- Build ---------------------------------------------------------------------------


def source_picture(entry: Entry, catalog: dict) -> tuple[Image.Image, Crop] | None:
    """The square painting of an entry and the crop used, None if its source is missing."""
    path = REPO_DIR / entry.source if entry.source else entry.raw_path
    if not path.exists():
        return None
    image = Image.open(path).convert("RGB")
    key = "derived_scale" if entry.source else "generated_scale"
    crop = entry.crop or auto_crop(image, catalog["crop"][key])
    return cut_square(image, crop), crop


def build(catalog: dict | None = None) -> dict[str, list[str]]:
    """Writes the framed PNGs and ``index.json`` from the sources (free).

    Returns ``{"derived": [...], "generated": [...], "missing": [...]}`` (ids).
    """
    catalog = catalog or load_catalog()
    report: dict[str, list[str]] = {"derived": [], "generated": [], "missing": []}
    index: dict[str, str] = {}
    crops: dict[str, list[float]] = {}
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for entry in entries(catalog):
        found = source_picture(entry, catalog)
        if found is None:
            report["missing"].append(entry.id)
            continue
        picture, crop = found
        compose(picture, catalog).save(entry.out_path, optimize=True)
        ensure_import_settings(entry.out_path)
        report["generated" if entry.generated else "derived"].append(entry.id)
        crops[entry.id] = [crop.x, crop.y, crop.scale]
        for target in entry.targets:
            index[target] = entry.out_path.name
    payload = {
        "source": "data/ui/entity_icons.json (lot DA5b)",
        "crops": crops,
        "icons": dict(sorted(index.items())),
    }
    (OUT_DIR / "index.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    return report


def contact_sheet(
    catalog: dict | None = None,
    out_path: Path | None = None,
    sizes: tuple[int, ...] = (32, 64, 128),
) -> Path:
    """Control sheet: every miniature at 32/64/128 px on parchment, grouped by kind."""
    catalog = catalog or load_catalog()
    out_path = out_path or REPO_DIR / "docs" / "img" / "da5b" / "planche_miniatures.png"
    icons = [e for e in entries(catalog) if e.out_path.exists()]
    columns = 6
    cell = sum(sizes) + 8 * len(sizes) + 16
    row_h = max(sizes) + 12
    rows = (len(icons) + columns - 1) // columns
    sheet = Image.new("RGB", (columns * cell, max(rows, 1) * row_h), (237, 222, 184))
    for index, entry in enumerate(icons):
        icon = Image.open(entry.out_path).convert("RGBA")
        x = (index % columns) * cell + 8
        y0 = (index // columns) * row_h + 6
        for size in sizes:
            scaled = icon.resize((size, size), Image.Resampling.LANCZOS)
            sheet.paste(scaled, (x, y0 + (max(sizes) - size) // 2), scaled)
            x += size + 8
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, optimize=True)
    return out_path
