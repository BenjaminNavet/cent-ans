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
