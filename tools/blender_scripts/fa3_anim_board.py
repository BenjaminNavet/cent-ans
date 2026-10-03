"""Lot FA3: compose the check boards from the ``fa3_anim_retarget.py -- render`` renders.

    uv run --project tools python tools/blender_scripts/fa3_anim_board.py RENDER_DIR BOARD_DIR

One JPEG per family (``fa3_<family>.jpg``, at most 1280 px wide). Each clip is a row: the
clip the game has (keyframed, and the NT14 default where there is one) on the left, the FA3
clip on the right, four poses each.
"""

import os
import sys

from fg0_planche import INK, PAPER, font
from PIL import Image, ImageDraw

WIDTH = 1280
TILE = (156, 208)
LABEL = 22
TAGS = {"k": "keyframé", "d": "défaut NT14", "f": "FA3"}


def _renders(directory):
    """``{family: {clip: {tag: [images]}}}`` of the renders of `directory`."""
    out = {}
    for name in sorted(os.listdir(directory)):
        if not name.endswith(".png") or name.count("__") != 3:
            continue
        family, clip, tag, _index = name[:-4].split("__")
        img = Image.open(os.path.join(directory, name)).convert("RGB")
        out.setdefault(family, {}).setdefault(clip, {}).setdefault(tag, []).append(
            img.resize(TILE, Image.LANCZOS)
        )
    return out


def _row(clip, left_tag, left, right):
    """One board row: `left` tiles, a gap, the FA3 tiles, with a caption line."""
    gap = WIDTH - TILE[0] * 8
    row = Image.new("RGB", (WIDTH, TILE[1] + LABEL), PAPER)
    draw = ImageDraw.Draw(row)
    draw.text((6, 3), f"{clip} — {TAGS[left_tag]}", fill=INK, font=font(15))
    draw.text(
        (TILE[0] * 4 + gap + 6, 3), f"{clip} — {TAGS['f']}", fill=INK, font=font(15)
    )
    for i, img in enumerate(left):
        row.paste(img, (i * TILE[0], LABEL))
    for i, img in enumerate(right):
        row.paste(img, (TILE[0] * 4 + gap + i * TILE[0], LABEL))
    return row


def main():
    """Compose one board per family of the renders."""
    renders, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    for family, clips in _renders(renders).items():
        rows = []
        for clip, tags in clips.items():
            for tag in ("k", "d"):
                if tag in tags:
                    rows.append(_row(clip, tag, tags[tag], tags["f"]))
        board = Image.new("RGB", (WIDTH, sum(r.height for r in rows)), PAPER)
        y = 0
        for row in rows:
            board.paste(row, (0, y))
            y += row.height
        path = os.path.join(out, f"fa3_{family}.jpg")
        board.save(path, quality=88)
        print("BOARD", path, board.size)


if __name__ == "__main__":
    main()
