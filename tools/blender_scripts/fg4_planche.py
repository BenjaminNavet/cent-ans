"""Lot FG4: compose the check sheets from the renders of ``fg4_render.py``.

Run (Pillow from the tools project):

    uv run --project tools python tools/blender_scripts/fg4_planche.py <render dir>

Writes ``docs/img/fg/fg4_clips.png`` (every clip of the ``cavalry`` rig, several frames)
and ``docs/img/fg/fg4_chevaux.png`` (horse builds, robes, head and bridle, LODs).
"""

import json
import os
import sys

from fg0_planche import INK, PAPER, font, on_backdrop
from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT_DIR = os.path.join(ROOT, "docs", "img", "fg")

CLIP_LABELS = {
    "c_idle": "arrêt (c_idle)",
    "c_walk": "pas (c_walk)",
    "c_gallop": "galop (c_gallop)",
    "c_charge": "charge (c_charge)",
    "c_thrust": "mêlée, cheval qui piaffe (c_thrust)",
    "c_death": "cheval abattu (c_death)",
    "c_fall": "chute du cavalier (c_fall)",
}
TYPE_LABELS = {"destrier": "destrier", "rouncey": "roncin", "jennet": "genet"}
TILE = (400, 300)


def _sheet(title, subtitle, rows, tile=TILE, label_w=260):
    """rows: [(label, [(image path, caption)])] -> Image."""
    cols = max(len(r[1]) for r in rows)
    head = 110
    row_h = tile[1] + 34
    w = label_w + cols * (tile[0] + 10) + 30
    h = head + len(rows) * row_h + 20
    sheet = Image.new("RGB", (w, h), PAPER)
    d = ImageDraw.Draw(sheet)
    d.text((30, 18), title, font=font(40), fill=INK)
    d.text((30, 68), subtitle, font=font(20), fill=INK)
    for r, (label, items) in enumerate(rows):
        y = head + r * row_h
        d.text((30, y + tile[1] // 2 - 12), label, font=font(22), fill=INK)
        for c, (path, caption) in enumerate(items):
            x = label_w + c * (tile[0] + 10)
            if os.path.exists(path):
                sheet.paste(on_backdrop(path, tile), (x, y))
            if caption:
                d.text((x + 4, y + tile[1] + 4), caption, font=font(18), fill=INK)
    return sheet


def main():
    """Compose both sheets from the render directory given on the command line."""
    src = sys.argv[1]
    with open(os.path.join(src, "meta.json")) as f:
        meta = json.load(f)
    fig = meta["figure"]
    rows = []
    for clip, count in meta["clips"].items():
        items = [
            (os.path.join(src, f"clip_{fig}_{clip}_{k}.png"), f"image {k + 1}/{count}")
            for k in range(count)
        ]
        rows.append((CLIP_LABELS.get(clip, clip), items))
    _sheet(
        f"FG4 — cheval fin sur les clips du rig « cavalry » ({fig})",
        "Mêmes os, même texture d'os : poids calculés dans la pose naturelle, articulations "
        "posées sur les pivots des clips. Rendu Eevee, cavalier actuel (en attendant FG1).",
        rows,
    ).save(os.path.join(OUT_DIR, "fg4_clips.png"), optimize=True)
    types = [
        (
            os.path.join(src, f"type_{f}_{robe}.png"),
            f"{f} — {TYPE_LABELS.get(t, t)}, {robe}",
        )
        for f, robe, t in meta["types"]
    ]
    lods = [
        (os.path.join(src, f"lod_{fig}_{lv}.png"), f"LOD{lv} : {n} triangles")
        for lv, n in sorted(meta["lods"].items())
    ]
    rows = [
        ("montures et robes", types[:4]),
        ("", types[4:8]),
        ("tête, bride", [(os.path.join(src, f"head_{fig}.png"), "")] + lods),
    ]
    rows = [r for r in rows if r[1]]
    _sheet(
        "FG4 — montures, robes et niveaux de détail",
        "Destrier caparaçonné (chevaliers, gendarmes, étendard), roncin sans barde "
        "(sergents, archers, écorcheurs, hobelars), genet (jinetes). Robes : teinte du "
        "shader × ombrage par sommet.",
        rows,
    ).save(os.path.join(OUT_DIR, "fg4_chevaux.png"), optimize=True)
    print("OK")


if __name__ == "__main__":
    main()
