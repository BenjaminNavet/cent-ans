"""Compose the boot splash: painted scene (local Z-Image Turbo) plus a programmatic title cartouche.

The scene is generated separately with mflux (ADR 0190, see docs/pipeline-assets-3d.md); this
script crops it to 16:9, resizes it to 1600x900, adds a soft vignette and draws the title in
EB Garamond inside a parchment cartouche. No text comes from the model.

Usage:
    uv run --project tools python tools/experiments/boot_splash.py SCENE.png OUT.png
"""

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[2]
FONT_PATH = (
    ROOT / "game/assets/third_party/fonts/eb_garamond/EBGaramond-VariableFont_wght.ttf"
)
SIZE = (1600, 900)
TITLE = "Cent Ans"

PARCHMENT = (236, 222, 186)
INK = (46, 32, 20)
GOLD = (176, 132, 52)


def fit_scene(scene: Image.Image, painted_height: int | None = None) -> Image.Image:
    """Center-crop to 16:9 and resize to the splash size.

    `painted_height` drops the blank strip the model sometimes leaves at the bottom.
    """
    if painted_height:
        scene = scene.crop((0, 0, scene.width, painted_height))
    width, height = scene.size
    target_ratio = SIZE[0] / SIZE[1]
    if width / height > target_ratio:
        new_width = round(height * target_ratio)
        left = (width - new_width) // 2
        scene = scene.crop((left, 0, left + new_width, height))
    else:
        new_height = round(width / target_ratio)
        top = (height - new_height) // 2
        scene = scene.crop((0, top, width, top + new_height))
    return scene.resize(SIZE, Image.LANCZOS).convert("RGB")


def add_vignette(image: Image.Image) -> Image.Image:
    """Darken the edges and the bottom slightly so the cartouche reads."""
    mask = Image.new("L", SIZE, 0)
    draw = ImageDraw.Draw(mask)
    draw.rectangle((-200, -200, SIZE[0] + 200, SIZE[1] + 200), fill=0)
    draw.ellipse((120, 60, SIZE[0] - 120, SIZE[1] - 40), fill=255)
    mask = mask.filter(ImageFilter.GaussianBlur(160))
    dark = Image.new("RGB", SIZE, (18, 12, 6))
    return Image.composite(image, dark, mask.point(lambda v: 110 + v * 145 // 255))


def draw_cartouche(image: Image.Image) -> Image.Image:
    """Draw the title in a parchment banner at the bottom center."""
    font = ImageFont.truetype(str(FONT_PATH), 112)
    font.set_variation_by_name("Medium")
    probe = ImageDraw.Draw(image)
    left, top, right, bottom = probe.textbbox((0, 0), TITLE, font=font)
    text_w, text_h = right - left, bottom - top
    pad_x, pad_y = 90, 38
    box_w, box_h = text_w + 2 * pad_x, text_h + 2 * pad_y
    x0 = (SIZE[0] - box_w) // 2
    y0 = SIZE[1] - box_h - 70
    x1, y1 = x0 + box_w, y0 + box_h

    shadow = Image.new("RGBA", SIZE, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rectangle(
        (x0 + 8, y0 + 12, x1 + 8, y1 + 12), fill=(0, 0, 0, 150)
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(12))
    image = Image.alpha_composite(image.convert("RGBA"), shadow).convert("RGB")

    draw = ImageDraw.Draw(image)
    draw.rectangle((x0, y0, x1, y1), fill=PARCHMENT, outline=INK, width=5)
    draw.rectangle((x0 + 12, y0 + 12, x1 - 12, y1 - 12), outline=GOLD, width=2)
    for corner_x, corner_y in ((x0, y0), (x1, y0), (x0, y1), (x1, y1)):
        draw.ellipse(
            (corner_x - 11, corner_y - 11, corner_x + 11, corner_y + 11),
            fill=GOLD,
            outline=INK,
            width=3,
        )
    draw.text((x0 + pad_x - left, y0 + pad_y - top), TITLE, font=font, fill=INK)
    return image


def main() -> None:
    """Parse arguments and write the composed splash."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scene", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--painted-height", type=int, default=None)
    args = parser.parse_args()
    image = add_vignette(fit_scene(Image.open(args.scene), args.painted_height))
    draw_cartouche(image).save(args.output, optimize=True)


if __name__ == "__main__":
    main()
