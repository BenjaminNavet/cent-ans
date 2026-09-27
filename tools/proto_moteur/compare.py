"""Assemble out/compare.png: full views and 1:1 details of the Godot and Unreal renders, labelled."""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent / "out"
DETAIL = (880, 430, 1400, 730)  # front rank, wall and cart at 1:1
renders = {"Godot 4.7 (Forward+, SDFGI)": Image.open(OUT / "godot.png").convert("RGB"), "Unreal 5.7 (Lumen, Nanite, VSM)": Image.open(OUT / "unreal.png").convert("RGB")}
half = (960, 540)
detail_size = (DETAIL[2] - DETAIL[0], DETAIL[3] - DETAIL[1])
sheet = Image.new("RGB", (half[0] * 2, half[1] + detail_size[1] + 40), (20, 20, 20))
draw = ImageDraw.Draw(sheet)
font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Bold.ttf", 28)
for column, (label, image) in enumerate(renders.items()):
    sheet.paste(image.resize(half, Image.LANCZOS), (column * half[0], 0))
    sheet.paste(image.crop(DETAIL), (column * half[0] + (half[0] - detail_size[0]) // 2, half[1] + 40))
    draw.text((column * half[0] + 16, half[1] + 6), label, fill=(240, 230, 210), font=font)
sheet.save(OUT / "compare.png")
print("PROTO compare.png written")
