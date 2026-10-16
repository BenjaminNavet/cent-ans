"""Refshots helpers: uniform-frame check and labelled Bevy montages (lot refshots).

The Godot side was removed (ADR 0366); a `reference/<name>.png` image, when present in the output
folder (an older Bevy run, a mock-up), is shown on the left of the Bevy shot.

Usage:
    python3 montage.py check <image.png>            exit 0 if the image has content, 1 if uniform/missing
    python3 montage.py montage <out_dir> <name>...  writes <out_dir>/montage/<name>.png
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageStat

HEIGHT = 450  # height of each half in the montage (two halves -> ~1440 px wide)
BAND = 28


def has_content(path: Path) -> bool:
    """True when the image exists and is not a uniform (black, blank) frame."""
    if not path.exists():
        return False
    with Image.open(path) as image:
        rgb = image.convert("RGB")
        stat = ImageStat.Stat(rgb)
        spread = max(high - low for low, high in rgb.getextrema())
        return spread > 24 and max(stat.stddev) > 4.0


def _font(size: int) -> ImageFont.ImageFont:
    for candidate in ("/System/Library/Fonts/Supplemental/Arial.ttf", "/Library/Fonts/Arial.ttf"):
        if Path(candidate).exists():
            return ImageFont.truetype(candidate, size)
    return ImageFont.load_default()


def _half(path: Path, label: str) -> Image.Image:
    if has_content(path):
        with Image.open(path) as image:
            source = image.convert("RGB")
        width = round(source.width * HEIGHT / source.height)
        body = source.resize((width, HEIGHT), Image.LANCZOS)
    else:
        body = Image.new("RGB", (round(HEIGHT * 1.6), HEIGHT), (60, 0, 0))
        label += " - ABSENTE / UNIFORME"
    half = Image.new("RGB", (body.width, HEIGHT + BAND), (20, 20, 20))
    half.paste(body, (0, BAND))
    ImageDraw.Draw(half).text((8, 4), label, fill=(240, 220, 160), font=_font(18))
    return half


def montage(out_dir: Path, name: str) -> Path:
    """Bevy shot (with the optional reference image on its left) into out_dir/montage/name.png."""
    right = _half(out_dir / "bevy" / f"{name}.png", f"Bevy - {name}")
    reference = out_dir / "reference" / f"{name}.png"
    if reference.exists():
        left = _half(reference, f"Reference - {name}")
        sheet = Image.new("RGB", (left.width + right.width + 6, left.height), (0, 0, 0))
        sheet.paste(left, (0, 0))
        sheet.paste(right, (left.width + 6, 0))
    else:
        sheet = right
    target = out_dir / "montage" / f"{name}.png"
    target.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(target)
    return target


def main() -> int:
    """Command-line entry point."""
    if len(sys.argv) >= 3 and sys.argv[1] == "check":
        return 0 if has_content(Path(sys.argv[2])) else 1
    if len(sys.argv) >= 4 and sys.argv[1] == "montage":
        for name in sys.argv[3:]:
            print(montage(Path(sys.argv[2]), name))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
