"""DA5: ink action icons and illuminated medallion buttons.

The catalogue ``data/ui/icons_ink.json`` lists one image per action icon (a single
brown-ink line drawing) and per medallion button (gold, azure and vermilion
illumination). Each paid image is generated once with OpenRouter through
:func:`cent_ans_tools.portraits.generate` (envelope, global cap of
``docs/budget.md``, one ledger row per batch), with a reference picture from the
validated DA plate so that the whole family keeps the same hand.

Raw model outputs are kept, downscaled, in ``tools/da5_raw/`` (``icons/<id>.jpg``,
``medallions/<id>.jpg``): a raw file that exists is never regenerated. The free
``build`` step then derives the game assets from them:

- icons: alpha drawn from the ink (luminance distance to the parchment), noise
  floor removed, cropped, padded to a square, stroke weight normalised (dilate or
  erode so that every icon has the same line width at 128 px), RGB = ``INK`` of
  ``HudStyle`` -> ``game/assets/icons/ink/<id>.png``;
- medallions: parchment around the disc flood-filled from the corners and made
  transparent, soft edge, cropped -> ``game/assets/ui/medallions/<id>.png``.

Both folders get an ``index.json`` (game id -> file) read by ``IconLibrary``.
"""

from __future__ import annotations

import io
import json
from collections.abc import Iterable
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

from cent_ans_tools.portraits import PortraitJob

REPO_DIR = Path(__file__).resolve().parents[2]
CATALOG_PATH = REPO_DIR / "data" / "ui" / "icons_ink.json"
RAW_DIR = REPO_DIR / "tools" / "da5_raw"
ICONS_OUT_DIR = REPO_DIR / "game" / "assets" / "icons" / "ink"
MEDALLIONS_OUT_DIR = REPO_DIR / "game" / "assets" / "ui" / "medallions"
BUDGET_SUBJECT = "DA5 : icônes d'action à l'encre et boutons-médaillons"

# HudStyle.INK (Color(0.22, 0.14, 0.07)) in 8 bits: the colour baked into the icons.
INK_RGB = (56, 36, 18)
# Raw sources are stored downscaled (JPEG) to keep the repository small.
RAW_ICON_SIZE = 512
RAW_MEDALLION_SIZE = 768
# Target stroke width of every icon at 128 px (so ~1.2 px at 24 px).
TARGET_STROKE_PX = 6.0
# Alpha below this share of full ink is parchment texture, not a stroke.
ALPHA_FLOOR = 0.18
# Medallion flood fill tolerance (per channel sum distance to the corner colour).
FLOOD_THRESHOLD = 60


@dataclass(frozen=True)
class Entry:
    """One catalogue image (icon or medallion)."""

    kind: str  # "icon" | "medallion"
    id: str
    label: str
    subject: str
    targets: tuple[str, ...]
    source: str | None = None

    @property
    def raw_path(self) -> Path:
        """Downscaled raw model output (``tools/da5_raw/<kind>s/<id>.jpg``)."""
        return RAW_DIR / f"{self.kind}s" / f"{self.id}.jpg"

    @property
    def out_path(self) -> Path:
        """Game asset written by :func:`build`."""
        folder = ICONS_OUT_DIR if self.kind == "icon" else MEDALLIONS_OUT_DIR
        return folder / f"{self.id}.png"


def load_catalog(path: Path = CATALOG_PATH) -> dict:
    """The parsed catalogue."""
    return json.loads(path.read_text(encoding="utf-8"))


def entries(catalog: dict, kind: str | None = None) -> list[Entry]:
    """Icons then medallions of the catalogue (optionally one ``kind`` only)."""
    result = []
    for key, entry_kind in (("icons", "icon"), ("medallions", "medallion")):
        if kind not in (None, entry_kind):
            continue
        for item in catalog[key]:
            result.append(
                Entry(
                    kind=entry_kind,
                    id=item["id"],
                    label=item["label"],
                    subject=item["subject"],
                    targets=tuple(item["targets"]),
                    source=item.get("source"),
                )
            )
    return result


def build_prompt(catalog: dict, entry: Entry) -> str:
    """Prompt of one image: the shared style block of its kind, then the subject."""
    style = catalog["icon_style" if entry.kind == "icon" else "medallion_style"]
    return f"{style['prompt']} {entry.subject}."


def reference_path(catalog: dict, entry: Entry) -> Path:
    """Reference picture sent with the prompt (validated DA plate)."""
    style = catalog["icon_style" if entry.kind == "icon" else "medallion_style"]
    return REPO_DIR / style["reference"]


def plan(
    catalog: dict,
    *,
    kind: str | None = None,
    only: Iterable[str] | None = None,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Paid jobs still to run: entries without a raw file and without a validated source.

    ``only`` selects ids (``icon:<id>``/``medallion:<id>`` or a bare id matching both).
    """
    wanted = set(only or [])
    jobs: list[PortraitJob] = []
    for entry in entries(catalog, kind):
        if entry.source is not None or entry.raw_path.exists():
            continue
        if wanted and not ({entry.id, f"{entry.kind}:{entry.id}"} & wanted):
            continue
        jobs.append(
            PortraitJob(
                character_id=f"{entry.kind}:{entry.id}",
                prompt=build_prompt(catalog, entry),
                out_path=entry.raw_path,
                reference=reference_path(catalog, entry),
            )
        )
    return jobs[:limit] if limit is not None else jobs


def to_raw_icon(image_bytes: bytes) -> bytes:
    """Raw icon kept in ``tools/da5_raw`` (512 px JPEG)."""
    return _to_raw(image_bytes, RAW_ICON_SIZE)


def to_raw_medallion(image_bytes: bytes) -> bytes:
    """Raw medallion kept in ``tools/da5_raw`` (768 px JPEG)."""
    return _to_raw(image_bytes, RAW_MEDALLION_SIZE)


def _to_raw(image_bytes: bytes, size: int) -> bytes:
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    image = image.resize((size, size), Image.Resampling.LANCZOS)
    out = io.BytesIO()
    image.save(out, format="JPEG", quality=92)
    return out.getvalue()


# --- Icons: alpha from the ink --------------------------------------------------------


def ink_alpha(image: Image.Image) -> np.ndarray:
    """Ink coverage in [0, 1] from luminance: 0 on the parchment, 1 on the darkest ink."""
    gray = np.asarray(image.convert("L"), dtype=np.float32)
    border = np.concatenate(
        [gray[:8].ravel(), gray[-8:].ravel(), gray[:, :8].ravel(), gray[:, -8:].ravel()]
    )
    paper = float(np.median(border))
    ink = float(np.percentile(gray, 1.0))
    if paper - ink < 20.0:
        return np.zeros_like(gray)
    alpha = np.clip((paper - gray) / (paper - ink), 0.0, 1.0)
    alpha = np.where(
        alpha < ALPHA_FLOOR, 0.0, (alpha - ALPHA_FLOOR) / (1.0 - ALPHA_FLOOR)
    )
    return np.clip(alpha * 1.15, 0.0, 1.0)


def stroke_width(alpha: np.ndarray) -> float:
    """Mean stroke width in pixels: 2 x inked area / outline length (thin-stroke estimate)."""
    mask = alpha > 0.5
    area = float(mask.sum())
    if area == 0:
        return 0.0
    edges = (mask[:, 1:] != mask[:, :-1]).sum() + (mask[1:, :] != mask[:-1, :]).sum()
    return 2.0 * area / max(float(edges), 1.0)


def _crop_square(alpha: np.ndarray, margin: float = 0.08) -> np.ndarray:
    ys, xs = np.nonzero(alpha > 0.3)
    if len(xs) == 0:
        return alpha
    top, bottom, left, right = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    side = int(max(bottom - top, right - left) * (1.0 + 2.0 * margin)) + 2
    canvas = np.zeros((side, side), dtype=np.float32)
    oy = (side - (bottom - top)) // 2
    ox = (side - (right - left)) // 2
    canvas[oy : oy + bottom - top, ox : ox + right - left] = alpha[
        top:bottom, left:right
    ]
    return canvas


def process_icon(raw: Image.Image, size: int = 128) -> tuple[Image.Image, float]:
    """Game icon (RGBA, RGB = ink colour) and its final stroke width in pixels."""
    alpha = _crop_square(ink_alpha(raw))
    work = 4 * size
    mask = Image.fromarray((alpha * 255).astype(np.uint8)).resize(
        (work, work), Image.Resampling.LANCZOS
    )
    # Normalise the stroke: measured at the working size, scaled to the final size.
    # A k x k max (min) filter widens (thins) every stroke by k - 1 working pixels.
    for _ in range(4):
        width = stroke_width(np.asarray(mask, dtype=np.float32) / 255.0) * size / work
        if width <= 0.0 or abs(width - TARGET_STROKE_PX) < TARGET_STROKE_PX * 0.15:
            break
        delta = abs(TARGET_STROKE_PX - width) * work / size
        if width > TARGET_STROKE_PX:
            delta = min(delta, width * work / size * 0.5)  # never erase a stroke
        kernel = max(3, int(round(delta)) // 2 * 2 + 1)
        grow = width < TARGET_STROKE_PX
        mask = mask.filter(
            ImageFilter.MaxFilter(kernel) if grow else ImageFilter.MinFilter(kernel)
        )
    final = mask.resize((size, size), Image.Resampling.LANCZOS)
    width = stroke_width(np.asarray(final, dtype=np.float32) / 255.0)
    rgba = Image.new("RGBA", (size, size), (*INK_RGB, 0))
    rgba.putalpha(final)
    return rgba, width


# --- Medallions: transparent parchment ------------------------------------------------


def process_medallion(raw: Image.Image, size: int = 256) -> Image.Image:
    """Medallion with the surrounding parchment made transparent, cropped to its disc."""
    rgb = raw.convert("RGB")
    marker = (255, 0, 255)
    filled = rgb.copy()
    w, h = filled.size
    for corner in (
        (0, 0),
        (w - 1, 0),
        (0, h - 1),
        (w - 1, h - 1),
        (w // 2, 0),
        (w // 2, h - 1),
        (0, h // 2),
        (w - 1, h // 2),
    ):
        if filled.getpixel(corner) != marker:
            ImageDraw.floodfill(filled, corner, marker, thresh=FLOOD_THRESHOLD)
    data = np.asarray(filled)
    background = np.all(data == marker, axis=-1)
    mask = Image.fromarray(np.where(background, 0, 255).astype(np.uint8))
    # Close pinholes, then soften the cut edge.
    mask = mask.filter(ImageFilter.MinFilter(3)).filter(ImageFilter.GaussianBlur(1.2))
    rgba = rgb.copy()
    rgba.putalpha(mask)
    box = mask.point(lambda v: 255 if v > 32 else 0).getbbox() or (0, 0, w, h)
    rgba = rgba.crop(box)
    side = max(rgba.size)
    square = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    square.paste(rgba, ((side - rgba.size[0]) // 2, (side - rgba.size[1]) // 2))
    return square.resize((size, size), Image.Resampling.LANCZOS)


# --- Build ---------------------------------------------------------------------------


def source_image(entry: Entry) -> Image.Image | None:
    """The validated source or the raw model output of an entry, None if missing."""
    path = REPO_DIR / entry.source if entry.source else entry.raw_path
    if not path.exists():
        return None
    return Image.open(path).convert("RGB")


def build(catalog: dict | None = None) -> dict[str, list[str]]:
    """Writes the game assets and both ``index.json`` from the raw sources (free).

    Returns ``{"icons": [...], "medallions": [...], "missing": [...]}`` (ids).
    """
    catalog = catalog or load_catalog()
    report: dict[str, list[str]] = {"icons": [], "medallions": [], "missing": []}
    indexes: dict[str, dict] = {"icon": {}, "medallion": {}}
    widths: dict[str, float] = {}
    for entry in entries(catalog):
        raw = source_image(entry)
        if raw is None:
            report["missing"].append(f"{entry.kind}:{entry.id}")
            continue
        entry.out_path.parent.mkdir(parents=True, exist_ok=True)
        if entry.kind == "icon":
            image, width = process_icon(raw, catalog["icon_style"]["size"])
            widths[entry.id] = round(width, 2)
        else:
            image = process_medallion(raw, catalog["medallion_style"]["size"])
        image.save(entry.out_path, optimize=True)
        report[f"{entry.kind}s"].append(entry.id)
        for target in entry.targets:
            indexes[entry.kind][target] = entry.out_path.name
    _write_index(
        ICONS_OUT_DIR,
        indexes["icon"],
        {"color": "#" + "".join(f"{c:02x}" for c in INK_RGB), "stroke_px": widths},
    )
    _write_index(MEDALLIONS_OUT_DIR, indexes["medallion"], {})
    return report


def _write_index(folder: Path, mapping: dict[str, str], extra: dict) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    payload = {
        "source": "data/ui/icons_ink.json (lot DA5)",
        **extra,
        "icons": dict(sorted(mapping.items())),
    }
    (folder / "index.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )


def contact_sheet(
    catalog: dict | None = None,
    out_path: Path | None = None,
    sizes: tuple[int, ...] = (24, 48, 128),
) -> Path:
    """Control sheet: every icon at 24/48/128 px on parchment, INK then GOLD."""
    catalog = catalog or load_catalog()
    out_path = out_path or REPO_DIR / "docs" / "img" / "da5" / "planche_icones_24px.png"
    icons = [e for e in entries(catalog, "icon") if e.out_path.exists()]
    columns = 8
    cell = sum(sizes) + 12 * len(sizes) + 24
    row_h = max(sizes) + 16
    rows = (len(icons) + columns - 1) // columns
    sheet = Image.new(
        "RGB", (columns * cell, max(rows, 1) * row_h * 2), (237, 222, 184)
    )
    gold = (184, 143, 61)
    for index, entry in enumerate(icons):
        icon = Image.open(entry.out_path).convert("RGBA")
        x0 = (index % columns) * cell + 8
        y0 = (index // columns) * row_h * 2 + 8
        for line, tint in enumerate((None, gold)):
            x = x0
            for size in sizes:
                scaled = icon.resize((size, size), Image.Resampling.LANCZOS)
                if tint is not None:
                    solid = Image.new("RGBA", scaled.size, (*tint, 255))
                    solid.putalpha(scaled.getchannel("A"))
                    scaled = solid
                sheet.paste(
                    scaled, (x, y0 + line * row_h + (max(sizes) - size) // 2), scaled
                )
                x += size + 12
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path, optimize=True)
    return out_path


def lot_spent(ledger) -> Decimal:  # noqa: ANN001
    """Real spend already recorded for the DA5 lot (rows whose subject starts with ``DA5``)."""
    return sum(
        (entry.actual for entry in ledger.entries if entry.subject.startswith("DA5")),
        Decimal("0.00"),
    )
