"""Local 2 x 2 tiled review sheet, outside the repository (T1b)."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw, ImageFont

from cent_ans_tools.texture_factory import RAW_ROOT


def _font(size: int) -> ImageFont.ImageFont | ImageFont.FreeTypeFont:
    try:
        return ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", size)
    except OSError:
        return ImageFont.load_default()


def board(
    document: dict[str, Any], raw_dir: Path | None = None, cell: int = 512
) -> Path:
    """Review sheet: each albedo tile repeated 2 x 2 (seams at the cell centre lines).

    Reads ``raw_dir/tiles/<id>_albedo.png`` and writes ``raw_dir/board_2x2.jpg``.
    """
    raw_dir = raw_dir if raw_dir is not None else RAW_ROOT
    materials = document["materials"]
    columns = 6
    rows = -(-len(materials) // columns)
    sheet = Image.new("RGB", (columns * cell, rows * (cell + 28)), (30, 30, 30))
    draw = ImageDraw.Draw(sheet)
    font = _font(20)
    for index, entry in enumerate(materials):
        path = raw_dir / "tiles" / f"{entry['id']}_albedo.png"
        if not path.exists():
            continue
        tile = Image.open(path).convert("RGB").resize((cell // 2, cell // 2))
        x, y = index % columns * cell, index // columns * (cell + 28)
        for dx in (0, cell // 2):
            for dy in (0, cell // 2):
                sheet.paste(tile, (x + dx, y + 28 + dy))
        draw.text(
            (x + 6, y + 3), f"{entry['layer']} {entry['id']}", fill="white", font=font
        )
    target = raw_dir / "board_2x2.jpg"
    sheet.save(target, quality=85)
    return target
