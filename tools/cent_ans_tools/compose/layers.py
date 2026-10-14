"""Couches dérivées de la carte composée indépendantes des provinces (lot CC, C4a).

``cent-ans geo compose-layers`` rejoue, sur le canevas composé, les étapes de la chaîne qui ne
lisent ni provinces, ni lieux, ni routes (celles-là attendent C4b : provinces, graphe des lieux,
navgrid, ports, voies maritimes, routes). Ordre (chaque étape lit les sorties des précédentes) :

1. ``coast`` : ``coast_dist.png`` (distance signée à la côte) ;
2. ``biomes`` : ``biomes.png`` (Köppen rééchantillonné par la déformation inverse, filtre majoritaire),
   ``biomes_base.png``, ``biome_parents.json`` ; ``biome-blend`` : fondu des frontières ;
3. ``agri`` : ``agri_regions.png`` (paysages agricoles régionaux) ;
4. ``landcover`` : ``wetlands.png`` (+ copie GPU BC1), ``forest_kind.png`` et ``splat.png`` PROVISOIRE
   (sans provinces ni essarts autour des lieux ; refait par ``geo splat`` en C4b) ; masques d'auteur
   (``compose/masks``) appliqués en surcharge ;
5. ``rivers`` : ``rivers_render.json``, ``river_bed.png``, ``crossings_px.json`` ;
6. ``lakes`` : ``lakes.json`` (lacs Natural Earth fins et lacs historiques) ;
7. ``freshwater`` : ``freshwater_px.json`` (zones humides, salins, cascades, gués, torrents) ;
8. ``occlusion`` : ``relief_occlusion.png`` ;
9. ``colormap`` : ``colormap_bc1_*.bin`` + aperçu, PROVISOIRES (sans routes, villages ni halos de villes).

Le relief (neige, roche, lacs plats) vient de ``geo compose-relief`` ; les lacs fins du masque de
``geo compose``. Reconstruire dans l'ordre : ``geo compose`` -> ``geo compose-relief`` -> ``geo compose-layers``.
"""

from __future__ import annotations

import time
from collections.abc import Callable
from pathlib import Path

from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"


def _coast(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import splat

    return [splat.build_coast_dist(map_dir)]


def _biomes(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import biomes

    return biomes.build(map_dir)


def _biome_blend(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import biome_blend

    return list(biome_blend.build(map_dir))


def _agri(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import agri_regions

    return [agri_regions.build(map_dir)]


def _landcover(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import landcover

    result = landcover.build(map_dir, independent=True)
    return [result.splat, result.wetlands, result.forest_kind]


def _rivers(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import river_render

    result = river_render.build(map_dir, with_roads=False)
    return [result.render, result.bed, result.crossings]


def _lakes(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import lakes

    return [lakes.build().output]


def _freshwater(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import freshwater

    freshwater.build(map_dir)
    return [map_dir / freshwater.OUTPUT_FILE]


def _occlusion(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import relief_occlusion

    return [relief_occlusion.build(map_dir).path]


def _colormap(map_dir: Path) -> list[Path]:
    from cent_ans_tools.geo import colormap

    return colormap.build(map_dir, independent=True)


#: Étapes dans l'ordre de reconstruction.
STEPS: dict[str, Callable[[Path], list[Path]]] = {
    "coast": _coast,
    "biomes": _biomes,
    "biome-blend": _biome_blend,
    "agri": _agri,
    "landcover": _landcover,
    "rivers": _rivers,
    "lakes": _lakes,
    "freshwater": _freshwater,
    "occlusion": _occlusion,
    "colormap": _colormap,
}


def run(only: list[str] | None = None, map_dir: Path = MAP_DIR) -> dict[str, dict]:
    """Rejoue les étapes (toutes, ou ``only`` dans l'ordre de ``STEPS``) ; renvoie fichiers et durées."""
    unknown = [name for name in only or [] if name not in STEPS]
    if unknown:
        raise ValueError(f"étapes inconnues : {unknown} ; connues : {list(STEPS)}")
    report: dict[str, dict] = {}
    for name, step in STEPS.items():
        if only and name not in only:
            continue
        started = time.monotonic()
        paths = step(map_dir)
        report[name] = {
            "seconds": round(time.monotonic() - started, 1),
            "files": [p.name for p in paths],
        }
        print(f"[layers] {name} : {report[name]['seconds']} s", flush=True)
    return report


#: Couleurs de la planche de contrôle (aperçu hors dépôt, pas une donnée de jeu), par indice de biome.
BOARD_BIOME_COLORS = {
    0: (70, 110, 150),
    1: (96, 160, 80),
    2: (150, 170, 80),
    3: (190, 170, 90),
    4: (215, 200, 120),
    5: (40, 100, 90),
    6: (140, 130, 130),
    7: (225, 190, 130),
    8: (240, 220, 160),
    9: (190, 210, 200),
    10: (120, 150, 60),
    11: (70, 150, 110),
    12: (190, 190, 90),
    13: (80, 130, 90),
    14: (170, 150, 80),
}


def make_board(out_dir: Path, map_dir: Path = MAP_DIR, width: int = 1500) -> list[Path]:
    """Planche de contrôle (hors dépôt) : biomes, couverture du sol, colormap, eau.

    Écrit ``biomes.png``, ``landcover.png`` (splat : herbe verte, champs ocre, forêt vert sombre,
    roche grise ; marais bleu), ``colormap.png`` (aperçu JPEG de la colormap) et ``water.png``
    (lacs, fleuves rendus) dans ``out_dir`` ; renvoie les chemins.
    """
    import json

    import numpy as np
    from PIL import Image

    from cent_ans_tools.geo import terrain

    Image.MAX_IMAGE_PIXELS = None
    out_dir.mkdir(parents=True, exist_ok=True)
    land = np.asarray(Image.open(map_dir / "land_mask.png")) > 0
    size = (width, round(width * land.shape[0] / land.shape[1]))
    written = []

    def save(name: str, array: np.ndarray) -> None:
        image = Image.fromarray(array.astype(np.uint8)).resize(size, Image.BOX)
        path = out_dir / name
        image.save(path)
        written.append(path)

    biome = np.asarray(Image.open(map_dir / "biomes.png"))
    lut = np.zeros((256, 3), dtype=np.uint8)
    for index, color in BOARD_BIOME_COLORS.items():
        lut[index] = color
    save("biomes.png", lut[biome])
    height = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    shade = np.clip(0.75 + 0.25 * np.tanh(np.gradient(height, axis=0) / 25.0), 0.5, 1.0)

    splat = np.asarray(Image.open(map_dir / "splat.png")).astype(np.float32) / 255.0
    wet = np.asarray(Image.open(map_dir / "wetlands.png")).astype(np.float32) / 255.0
    if splat.shape[:2] != land.shape:
        splat = (
            np.asarray(
                Image.fromarray((splat * 255).astype(np.uint8), mode="RGBA").resize(
                    land.shape[::-1], Image.BILINEAR
                )
            ).astype(np.float32)
            / 255.0
        )
    palette = np.array(
        [[110, 160, 70], [205, 175, 90], [35, 85, 45], [140, 135, 125]],
        dtype=np.float32,
    )
    cover = splat[..., :4] @ palette
    cover = (
        cover * (1 - 0.6 * wet[..., :1]) + np.array([70, 120, 150]) * 0.6 * wet[..., :1]
    )
    cover = np.where(
        land[..., None], cover * shade[..., None], np.array([70, 110, 150])
    )
    save("landcover.png", cover)

    preview = map_dir / "colormap_preview.jpg"
    if preview.exists():
        image = Image.open(preview).convert("RGB").resize(size, Image.LANCZOS)
        path = out_dir / "colormap.png"
        image.save(path)
        written.append(path)

    water = np.where(
        land[..., None],
        np.array([150, 160, 120]) * shade[..., None],
        np.array([70, 110, 150]),
    )
    lakes = json.loads((map_dir / "lakes.json").read_text(encoding="utf-8"))["lakes"]
    scale = width / land.shape[1]
    canvas = Image.fromarray(water.astype(np.uint8)).resize(size, Image.BOX)
    from PIL import ImageDraw

    draw = ImageDraw.Draw(canvas)
    for lake in lakes:
        draw.polygon(
            [(x * scale, y * scale) for x, y in lake["polygon_px"]], fill=(20, 60, 190)
        )
    rivers = json.loads((map_dir / "rivers_render.json").read_text(encoding="utf-8"))[
        "rivers"
    ]
    for river in rivers:
        pts = [(x * scale, y * scale) for x, y in river["points"]]
        if len(pts) > 1:
            draw.line(pts, fill=(30, 80, 200), width=1)
    path = out_dir / "water.png"
    canvas.save(path)
    written.append(path)
    return written
