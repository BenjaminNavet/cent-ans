"""Painted settlement markers of the campaign map (lot DA3, ADR 0066).

One language for every place on the map (bible DA § 8): the **shape** says the kind
(capital, city, town, village, castle, abbey, plus a port badge), the **shield** says
who holds it (composed at run time from the faction arms), the **size** says the rank.

Everything is driven by ``data/map/settlement_markers.json`` (schema
``data/schemas/settlement_markers.schema.json``):

1. :func:`plan` lists the pictograms whose raw painting is missing from
   :data:`RAW_DIR` (idempotent); the prompt is built from the entry ``subject`` and
   ends with :data:`STYLE`;
2. paid generation goes through :func:`cent_ans_tools.portraits.generate` (envelope,
   global cap, one ``docs/budget.md`` row per batch); :func:`to_raw_jpg` keeps the
   painting as a 768 px JPEG source;
3. :func:`build_atlas` (free, deterministic) cuts every painting out of its flat
   background (:func:`cut_out`), frames it in its cell, adds the common ink contour and
   vellum halo (:func:`outline`) and writes the single atlas read by
   ``settlement_icon.gdshader``.
"""

from __future__ import annotations

import io
import json
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.portraits import PortraitJob

REPO_DIR = Path(__file__).resolve().parents[2]
CATALOG_PATH = REPO_DIR / "data" / "map" / "settlement_markers.json"
RAW_DIR = REPO_DIR / "tools" / "assets" / "map_markers"
GAME_DIR = REPO_DIR / "game"
RAW_SIZE = 768
RAW_QUALITY = 90

# Ink of the illuminated UI (hud_style.gd INK, ui_illumination.INK) and vellum halo.
INK = (51, 31, 15)
VELLUM = (241, 227, 194)
# Background keying: distance (0-255 RGB) to the sampled background colour.
KEY_TOLERANCE = 38.0
# Sobel magnitude of the blurred luminance (0-255) above which a pixel is an edge.
GRADIENT_LIMIT = 40.0
# Contour widths in atlas pixels (cell of 128 px): ink line, then a thin vellum halo
# that keeps the silhouette readable over dark forests and the sea.
INK_WIDTH = 3
HALO_WIDTH = 2
# Share of the cell the painting may fill (the rest is contour and halo).
FILL = 0.86

STYLE = (
    "Style: a single small pictogram painted like a vignette on a 14th-century French "
    "Gothic illuminated map (enluminure gothique, atelier de Jean Pucelle): egg tempera, "
    "bold dark-brown ink outlines of even thickness, flat muted earth colours for stone, "
    "timber and roofs (ochre, grey stone, brick red, slate blue) with small gold-leaf "
    "highlights. Frontal elevated view, compact silhouette, centred and filling about 85 % "
    "of the frame, readable when reduced to 32 pixels. Plain flat pure white background, "
    "no ground plane, no landscape, no cast shadow, no frame, no border, no flag, no "
    "banner, no coat of arms, no text, no letters."
)


def load_catalog(path: Path = CATALOG_PATH) -> dict:
    """Parsed ``settlement_markers.json``."""
    return json.loads(path.read_text(encoding="utf-8"))


def raw_path(pictogram_id: str, raw_dir: Path = RAW_DIR) -> Path:
    """Raw painting of a pictogram (source of the atlas)."""
    return raw_dir / f"{pictogram_id}.jpg"


def atlas_path(catalog: dict, game_dir: Path = GAME_DIR) -> Path:
    """Absolute path of the atlas (``res://`` resolved)."""
    return game_dir / catalog["atlas"]["path"].removeprefix("res://")


def build_prompt(pictogram: dict) -> str:
    """Prompt of one pictogram, from the data only, ending with :data:`STYLE`."""
    grounding = (
        "It floats on the white background like a badge."
        if pictogram.get("badge")
        else "It stands on a small oval patch of grass and earth, no wider than its walls."
    )
    return f"Map pictogram: {pictogram['subject']}. {grounding}\n{STYLE}"


def plan(
    catalog: dict | None = None,
    *,
    raw_dir: Path = RAW_DIR,
    only: list[str] | None = None,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Pictograms whose raw painting is missing (idempotent), in catalogue order.

    ``only`` restricts to the given pictogram ids (3-image style probe).
    """
    catalog = catalog or load_catalog()
    jobs: list[PortraitJob] = []
    for pictogram in catalog["pictograms"]:
        out_path = raw_path(pictogram["id"], raw_dir)
        if out_path.exists() or (only is not None and pictogram["id"] not in only):
            continue
        jobs.append(PortraitJob(pictogram["id"], build_prompt(pictogram), out_path))
    return jobs[:limit] if limit is not None else jobs


def to_raw_jpg(image_bytes: bytes, size: int = RAW_SIZE) -> bytes:
    """Model output resized to the raw source format (square JPEG)."""
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    side = min(image.size)
    left = (image.width - side) // 2
    top = (image.height - side) // 2
    image = image.crop((left, top, left + side, top + side))
    buffer = io.BytesIO()
    image.resize((size, size), Image.Resampling.LANCZOS).save(
        buffer, "JPEG", quality=RAW_QUALITY, optimize=True
    )
    return buffer.getvalue()


def cut_out(
    image: Image.Image,
    tolerance: float = KEY_TOLERANCE,
    gradient_limit: float = GRADIENT_LIMIT,
) -> Image.Image:
    """RGBA copy of ``image`` with its background (connected to the border) removed.

    The model rarely paints the asked flat white: it gives a vellum or gold wash with a
    vignette. A pixel is background-like when it is smooth (luminance gradient below
    ``gradient_limit``) and not far from the border colours (``tolerance`` × 3); the
    background is the part of it connected to the border. The painted ink contour of
    the subject stops the flood, so the light stone inside the silhouette is kept.
    The silhouette is closed (holes filled), eroded by one pixel (no fringe of the wash)
    and its alpha edge softened.
    """
    rgb = np.asarray(image.convert("RGB"), dtype=np.float32)
    smooth = ndimage.gaussian_filter(rgb, (1.2, 1.2, 0))
    luminance = smooth @ np.array([0.299, 0.587, 0.114], dtype=np.float32)
    gradient = np.hypot(
        ndimage.sobel(luminance, axis=0), ndimage.sobel(luminance, axis=1)
    )
    border = np.concatenate([smooth[0], smooth[-1], smooth[:, 0], smooth[:, -1]])
    distance = np.min(
        np.stack(
            [
                np.linalg.norm(smooth - colour, axis=2)
                for colour in np.percentile(border, [10, 50, 90], axis=0)
            ]
        ),
        axis=0,
    )
    candidate = (gradient < gradient_limit) & (distance < tolerance * 3.0)
    labels, _ = ndimage.label(candidate)
    edge_labels = np.unique(
        np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    )
    edge_labels = edge_labels[edge_labels > 0]
    background_mask = np.isin(labels, edge_labels)
    solid = ndimage.binary_fill_holes(~background_mask)
    solid = ndimage.binary_opening(solid, iterations=2)
    solid = ndimage.binary_erosion(solid, iterations=1)
    alpha = ndimage.gaussian_filter(solid.astype(np.float32), 0.6)
    rgba = np.dstack([rgb, np.clip(alpha, 0.0, 1.0) * 255.0]).astype(np.uint8)
    return Image.fromarray(rgba, "RGBA")


def fit_cell(cutout: Image.Image, cell_px: int, fill: float = FILL) -> Image.Image:
    """Crop to the silhouette and centre it in a transparent ``cell_px`` square.

    The silhouette keeps its proportions; its bottom sits at the same height in every
    cell so that the markers share a baseline.
    """
    box = cutout.getchannel("A").point(lambda value: 255 if value > 24 else 0).getbbox()
    if box is None:
        return Image.new("RGBA", (cell_px, cell_px), (0, 0, 0, 0))
    subject = cutout.crop(box)
    target = cell_px * fill
    scale = target / max(subject.width, subject.height)
    width = max(1, round(subject.width * scale))
    height = max(1, round(subject.height * scale))
    subject = subject.resize((width, height), Image.Resampling.LANCZOS)
    cell = Image.new("RGBA", (cell_px, cell_px), (0, 0, 0, 0))
    margin = (cell_px - target) / 2
    left = round((cell_px - width) / 2)
    top = round(cell_px - margin - height)
    cell.alpha_composite(subject, (left, max(0, top)))
    return cell


def outline(
    cell: Image.Image, ink_width: int = INK_WIDTH, halo_width: int = HALO_WIDTH
) -> Image.Image:
    """Common contour of every marker: an ink line, then a thin vellum halo."""
    alpha = np.asarray(cell.getchannel("A"), dtype=np.float32) / 255.0
    solid = alpha > 0.5
    ink = ndimage.binary_dilation(solid, iterations=ink_width)
    halo = ndimage.binary_dilation(ink, iterations=halo_width)
    ink_a = ndimage.gaussian_filter(ink.astype(np.float32), 0.7)
    halo_a = ndimage.gaussian_filter(halo.astype(np.float32), 0.9) * 0.85
    size = cell.size
    base = np.zeros((size[1], size[0], 4), dtype=np.float32)
    base[..., :3] = VELLUM
    base[..., 3] = halo_a
    ink_layer = np.zeros_like(base)
    ink_layer[..., :3] = INK
    ink_layer[..., 3] = ink_a
    result = Image.fromarray(np.clip(base * [1, 1, 1, 255], 0, 255).astype(np.uint8))
    result.alpha_composite(
        Image.fromarray(np.clip(ink_layer * [1, 1, 1, 255], 0, 255).astype(np.uint8))
    )
    result.alpha_composite(cell)
    return result


def build_atlas(
    catalog: dict | None = None,
    *,
    raw_dir: Path = RAW_DIR,
    out_path: Path | None = None,
) -> tuple[Path, list[str]]:
    """Assemble the atlas from the raw paintings (free); returns (path, missing ids).

    A pictogram without raw painting leaves its cell empty (the shader then draws
    nothing but the shield): the game still runs during a partial generation.
    """
    catalog = catalog or load_catalog()
    cell_px = int(catalog["atlas"]["cell_px"])
    columns = int(catalog["atlas"]["columns"])
    cells = max(int(p["cell"]) for p in catalog["pictograms"]) + 1
    rows = (cells + columns - 1) // columns
    atlas = Image.new("RGBA", (columns * cell_px, rows * cell_px), (0, 0, 0, 0))
    missing: list[str] = []
    for pictogram in catalog["pictograms"]:
        source = raw_path(pictogram["id"], raw_dir)
        if not source.exists():
            missing.append(pictogram["id"])
            continue
        inner = cell_px - 2 * (INK_WIDTH + HALO_WIDTH + 1)
        fitted = fit_cell(cut_out(Image.open(source)), inner)
        padded = Image.new("RGBA", (cell_px, cell_px), (0, 0, 0, 0))
        offset = (cell_px - inner) // 2
        padded.alpha_composite(fitted, (offset, offset))
        cell = int(pictogram["cell"])
        atlas.alpha_composite(
            outline(padded), ((cell % columns) * cell_px, (cell // columns) * cell_px)
        )
    out_path = out_path or atlas_path(catalog)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    atlas.save(out_path, optimize=True)
    return out_path, missing
