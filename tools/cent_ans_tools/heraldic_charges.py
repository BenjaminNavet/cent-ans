"""Vector heraldic charges (lot DA1b): drawn lions, leopards, eagle, dolphin, castle, guivre.

The drawings are public-domain, CC0 or CC BY SVG files from Wikimedia Commons, vendored in
``data/heraldry/charges/`` with a manifest (``charges.json``, credits in ``SOURCE.md``). Each
colour of a file plays a *role* (body, armed, crown, outline...). To recolour a charge by
tincture, the SVG is rendered once per role with that role's colours in white and every other
colour in black, on black: the grey level is the coverage of the role at each pixel, so the
anti-aliasing stays exact. The masks are cropped to the drawing, cached, then scaled and
painted with the tinctures read from the blazon.

Rendering uses ``resvg`` (``resvg-py`` wheel): same SVG, same library version, same pixels.
"""

from __future__ import annotations

import io
import json
import re
from dataclasses import dataclass
from functools import cache
from pathlib import Path

import resvg_py
from PIL import Image, ImageFilter

REPO_DIR = Path(__file__).resolve().parents[2]
CHARGES_DIR = REPO_DIR / "data" / "heraldry" / "charges"
MANIFEST = CHARGES_DIR / "charges.json"

# Height at which every SVG is rasterised before cropping and scaling down.
RENDER_HEIGHT = 1024
ROLE_ORDER = (
    "body",
    "shade",
    "armed",
    "crown",
    "openings",
    "issant",
    "argent",
    "outline",
)
_COLOR_RE = re.compile(r"#(?:[0-9a-fA-F]{6}|[0-9a-fA-F]{3})\b")
_ELEMENT_RE = re.compile(r"<(?:path|ellipse|circle|rect|polygon)\b[^>]*?/>", re.S)


@dataclass(frozen=True)
class ChargeArt:
    """Role masks of one charge, cropped to the drawing (L images, same size)."""

    size: tuple[int, int]
    masks: dict[str, Image.Image]


@cache
def manifest() -> dict:
    """The charge manifest (``data/heraldry/charges/charges.json``)."""
    return json.loads(MANIFEST.read_text(encoding="utf-8"))


def charge_ids() -> list[str]:
    """Ids of the vendored charges."""
    return sorted(manifest()["charges"])


def _long_hex(color: str) -> str:
    color = color.lower()
    if len(color) == 4:
        return "#" + "".join(c * 2 for c in color[1:])
    return color


def _prepared_svg(entry: dict) -> str:
    """SVG text with dropped elements removed and inherited default fill made explicit."""
    text = (CHARGES_DIR / entry["file"]).read_text(encoding="utf-8")
    drop = {_long_hex(color) for color in entry.get("drop", [])}
    if drop:

        def keep(match: re.Match) -> str:
            colors = {_long_hex(c) for c in _COLOR_RE.findall(match.group(0))}
            return "" if colors & drop else match.group(0)

        text = _ELEMENT_RE.sub(keep, text)
    # Elements without a fill are black: make the default explicit on the root so that
    # it is recoloured like the others.
    root = re.search(r"<svg\b[^>]*>", text)
    if root and re.search(r"\sfill=", root.group(0)):
        return text
    return re.sub(r"<svg\b", '<svg fill="#000000"', text, count=1)


def _render(svg: str, background: str | None) -> Image.Image:
    png = resvg_py.svg_to_bytes(
        svg_string=svg, height=RENDER_HEIGHT, background=background
    )
    return Image.open(io.BytesIO(bytes(png)))


@cache
def charge_art(charge_id: str) -> ChargeArt:
    """Role masks of ``charge_id``, rendered once and cropped to the drawing."""
    entry = manifest()["charges"][charge_id]
    svg = _prepared_svg(entry)
    roles = {_long_hex(color): role for color, role in entry["colors"].items()}
    alpha = _render(svg, None).convert("RGBA").getchannel("A")
    box = alpha.getbbox()
    if box is None:
        raise ValueError(f"empty charge drawing: {charge_id}")
    masks: dict[str, Image.Image] = {}
    for role in ROLE_ORDER:
        if role not in roles.values():
            continue

        def recolor(match: re.Match, role: str = role) -> str:
            color = _long_hex(match.group(0))
            return "#ffffff" if roles.get(color) == role else "#000000"

        layer = _render(_COLOR_RE.sub(recolor, svg), "#000000").convert("L")
        masks[role] = layer.crop(box)
    width, height = box[2] - box[0], box[3] - box[1]
    return ChargeArt((width, height), masks)


def fitted_masks(
    charge_id: str,
    box_size: tuple[int, int],
    outline_boost: int = 0,
    mirror: bool = False,
) -> tuple[dict[str, Image.Image], tuple[int, int]]:
    """Role masks scaled to fit ``box_size`` (pixels), aspect ratio kept.

    ``outline_boost`` (odd pixel size, 0 = none) thickens the outline so that the inner
    lines survive the final downsampling of small shields.
    """
    art = charge_art(charge_id)
    width, height = art.size
    factor = min(box_size[0] / width, box_size[1] / height)
    size = (max(1, round(width * factor)), max(1, round(height * factor)))
    masks = {}
    for role, mask in art.masks.items():
        scaled = mask.resize(size, Image.Resampling.LANCZOS)
        if mirror:
            scaled = scaled.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
        if role == "outline" and outline_boost > 1:
            scaled = scaled.filter(ImageFilter.MaxFilter(outline_boost))
        masks[role] = scaled
    return masks, size


def crown_anchor(charge_id: str) -> tuple[float, float, float] | None:
    """Where a crown is added on a charge drawn without one (fractions of the drawing)."""
    anchor = manifest()["charges"][charge_id].get("crown_anchor")
    return tuple(anchor) if anchor else None


def credits() -> list[dict]:
    """Source, author and licence of every vendored charge."""
    return [
        {"id": charge_id, **manifest()["charges"][charge_id]}
        for charge_id in charge_ids()
    ]
