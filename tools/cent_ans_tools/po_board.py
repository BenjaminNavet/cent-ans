"""Before/after board of the PO polish slice (ADR 0097, criterion C5).

Pairs the views of two folders written by `game/tests/po_shot.gd` (same `NN-name.jpg` file
names) and writes one JPEG: one row per view, "avant" on the left, "après" on the right.

Usage:
    uv run --project tools python -m cent_ans_tools.po_board \
        docs/img/po/avant docs/img/po/apres docs/img/po/planche_avant_apres.jpg [--width 960]
"""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw

GUTTER = 8
LABEL_HEIGHT = 28
BACKGROUND = (38, 30, 22)
INK = (238, 226, 196)


def build_board(before_dir: Path, after_dir: Path, width: int) -> Image.Image:
    """Returns the side-by-side board of the views present in both folders."""
    names = sorted(
        p.name for p in before_dir.glob("*.jpg") if (after_dir / p.name).exists()
    )
    if not names:
        raise SystemExit(f"no common view in {before_dir} and {after_dir}")
    with Image.open(before_dir / names[0]) as first:
        height = round(first.height * width / first.width)
    row_height = LABEL_HEIGHT + height + GUTTER
    board = Image.new(
        "RGB", (2 * width + 3 * GUTTER, len(names) * row_height + GUTTER), BACKGROUND
    )
    draw = ImageDraw.Draw(board)
    for row, name in enumerate(names):
        top = GUTTER + row * row_height
        for column, (folder, caption) in enumerate(
            ((before_dir, "avant"), (after_dir, "après"))
        ):
            left = GUTTER + column * (width + GUTTER)
            draw.text((left, top + 6), f"{Path(name).stem} — {caption}", fill=INK)
            with Image.open(folder / name) as view:
                board.paste(
                    view.convert("RGB").resize((width, height), Image.LANCZOS),
                    (left, top + LABEL_HEIGHT),
                )
    return board


def main() -> None:
    """Command-line entry point."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument(
        "--width", type=int, default=960, help="width of each view in pixels"
    )
    args = parser.parse_args()
    board = build_board(args.before, args.after, args.width)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    board.save(args.out, quality=82)
    print(f"po_board: {args.out} ({board.width}×{board.height})")


if __name__ == "__main__":
    main()
