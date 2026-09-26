"""Lot FG0: assemble the style sheet from the renders of ``battle_fine_proto.py``.

Run (Pillow from the tools project):

    uv run --project tools python tools/blender_scripts/fg0_planche.py <render dir>

Writes ``docs/img/fg/planche_fg0.png`` and the separate images (neutral backdrop,
reduced size) into ``docs/img/fg/``.
"""

import os
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "docs", "img", "fg")
FONT = os.path.join(
    ROOT,
    "game",
    "assets",
    "third_party",
    "fonts",
    "eb_garamond",
    "EBGaramond-VariableFont_wght.ttf",
)
BACK = (118, 116, 110)  # neutral warm grey backdrop
PAPER = (236, 229, 212)
INK = (40, 30, 22)

# Figures, triangles per LOD (from the build logs of battle_fine_proto.py).
TRIS = {
    "infantry": {"actuel": (2374, 517, 230), "proto": (12291, 2400, 499)},
    "cavalry": {"actuel": (2956, 746, 370), "proto": (16440, 2796, 713)},
}


def font(size):
    """EB Garamond at `size` px (falls back to Pillow's default)."""
    try:
        return ImageFont.truetype(FONT, size)
    except OSError:
        return ImageFont.load_default()


def on_backdrop(path, size=None):
    """Render (transparent film) composited on the neutral backdrop, optionally fitted."""
    im = Image.open(path).convert("RGBA")
    bg = Image.new("RGBA", im.size, (*BACK, 255))
    bg.alpha_composite(im)
    out = bg.convert("RGB")
    if size:
        out.thumbnail(size, Image.LANCZOS)
    return out


def far_crop(path, box=(260, 300), zoom=3):
    """Crop around the figure of a 1920x1080 game view and enlarge it (nearest)."""
    im = Image.open(path).convert("RGBA")
    x0, y0, x1, y1 = im.getbbox()
    cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
    w, h = box[0] // zoom, box[1] // zoom
    crop = im.crop((cx - w // 2, cy - h // 2, cx + w // 2, cy + h // 2))
    bg = Image.new("RGBA", crop.size, (*BACK, 255))
    bg.alpha_composite(crop)
    return bg.convert("RGB").resize(box, Image.NEAREST)


def label(draw, xy, text, size=22, fill=INK):
    """Text at `xy`."""
    draw.text(xy, text, font=font(size), fill=fill)


def main():
    """Build the sheet and the separate images."""
    src = sys.argv[1]
    os.makedirs(OUT_DIR, exist_ok=True)
    cell = (300, 420)
    head = (300, 300)
    W = 60 + 6 * (cell[0] + 12) + 40
    H = 2400
    sheet = Image.new("RGB", (W, H), PAPER)
    d = ImageDraw.Draw(sheet)
    label(d, (60, 30), "FG0 — planche de style : figurines fines (prototype)", 48)
    label(
        d,
        (60, 92),
        "Actuel (Quaternius, couleurs par sommet) contre prototype (MakeHuman CC0 ajusté au rig "
        "« human », cheval OpenGameArt CC0, normal/ORM/masque cuits). Eevee, lumière de jour douce.",
        22,
    )
    y = 150
    for fig, title in (
        ("infantry", "Homme d'armes (infantry_0)"),
        ("cavalry", "Chevalier monté (cavalry_0)"),
    ):
        label(d, (60, y), title, 34)
        y += 50
        x = 60
        row_h = 0
        for view, name, size in (
            ("face", "de face", cell),
            ("trois_quarts", "de 3/4", cell),
            ("tete", "tête", head),
        ):
            for kind, tag in (("current", "actuel"), ("proto", "prototype")):
                path = os.path.join(src, f"{kind}_{fig}_{view}.png")
                im = on_backdrop(path, size)
                sheet.paste(im, (x, y + 30))
                row_h = max(row_h, im.height)
                label(d, (x, y), f"{tag} — {name}", 20)
                im.save(os.path.join(OUT_DIR, f"{kind}_{fig}_{view}.png"))
                x += cell[0] + 12
        y += row_h + 60
        t = TRIS[fig]
        label(
            d,
            (60, y - 10),
            f"Triangles LOD0 / LOD1 / LOD2 — actuel : {t['actuel'][0]} / {t['actuel'][1]} / "
            f"~{t['actuel'][2]} ; prototype : {t['proto'][0]} / {t['proto'][1]} / {t['proto'][2]}",
            22,
        )
        y += 40
    # 30 m views (LOD1, the level shown beyond the 24 m relay), enlarged x3.
    label(
        d,
        (60, y),
        "Vue de jeu à ~30 m (LOD1, 1920×1080, 70°), agrandie ×3 (cavalier ×2)",
        34,
    )
    y += 50
    x = 60
    for fig in ("infantry", "cavalry"):
        for kind, tag in (("current", "actuel"), ("proto", "prototype")):
            path = os.path.join(src, f"{kind}_{fig}_lod1_30m_full.png")
            im = far_crop(path, (300, 360), zoom=2 if fig == "cavalry" else 3)
            sheet.paste(im, (x, y + 30))
            label(
                d,
                (x, y),
                f"{tag} — {'fantassin' if fig == 'infantry' else 'cavalier'}",
                20,
            )
            on_backdrop(path).save(os.path.join(OUT_DIR, f"{kind}_{fig}_30m.png"))
            x += 312
    # Baked maps of the man-at-arms.
    for key, name in (
        ("normal", "normale"),
        ("orm", "ORM (AO, rugosité, métal)"),
        ("mask", "masque (livrée, armoiries, peau)"),
    ):
        im = Image.open(os.path.join(src, f"infantry_{key}.png")).convert("RGB")
        im.thumbnail((170, 170))
        sheet.paste(im, (x, y + 30))
        label(d, (x, y), name.split(" ")[0], 20)
        im.save(os.path.join(OUT_DIR, f"atlas_infantry_{key}.png"))
        x += 182
    y += 420
    # Deformation check on the unchanged bone texture clips.
    label(
        d,
        (60, y),
        "Déformation sur les clips existants (mêmes os, même texture d'os)",
        34,
    )
    y += 50
    x = 60
    clips = [
        ("proto_infantry_clip_walk.png", "marche"),
        ("proto_infantry_clip_thrust.png", "estoc"),
        ("proto_infantry_clip_bow_shoot.png", "arc long"),
        ("proto_infantry_clip_death.png", "mort"),
        ("horse_c_walk_1.png", "cheval : pas"),
        ("horse_c_gallop_1.png", "cheval : galop"),
    ]
    for name, text in clips:
        path = os.path.join(src, name)
        im = on_backdrop(path, (300, 300))
        sheet.paste(im, (x, y + 30))
        label(d, (x, y), text, 20)
        im.save(os.path.join(OUT_DIR, f"clip_{name}"))
        x += 312
    y += 360
    sheet = sheet.crop((0, 0, W, y + 20))
    sheet.save(os.path.join(OUT_DIR, "planche_fg0.png"), optimize=True)
    print("SHEET", os.path.join(OUT_DIR, "planche_fg0.png"), sheet.size)


if __name__ == "__main__":
    main()
