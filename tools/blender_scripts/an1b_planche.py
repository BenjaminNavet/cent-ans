"""Lot AN1b: compose the check sheet of the new clips from ``an1b_render.py`` renders.

    uv run --project tools python tools/blender_scripts/an1b_planche.py HUMAN_DIR CAVALRY_DIR [OUT]

Writes ``docs/img/an1/an1b_clips.png`` (640 px wide): one tile per rendered frame, labelled
with its clip, the human clips first, then the horse clips.
"""

import os
import sys

from fg0_planche import INK, PAPER, font
from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "docs", "img", "an1", "an1b_clips.png")
WIDTH = 640


def _tiles(directory, prefix=""):
    """(label, image) of the PNGs of `directory`, sorted by name."""
    out = []
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".png") or not name.startswith(prefix):
            continue
        stem = name[:-4]
        parts = stem.split("_")
        # <figure>_<n>_<clip...>_<k> (human) or clip_<figure>_<n>_<clip...>_<k> (horse)
        clip = "_".join(parts[3:-1] if stem.startswith("clip_") else parts[2:-1])
        img = Image.open(os.path.join(directory, name)).convert("RGB")
        box = _content_box(img)
        out.append((f"{clip} {parts[-1]}", img.crop(box)))
    return out


def _content_box(img):
    """Bounding box of what differs from the corner colour, with a margin."""
    bg = img.getpixel((0, 0))
    mask = Image.new("L", img.size, 0)
    px = img.load()
    mp = mask.load()
    w, h = img.size
    for y in range(0, h, 2):
        for x in range(0, w, 2):
            p = px[x, y]
            if sum(abs(a - b) for a, b in zip(p, bg, strict=True)) > 24:
                mp[x, y] = 255
    box = mask.getbbox() or (0, 0, w, h)
    m = 12
    return (
        max(box[0] - m, 0),
        max(box[1] - m, 0),
        min(box[2] + m, w),
        min(box[3] + m, h),
    )


def _grid(draw, sheet, tiles, top, cols, tile_h):
    tile_w = WIDTH // cols
    for i, (label, img) in enumerate(tiles):
        x = (i % cols) * tile_w
        y = top + (i // cols) * (tile_h + 16)
        scale = min((tile_w - 4) / img.width, tile_h / img.height)
        small = img.resize(
            (max(int(img.width * scale), 1), max(int(img.height * scale), 1))
        )
        sheet.paste(small, (x + (tile_w - small.width) // 2, y))
        draw.text((x + 3, y + tile_h + 1), label, font=font(11), fill=INK)
    rows = (len(tiles) + cols - 1) // cols
    return top + rows * (tile_h + 16)


def main():
    """Compose the sheet."""
    human_dir, cavalry_dir = sys.argv[1], sys.argv[2]
    out = sys.argv[3] if len(sys.argv) > 3 else OUT
    human = _tiles(human_dir)
    horse = _tiles(cavalry_dir, "clip_")
    h_rows = (len(human) + 6) // 7
    c_rows = (len(horse) + 3) // 4
    height = 30 + h_rows * (130 + 16) + 10 + c_rows * (120 + 16)
    sheet = Image.new("RGB", (WIDTH, height), PAPER)
    draw = ImageDraw.Draw(sheet)
    draw.text((6, 6), "AN1b — nouveaux clips (corps fin)", font=font(16), fill=INK)
    y = _grid(draw, sheet, human, 30, 7, 130)
    _grid(draw, sheet, horse, y + 10, 4, 120)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    sheet.save(out)
    print(out, sheet.size)


if __name__ == "__main__":
    main()
