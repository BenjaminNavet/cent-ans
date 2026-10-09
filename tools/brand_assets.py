"""Génère l'icône d'application et l'image Open Graph de Cent Ans (lot QW-H).

Dessin : la lettrine « C » d'or sur azur de l'écran titre (`Lettrine`, `FrontEndStyle`), police
IM Fell English (bible DA § 4). Sorties : game/icon.png (1024), game/icon.icns, game/icon.ico
(16-256 px, versions simplifiées aux petites tailles) et docs/img/readme/og.png (1280x640).

Usage : uv run --project tools python tools/brand_assets.py
"""

import shutil
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT_ROMAN = ROOT / "game/assets/third_party/fonts/im_fell_english/IMFeENrm28P.ttf"
FONT_ITALIC = ROOT / "game/assets/third_party/fonts/im_fell_english/IMFeENit28P.ttf"
MENU = ROOT / "docs/img/readme/menu.jpg"

# Valeurs de game/scripts/ui/front_end_style.gd (flottants 0-1 -> octets).
GOLD = (237, 199, 107)
GOLD_DARK = (158, 117, 46)
AZURE = (26, 43, 107)
GULES = (148, 26, 20)
INK = (20, 10, 5)

ICO_SIZES = [16, 24, 32, 48, 64, 128, 256]
ICNS_SIZES = [16, 32, 64, 128, 256, 512, 1024]


def render_icon(size: int) -> Image.Image:
    """Lettrine « C » sur azur ; filet et points d'or retirés sous 48 px."""
    scale = 4  # sur-échantillonnage pour l'anticrénelage
    px = size * scale
    img = Image.new("RGBA", (px, px), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    radius = int(px * 0.18)
    margin = int(px * 0.02)
    box = (margin, margin, px - margin, px - margin)
    draw.rounded_rectangle(box, radius, fill=AZURE)
    border = max(int(px * (0.045 if size >= 48 else 0.07)), scale)
    draw.rounded_rectangle(box, radius, outline=GOLD, width=border)
    if size >= 48:
        inset = int(px * 0.095)
        draw.rounded_rectangle(
            (inset, inset, px - inset, px - inset),
            int(radius * 0.6),
            outline=GULES,
            width=max(int(px * 0.008), 1),
        )
        step = px / 8
        for row in range(1, 8):
            for col in range(1, 8):
                if (row + col) % 2:
                    continue
                cx, cy, d = col * step, row * step, px * 0.008
                draw.polygon(
                    [(cx, cy - d), (cx + d, cy), (cx, cy + d), (cx - d, cy)],
                    fill=(*GOLD, 110),
                )
    font = ImageFont.truetype(str(FONT_ROMAN), int(px * 0.86))
    left, top, right, bottom = font.getbbox("C")
    pos = (
        (px - (right - left)) / 2 - left,
        (px - (bottom - top)) / 2 - top + px * 0.01,
    )
    outline = max(int(px * 0.03), scale)
    draw.text(pos, "C", font=font, fill=GOLD, stroke_width=outline, stroke_fill=INK)
    draw.text(pos, "C", font=font, fill=GOLD)
    return img.resize((size, size), Image.LANCZOS)


def write_icons() -> None:
    """Écrit game/icon.png, .icns et .ico."""
    master = render_icon(1024)
    master.save(ROOT / "game/icon.png")
    master.save(ROOT / "game/icon.icns", sizes=[(s, s) for s in ICNS_SIZES])
    frames = [render_icon(s) for s in ICO_SIZES]
    frames[-1].save(
        ROOT / "game/icon.ico",
        format="ICO",
        sizes=[(s, s) for s in ICO_SIZES],
        append_images=frames[:-1],
    )
    shutil.copyfile(ROOT / "game/icon.ico", ROOT / "tools/launcher-windows/icon.ico")


def write_og() -> None:
    """1280x640 : écran titre recadré sur l'ost et la ville, lettrine, titre et accroche."""
    width, height = 1280, 640
    menu = Image.open(MENU).convert("RGB")
    # Recadrage 2:1 à droite du menu (le menu est cuit dans le jpg, x < 650) : ciel, ville, ost.
    left, top, crop_w = 600, 290, 1000
    base = menu.crop((left, top, left + crop_w, top + crop_w // 2)).resize(
        (width, height), Image.LANCZOS
    )
    # Voile sombre à gauche pour la lisibilité du texte.
    shade = Image.new("L", (width, height), 0)
    sd = ImageDraw.Draw(shade)
    for x in range(width):
        sd.line([(x, 0), (x, height)], fill=int(max(0, 235 * (1 - x / (width * 0.75)))))
    base = Image.composite(Image.new("RGB", (width, height), (10, 8, 6)), base, shade)
    canvas = base.convert("RGBA")
    draw = ImageDraw.Draw(canvas)

    letter = render_icon(220)
    canvas.alpha_composite(letter, (70, 120))
    title_font = ImageFont.truetype(str(FONT_ROMAN), 150)
    ent_pos = (312, 118)
    shadow = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).text(
        (ent_pos[0] + 4, ent_pos[1] + 5),
        "ent Ans",
        font=title_font,
        fill=(0, 0, 0, 200),
    )
    canvas.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(4)))
    draw = ImageDraw.Draw(canvas)
    draw.text(ent_pos, "ent Ans", font=title_font, fill=(250, 240, 214))
    draw.line([(76, 372), (720, 372)], fill=GOLD, width=2)
    draw.polygon([(398, 372 - 7), (405, 372), (398, 372 + 7), (391, 372)], fill=GULES)
    tag_font = ImageFont.truetype(str(FONT_ITALIC), 44)
    draw.text((76, 400), "La guerre de Cent Ans, 1337-1453", font=tag_font, fill=GOLD)
    sub_font = ImageFont.truetype(str(FONT_ITALIC), 32)
    draw.text(
        (76, 468),
        "Un jeu de grande stratégie libre,\ncampagne et batailles en 3D.",
        font=sub_font,
        fill=(236, 226, 200),
        spacing=8,
    )
    canvas.convert("RGB").save(ROOT / "docs/img/readme/og.png", optimize=True)


if __name__ == "__main__":
    write_icons()
    write_og()
