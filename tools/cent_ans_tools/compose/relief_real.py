"""Relief réel déformé de la carte composée (lot CC CD, ADR 0355 addendum) : ``cent-ans geo compose-relief-real``.

Décision du 14/10 : les massifs ne sont plus procéduraux. Pour chaque pixel composé, la
déformation inverse (``warp.json``) donne la position réelle ; la hauteur vient du MNT réel
(cache de la pyramide réelle E0-E4 : ETOPO + Copernicus GLO-90/30, re-téléchargeable par
``geo pyramid``). On retrouve de vraies vallées ramifiées, des arêtes, des cirques.

La géométrie d'auteur (C2b, ``compose/ridges.json``) reste une CONTRAINTE, pas la source de la forme :

- **plancher sur les crêtes d'auteur** : une bande étroite le long des chaînes ``massif`` est relevée
  à ``crest.floor_frac`` de la hauteur d'auteur, pour que le mur reste continu là où le réel est bas ;
- **cols en encoche** de ``crossings.json`` (``relief.carve_passes``) ;
- **plaine** : plancher de rampe côtière, côtes du masque composé, lacs plats, plateformes sous les lieux.

Les hauteurs écrites sont VRAIES (pas d'exagération cuite) : l'exagération façon RE (ADR 0353) est
appliquée à l'exécution par le profil ``relief_exaggeration.tres``, comme sur la carte réelle. La
règle ``massif_impassable`` lit ``heightmap.png`` : ``geo navgrid`` est à rejouer après cette commande.

Sorties : ``heightmap.png``, ``heightmap_render.png``, ``height/`` (E0), ``relief_shade``, ``map.json``
(comme ``compose-relief``) et la pyramide fine ``data/map/pyramid/`` E1-E3 (hors git).
"""

from __future__ import annotations

import gc
import json
import os
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.compose import erode
from cent_ans_tools.compose import relief as author_relief
from cent_ans_tools.compose.contract import COMPOSE_DIR, ComposedWarp
from cent_ans_tools.compose.warp import MAP_DIR
from cent_ans_tools.geo import terrain

Image.MAX_IMAGE_PIXELS = None

CONFIG_FILE = "relief_real.json"
TILE_PX = 512
NOMINAL_M = 719.0
LAND_MIN_M = 0.5
#: Plafond de sécurité d'une mosaïque réelle lue pour une fenêtre (pixels) : ~ 1 Go en float32 x 3 copies.
MAX_MOSAIC_PX = 16_000_000
REAL_ENV = "CENT_ANS_REAL_RELIEF"
DERIVED_DIR = COMPOSE_DIR / "derived"


def peak_rss_gb() -> float:
    """Pic de mémoire résidente du processus (Go) ; ``ru_maxrss`` est en octets sur macOS, en Ko sous Linux."""
    import resource
    import sys

    peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return peak / (1 << 30) if sys.platform == "darwin" else peak / (1 << 20)


def load_config(path: Path | None = None) -> dict:
    """``data/map/compose/relief_real.json``."""
    return json.loads((path or COMPOSE_DIR / CONFIG_FILE).read_text("utf-8"))


# ----------------------------------------------------------------------------
# Pyramide réelle (lecture)
# ----------------------------------------------------------------------------


class RealPyramid:
    """Lecture du cache de relief réel (E0 ``height/``, E1+ ``pyramid/E{k}/``), cadre monde 28 x 24 tuiles."""

    def __init__(self, root: Path) -> None:
        """``root`` : dossier ``data/map`` d'une carte réelle (contient ``height/`` et ``pyramid/``)."""
        self.root = Path(root)
        meta = json.loads((self.root / "relief_pyramid.json").read_text("utf-8"))
        self.h_min = float(meta["height_min_m"])
        self.h_range = float(meta["height_range_m"])
        self._cache: dict[tuple[int, int, int], np.ndarray | None] = {}

    @staticmethod
    def find_root(candidates: list[str]) -> Path:
        """Premier dossier candidat (ou ``$CENT_ANS_REAL_RELIEF``) qui porte la pyramide réelle."""
        paths = [os.environ.get(REAL_ENV, "")] + candidates
        for raw in paths:
            if not raw:
                continue
            root = Path(raw).expanduser()
            if (root / "pyramid").is_dir() and (root / "height").is_dir():
                return root
        raise FileNotFoundError(
            "pyramide de relief réel introuvable : installer le cache (`geo relief-fetch`/`geo pyramid`) "
            f"ou définir ${REAL_ENV} (candidats : {paths})"
        )

    def path(self, level: int, col: int, row: int) -> Path:
        """Chemin d'une tuile."""
        if level == 0:
            return self.root / "height" / f"h_{col}_{row}.png"
        return self.root / "pyramid" / f"E{level}" / f"{col}_{row}.png"

    def tile(self, level: int, col: int, row: int) -> np.ndarray | None:
        """Tuile décodée en mètres, ou ``None`` si absente."""
        key = (level, col, row)
        if key not in self._cache:
            path = self.path(level, col, row)
            value = None
            if path.exists():
                raw = terrain.read_png16(path).astype(np.float32)
                value = raw * (self.h_range / 65535.0) + self.h_min
            if len(self._cache) > 96:
                self._cache.clear()
            self._cache[key] = value
        return self._cache[key]

    def has(self, level: int, col: int, row: int) -> bool:
        """Tuile présente sur le disque."""
        return self.path(level, col, row).exists()

    def mosaic(self, level: int, x0: int, y0: int, x1: int, y1: int) -> np.ndarray:
        """Pixels ``[x0, x1) x [y0, y1)`` de l'étage ``level`` ; trous remplis par l'étage plus grossier."""
        out = np.full((y1 - y0, x1 - x0), np.nan, dtype=np.float32)
        for row in range(max(y0, 0) // TILE_PX, max(-(-y1 // TILE_PX), 0)):
            for col in range(max(x0, 0) // TILE_PX, max(-(-x1 // TILE_PX), 0)):
                tile = self.tile(level, col, row)
                if tile is None:
                    continue
                tx0, ty0 = col * TILE_PX, row * TILE_PX
                ax0, ay0 = max(x0, tx0), max(y0, ty0)
                ax1, ay1 = min(x1, tx0 + TILE_PX), min(y1, ty0 + TILE_PX)
                if ax1 <= ax0 or ay1 <= ay0:
                    continue
                out[ay0 - y0 : ay1 - y0, ax0 - x0 : ax1 - x0] = tile[
                    ay0 - ty0 : ay1 - ty0, ax0 - tx0 : ax1 - tx0
                ]
        if np.isnan(out).any():
            if level == 0:
                return np.nan_to_num(out, nan=0.0)
            cx0, cy0 = x0 // 2 - 2, y0 // 2 - 2
            coarse = self.mosaic(level - 1, cx0, cy0, x1 // 2 + 3, y1 // 2 + 3)
            up = ndimage.zoom(coarse, 2, order=1, mode="nearest", grid_mode=True)
            fill = up[
                y0 - 2 * cy0 : y0 - 2 * cy0 + out.shape[0],
                x0 - 2 * cx0 : x0 - 2 * cx0 + out.shape[1],
            ]
            out = np.where(np.isnan(out), fill, out)
        return out

    def coverage(self, level: int, x0: int, y0: int, x1: int, y1: int) -> float:
        """Part des tuiles de la fenêtre réellement présentes à cet étage (0-1)."""
        cols = range(max(x0, 0) // TILE_PX, max(-(-x1 // TILE_PX), 1))
        rows = range(max(y0, 0) // TILE_PX, max(-(-y1 // TILE_PX), 1))
        total = len(cols) * len(rows)
        present = sum(self.has(level, c, r) for r in rows for c in cols)
        return present / max(total, 1)


def sample_real(
    real: RealPyramid,
    warp: ComposedWarp,
    level: int,
    x0: int,
    y0: int,
    width: int,
    height: int,
    real_level: int | None = None,
) -> np.ndarray:
    """Hauteur réelle sous la fenêtre composée de l'étage composé ``level`` (pixels ``[x0, x0+width) x ...``).

    Étage composé L : ``2^(L+1)`` pixels par pixel de carte ; il lit l'étage réel L + 1 (un pixel
    composé couvre ≈ 0,85 pixel réel de carte, donc deux pixels réels de l'étage suivant).
    """
    per_unit = float(1 << (level + 1))
    r_level = min(level + 1 if real_level is None else real_level, 7)
    xs = (x0 + np.arange(width) + 0.5) / per_unit
    ys = (y0 + np.arange(height) + 0.5) / per_unit
    gx, gy = np.meshgrid(xs, ys)
    rx_u, ry_u = warp.inverse_px(gx, gy)
    del gx, gy
    valid = np.isfinite(rx_u) & np.isfinite(ry_u)
    if not valid.any():
        return np.full((height, width), -50.0, dtype=np.float32)
    # Hors canevas (NaN) : repli sur la moyenne des positions valides (sinon la boîte englobante
    # s'étendrait jusqu'à l'origine et la mosaïque lue ferait des Go).
    rx_u = np.where(valid, rx_u, np.mean(rx_u[valid]))
    ry_u = np.where(valid, ry_u, np.mean(ry_u[valid]))
    # Bords très comprimés du cartogramme : la fenêtre couvre beaucoup de pixels réels ; on lit alors
    # un étage réel plus grossier (4 fois moins de pixels par cran) jusqu'à tenir sous le plafond.
    while True:
        r_per_unit = float(1 << (r_level + 1))
        rx, ry = rx_u * r_per_unit - 0.5, ry_u * r_per_unit - 0.5
        mx0, my0 = int(np.floor(rx.min())) - 3, int(np.floor(ry.min())) - 3
        mx1, my1 = int(np.ceil(rx.max())) + 4, int(np.ceil(ry.max())) + 4
        if (mx1 - mx0) * (my1 - my0) <= MAX_MOSAIC_PX or r_level == 0:
            break
        r_level -= 1
    mosaic = real.mosaic(r_level, mx0, my0, mx1, my1)
    return ndimage.map_coordinates(
        mosaic, [ry - my0, rx - mx0], order=3, mode="nearest"
    ).astype(np.float32)


def real_coverage(
    real: RealPyramid,
    warp: ComposedWarp,
    level: int,
    x0: int,
    y0: int,
    width: int,
    height: int,
) -> float:
    """Part des tuiles réelles présentes (étage L + 1) sous une fenêtre composée (grille 5 x 5 de points)."""
    per_unit = float(1 << (level + 1))
    r_level = min(level + 1, 7)
    r_per = float(1 << (r_level + 1))
    gx, gy = np.meshgrid(
        (x0 + np.linspace(0, width, 5)) / per_unit,
        (y0 + np.linspace(0, height, 5)) / per_unit,
    )
    rx, ry = warp.inverse_px(gx, gy)
    ok = np.isfinite(rx) & np.isfinite(ry)
    if not ok.any():
        return 0.0
    cols = (rx[ok] * r_per / TILE_PX).astype(int)
    rows = (ry[ok] * r_per / TILE_PX).astype(int)
    return float(
        np.mean(
            [real.has(r_level, int(c), int(r)) for c, r in zip(cols, rows, strict=True)]
        )
    )


# ----------------------------------------------------------------------------
# Contraintes d'auteur (à la résolution de la carte)
# ----------------------------------------------------------------------------


def author_height(
    cw: ComposedWarp, rcfg: dict, scratch: Path
) -> tuple[np.ndarray, author_relief.Inputs]:
    """Relief d'auteur (C2b) du canevas, en cache ``derived/author_height.npy`` (invalidé si les réglages changent)."""
    inp = author_relief.load_inputs(cw, rcfg, inside_cache=scratch / "inside.npy")
    stamp = str(
        (COMPOSE_DIR / "relief.json").stat().st_mtime_ns
        + (COMPOSE_DIR / "ridges.json").stat().st_mtime_ns
    )
    cache = scratch / "author_height.npy"
    stamp_file = scratch / "author_height.stamp"
    if cache.exists() and stamp_file.exists() and stamp_file.read_text() == stamp:
        return np.load(cache), inp
    h, _ = author_relief.build_height(inp, rcfg)
    np.save(cache, h)
    stamp_file.write_text(stamp)
    return h, inp


def pad_to(array: np.ndarray, shape: tuple[int, int], mode: str = "edge") -> np.ndarray:
    """Complète ``array`` à ``shape`` (rows, cols)."""
    return np.pad(
        array,
        ((0, shape[0] - array.shape[0]), (0, shape[1] - array.shape[1])),
        mode=mode,
    )


def crest_floor(
    inp: author_relief.Inputs, author: np.ndarray, real_b: np.ndarray, cfg: dict
) -> np.ndarray:
    """Plancher des crêtes d'auteur ``massif`` : bande étroite le long de la polyligne.

    Hauteur du plancher : ``floor_real_frac`` du sommet RÉEL voisin (maximum sur ``peak_radius_px``,
    jamais au-dessus du réel) ou, pour garder la règle des massifs, ``floor_author_frac`` x hauteur
    d'auteur plafonnée à ``floor_author_cap_m``.
    """
    c = cfg["crest"]
    out = np.zeros(author.shape, dtype=np.float32)
    size = 2 * int(c["peak_radius_px"]) + 1
    peak = ndimage.maximum_filter(real_b, size=size)
    targets: dict[
        float, np.ndarray
    ] = {}  # plafond d'auteur -> cible (la plupart des chaînes : celui de cfg)
    for ridge in inp.ridges:
        if ridge["kind"] != "massif":
            continue
        if not cfg.get("chains", {}).get(ridge["id"], {}).get("crest", True):
            continue  # chaîne traitée par gain réel (`chain_gain_fields`), sans mur de crête
        # Lot CC CS : une chaîne peut élargir sa bande (`crest_band_px`, `crest_fade_px`) et baisser
        # le plancher d'auteur (`crest_floor_cap_m`) : l'Apennin est un massif large, pas un mur.
        band_px = float(ridge.get("crest_band_px", c["band_px"]))
        fade_px = float(ridge.get("crest_fade_px", c["fade_px"]))
        cap = float(ridge.get("crest_floor_cap_m", c["floor_author_cap_m"]))
        if cap not in targets:
            ceiling = np.minimum(c["floor_author_frac"] * author, cap)
            targets[cap] = np.maximum(c["floor_real_frac"] * peak, ceiling)
        target = targets[cap]
        reach = band_px + fade_px + 2.0
        pts = author_relief.smooth_points(ridge["points"], 2)
        window = author_relief._window(pts, reach, author.shape)
        x0, y0, x1, y1 = window
        if x1 <= x0 or y1 <= y0:
            continue
        field = author_relief.polyline_field(pts, window)
        band = 1.0 - author_relief.smoothstep(
            (field.dist - band_px) / max(fade_px, 1e-3)
        )
        view = out[y0:y1, x0:x1]
        np.maximum(view, (band * target[y0:y1, x0:x1]).astype(np.float32), out=view)
    return out


def chain_gain_fields(
    inp: author_relief.Inputs, shape: tuple[int, int], cfg: dict
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Gain de relief réel par chaîne (lot CC CS) : champs ``gain`` (g - 1, pondéré), ``socle`` et ``span`` (m).

    Le plancher de crête (``crest_floor``) fabrique un mur lisse qui ignore la vraie topographie. Pour
    un massif réel modéré (Apennin 1500-2900 m) on préfère amplifier le RELIEF RÉEL au-dessus d'un
    socle : ``h' = h + (g - 1) x clip(h - socle, 0, span_m) x w`` ; ``w`` vaut 1 à moins de ``core_px`` de la
    polyligne de la chaîne et tombe en douceur (smoothstep) à 0 à ``influence_px``. Vallées et arêtes
    réelles sont conservées (la forme vient du MNT), seule l'amplitude croît ; la plaine (sous le socle)
    reste intacte. Réglages : ``cfg["chains"][id]`` = ``gain``, ``socle_m``, ``core_px``, ``influence_px``.
    """
    gain = np.zeros(shape, dtype=np.float32)
    socle = np.zeros(shape, dtype=np.float32)
    span = np.full(shape, 1e4, dtype=np.float32)
    by_id = {r["id"]: r for r in inp.ridges}
    for rid, spec in cfg.get("chains", {}).items():
        if "gain" not in spec or rid not in by_id:
            continue
        core, reach = float(spec["core_px"]), float(spec["influence_px"])
        pts = author_relief.smooth_points(by_id[rid]["points"], 2)
        window = author_relief._window(pts, reach + 2.0, shape)
        x0, y0, x1, y1 = window
        if x1 <= x0 or y1 <= y0:
            continue
        field = author_relief.polyline_field(pts, window)
        weight = 1.0 - author_relief.smoothstep(
            (field.dist - core) / max(reach - core, 1e-3)
        )
        amount = (weight * (float(spec["gain"]) - 1.0)).astype(np.float32)
        view = gain[y0:y1, x0:x1]
        better = amount > view
        view[better] = amount[better]
        socle[y0:y1, x0:x1][better] = float(spec["socle_m"])
        span[y0:y1, x0:x1][better] = float(spec.get("span_m", 1e4))
    return gain, socle, span


def apply_gain(
    real: np.ndarray, gain: np.ndarray, socle: np.ndarray, span: np.ndarray
) -> np.ndarray:
    """``real + gain x clip(real - socle, 0, span)`` (gain nul : identité)."""
    return real + gain * np.clip(real - socle, 0.0, span)


def write_author_mass(
    author: np.ndarray, land: np.ndarray, size: tuple[int, int]
) -> Path:
    """Masse des chaînes d'auteur (règle ``massif_impassable`` sur le relief d'auteur) -> ``compose/author_mass.png``.

    Le MNT réel est plus étroit et plus bas que les chaînes d'auteur (Carpates, Apennins...) : la règle
    appliquée aux seules hauteurs réelles ne mure plus ce que C4a avait muré. Le masque d'auteur (une case
    de navgrid, 255 = masse) est donc conservé tel quel et réuni (OU) au masque calculé sur le relief réel
    par ``geo navgrid`` (``navgrid.authored_mass``). Données versionnées, petites (~ 20 Ko).
    """
    from cent_ans_tools.geo import navgrid

    scale = navgrid.NAVGRID_SCALE
    rows, cols = size[1] // scale * scale, size[0] // scale * scale
    small = (
        author[:rows, :cols]
        .reshape(rows // scale, scale, cols // scale, scale)
        .mean(axis=(1, 3))
    )
    water = (~land[:rows, :cols]).reshape(
        rows // scale, scale, cols // scale, scale
    ).mean(axis=(1, 3)) > 0.5
    mass = navgrid.massif_mask(small, water, navgrid.load_rules())
    path = COMPOSE_DIR / navgrid.AUTHORED_MASS
    Image.fromarray((mass * 255).astype(np.uint8), mode="L").save(path, optimize=True)
    return path


def constrain_base(
    real_b: np.ndarray,
    land: np.ndarray,
    author: np.ndarray,
    inp: author_relief.Inputs,
    inside: np.ndarray,
    rcfg: dict,
    cfg: dict,
) -> tuple[dict[str, np.ndarray], dict]:
    """Contraintes d'auteur à la résolution de la carte, sous forme de champs ADDITIFS appliqués au relief réel fin.

    ``ramp`` : rampe côtière (plancher de la terre, <= ``lowland_cap_m``) ; ``delta`` : correction additive lisse
    (plancher des crêtes d'auteur, encoches de cols, plateformes sous les lieux). Le relief fin devient
    ``max(réel, ramp) + delta`` : le détail réel reste PAR-DESSUS les contraintes (pas de dalle plate).

    Mémoire (lot CC CS) : tout se fait en place sur un seul raster de travail ``h`` et les temporaires
    sont libérés dès qu'ils ne servent plus (pic ~ 4 rasters de la carte au lieu de ~ 12).
    """
    report: dict = {}
    ramp = np.where(land, np.minimum(author, cfg["lowland_cap_m"]), -9000.0).astype(
        np.float32
    )
    h0 = np.where(land, np.maximum(real_b, ramp), author).astype(np.float32)
    crest = crest_floor(inp, author, real_b, cfg)
    h = np.where(land, np.maximum(crest, h0), h0).astype(np.float32)  # h0 + lift
    del crest
    report["crest_lifted_px"] = int((h - h0 > 1.0).sum())
    valley_floor = ndimage.minimum_filter(h, 25)
    valley_floor = ndimage.gaussian_filter(valley_floor, 12.0)
    pass_cfg = {**rcfg, "passes": {**rcfg["passes"], **cfg["passes"]}}
    report["passes"] = len(author_relief.carve_passes(h, valley_floor, inp, pass_cfg))
    # Fleuves : le tracé composé n'est pas pile au fond de la vallée réelle ; lit monotone aux plaines et collines.
    d_land = ndimage.distance_transform_edt(land).astype(np.float32)
    report["rivers"] = len(
        author_relief.carve_rivers(h, land, d_land, inp, rcfg, valley_floor)
    )
    del d_land, valley_floor
    report["platforms"] = author_relief.flatten_platforms(h, land, inp.places, rcfg)
    h -= h0
    del h0
    h[~land] = 0.0
    delta = ndimage.gaussian_filter(h, cfg["delta_smooth_px"]).astype(np.float32)
    del h
    report["delta_range_m"] = [
        round(float(delta.min()), 1),
        round(float(delta.max()), 1),
    ]
    return {"ramp": ramp, "delta": delta}, report


# ----------------------------------------------------------------------------
# Fenêtres fines (processus de calcul)
# ----------------------------------------------------------------------------

_CTX: dict = {}


def init_worker(ctx_paths: dict) -> None:
    """Ouvre en lecture seule (mmap) les rasters de la carte et la pyramide réelle."""
    _CTX.clear()
    _CTX["cfg"] = ctx_paths["cfg"]
    _CTX["e0_dir"] = ctx_paths.get("e0_dir", "")
    _CTX["real"] = RealPyramid(Path(ctx_paths["real_root"]))
    _CTX["warp"] = ComposedWarp.load(COMPOSE_DIR)
    for key in ("ramp", "delta", "land", "author", "lake", "gain", "socle", "span"):
        if key in ctx_paths:
            _CTX[key] = np.load(ctx_paths[key], mmap_mode="r")


def fine_window(level: int, x0: int, y0: int, width: int, height: int) -> np.ndarray:
    """Hauteur finale (m, vraie) de la fenêtre composée de l'étage ``level`` (aligné sur les multiples de l'échelle)."""
    scale = 1 << (level + 1)
    real = sample_real(_CTX["real"], _CTX["warp"], level, x0, y0, width, height)
    if (
        "gain" in _CTX
    ):  # gain réel par chaîne (même formule qu'à la résolution de la carte)
        gain = erode.upsample_window(_CTX["gain"], x0, y0, width, height, scale, 1)
        socle = erode.upsample_window(_CTX["socle"], x0, y0, width, height, scale, 0)
        span = erode.upsample_window(_CTX["span"], x0, y0, width, height, scale, 0)
        real = apply_gain(real, gain, socle, span)
        del gain, socle, span
    ramp = erode.upsample_window(_CTX["ramp"], x0, y0, width, height, scale, 1)
    delta = erode.upsample_window(_CTX["delta"], x0, y0, width, height, scale, 3)
    land = erode.upsample_window(_CTX["land"], x0, y0, width, height, scale, 1) > 0.5
    sea = erode.upsample_window(_CTX["author"], x0, y0, width, height, scale, 1)
    land_h = np.maximum(np.maximum(real, ramp) + delta, LAND_MIN_M)
    # Lacs intérieurs (eau du masque dont le relief d'auteur est > 0) : cuvettes plates d'auteur, niveau lu au plus proche.
    lake = erode.upsample_window(_CTX["lake"], x0, y0, width, height, scale, 0)
    water = np.where(lake > 0.0, lake, np.minimum(sea, -0.5))
    return np.where(land, land_h, water).astype(np.float32)


def tile_job(args: tuple[int, int, str]) -> dict:
    """Une tuile E0 (1024 x 1024 pixels E1) : écrit E1, renvoie E0 et le bloc carte, raffine E2-E3."""
    col0, row0, out_dir = args
    cfg = _CTX["cfg"]
    width = 2 * TILE_PX
    x0, y0 = col0 * width, row0 * width
    e1 = fine_window(1, x0, y0, width, width)
    e0 = e1.reshape(TILE_PX, 2, TILE_PX, 2).mean(axis=(1, 3))
    e0_u16 = terrain.height_to_uint16(e0)
    terrain.write_png16(e0_u16, Path(_CTX["e0_dir"]) / f"h_{col0}_{row0}.png")
    base_block = e1.reshape(256, 4, 256, 4).mean(axis=(1, 3))
    written: dict[int, list] = {1: [], 2: [], 3: []}
    relief_tiles = []
    for dj in range(2):
        for di in range(2):
            block = e1[
                dj * TILE_PX : (dj + 1) * TILE_PX, di * TILE_PX : (di + 1) * TILE_PX
            ]
            col, row = 2 * col0 + di, 2 * row0 + dj
            if float(block.max()) <= 0.0:
                continue
            directory = Path(out_dir) / "E1"
            directory.mkdir(parents=True, exist_ok=True)
            erode.write_tile(str(directory / f"{col}_{row}.png"), block)
            written[1].append((col, row))
            if (
                float(block.max() - max(block.min(), 0.0))
                >= cfg["refine"]["min_relief_m"]
            ):
                relief_tiles.append((col, row))
    real = _CTX["real"]
    for col, row in relief_tiles:
        for level in cfg["refine"]["levels"]:
            n = 1 << (level - 1)
            lx, ly = col * n * TILE_PX, row * n * TILE_PX
            if (
                real_coverage(
                    real, _CTX["warp"], level, lx, ly, n * TILE_PX, n * TILE_PX
                )
                < cfg["refine"]["min_real_coverage"]
            ):
                continue
            window = fine_window(level, lx, ly, n * TILE_PX, n * TILE_PX)
            directory = Path(out_dir) / f"E{level}"
            directory.mkdir(parents=True, exist_ok=True)
            for dj in range(n):
                for di in range(n):
                    block = window[
                        dj * TILE_PX : (dj + 1) * TILE_PX,
                        di * TILE_PX : (di + 1) * TILE_PX,
                    ]
                    if float(block.max()) <= 0.0:
                        continue
                    c, r = col * n + di, row * n + dj
                    erode.write_tile(str(directory / f"{c}_{r}.png"), block)
                    written[level].append((c, r))
    return {
        "col": col0,
        "row": row0,
        "e0": e0_u16,
        "base": base_block,
        "written": written,
    }


def base_job(args: tuple[int, int]) -> tuple[int, int, np.ndarray]:
    """Relief réel à la résolution de la carte : une tuile racine (256 x 256 unités), échelle E0 -> moyenne 2 x 2."""
    col, row = args
    e0 = fine_window_raw(0, col * TILE_PX, row * TILE_PX, TILE_PX, TILE_PX)
    return col, row, e0.reshape(256, 2, 256, 2).mean(axis=(1, 3))


def fine_window_raw(
    level: int, x0: int, y0: int, width: int, height: int
) -> np.ndarray:
    """Hauteur réelle brute (sans contrainte)."""
    return sample_real(_CTX["real"], _CTX["warp"], level, x0, y0, width, height)


# ----------------------------------------------------------------------------
# Orchestration
# ----------------------------------------------------------------------------


def tiles_in_bbox(
    shape: tuple[int, int], bbox: tuple[int, int, int, int] | None
) -> list[tuple[int, int]]:
    """Tuiles racines ``(col, row)`` (256 x 256 unités) du raster ``shape`` = (rows, cols) qui touchent ``bbox``.

    ``bbox`` = ``(x0, y0, x1, y1)`` en pixels du canevas composé (unités de carte) ; ``None`` : toutes.
    """
    ph, pw = shape
    tiles = []
    for row in range(ph // 256):
        for col in range(pw // 256):
            if bbox is not None:
                x0, y0, x1, y1 = bbox
                if (
                    (col + 1) * 256 <= x0
                    or col * 256 >= x1
                    or (row + 1) * 256 <= y0
                    or row * 256 >= y1
                ):
                    continue
            tiles.append((col, row))
    return tiles


def swap_dir(new: Path, final: Path) -> None:
    """Remplace ``final`` par ``new`` (renommages : le dossier final n'est jamais à moitié écrit)."""
    import shutil

    old = final.with_name(final.name + ".old")
    if old.exists():
        shutil.rmtree(old)
    if final.exists():
        final.rename(old)
    new.rename(final)
    if old.exists():
        shutil.rmtree(old)


def compose_relief_real(
    cfg_path: Path | None = None,
    map_dir: Path = MAP_DIR,
    workers: int = 2,
    scratch: Path = erode.RAW_DIR,
    progress: bool = True,
    write: bool = True,
    reuse_base: bool = False,
    bbox: tuple[int, int, int, int] | None = None,
) -> dict:
    """Relief réel déformé + contraintes d'auteur : base, heightmap, E0, pyramide E1-E3 ; rapport.

    Écriture atomique (lot CC CS) : pyramide et ``height/`` sont écrits dans un dossier voisin ``*.new``
    puis échangés par renommage à la fin (la pyramide partagée n'est plus effacée d'abord).
    ``bbox`` : recalcul limité aux tuiles racines qui touchent la région (pixels composés) ; la sortie va
    alors dans ``scratch/pyramid_region`` et ``scratch/height_region`` (aucun fichier de la carte ni pyramide
    partagée touchés), pour itérer vite. Les tuiles obtenues sont identiques à celles d'un calcul complet.
    """
    t0 = time.time()

    def log(msg: str) -> None:
        if progress:
            print(
                f"[relief-real] {msg} ({time.time() - t0:.0f} s, pic {peak_rss_gb():.2f} Go)",
                flush=True,
            )

    cfg = load_config(cfg_path)
    rcfg = author_relief.load_config()
    scratch.mkdir(parents=True, exist_ok=True)
    DERIVED_DIR.mkdir(parents=True, exist_ok=True)
    real_root = RealPyramid.find_root(cfg["real_root_candidates"])
    cw = ComposedWarp.load(COMPOSE_DIR)
    size = (cw.size_px[0], cw.size_px[1])
    pw, ph = erode.padded_units(size)
    log(f"relief réel : {real_root}")
    author_raw, inp = author_height(cw, rcfg, DERIVED_DIR)
    log("entrées chargées")
    shape = (ph, pw)
    author = pad_to(author_raw, shape)
    del author_raw
    land = pad_to(inp.land, shape, "constant")
    inside = pad_to(inp.inside, shape, "constant")
    paths = {"cfg": cfg, "real_root": str(real_root)}
    for key, array in (
        ("land", land.astype(np.float32)),
        ("author", author.astype(np.float32, copy=False)),
        (
            "lake",
            np.where(~land & (author > 0.0), author, -9000.0).astype(np.float32),
        ),
    ):
        np.save(scratch / f"ctx_{key}.npy", array)
        paths[key] = str(scratch / f"ctx_{key}.npy")
        del array
    log("relief d'auteur prêt")
    base_file = scratch / "real_base.npy"
    if reuse_base and base_file.exists():
        real_b = np.load(base_file)
        log("relief réel (résolution de la carte) relu du cache")
    else:
        # passe 1 : relief réel brut à la résolution de la carte
        real_b = np.zeros(shape, dtype=np.float32)
        jobs = [(c, r) for r in range(ph // 256) for c in range(pw // 256)]
        with ProcessPoolExecutor(
            workers, initializer=init_worker, initargs=(paths,)
        ) as pool:
            for col, row, block in pool.map(base_job, jobs, chunksize=4):
                real_b[row * 256 : (row + 1) * 256, col * 256 : (col + 1) * 256] = block
        np.save(base_file, real_b)  # brut : indépendant des réglages
        log("relief réel échantillonné à la résolution de la carte")
    gain, socle, span = chain_gain_fields(inp, shape, cfg)
    report: dict = {"gain_chains": sorted(cfg.get("chains", {}))}
    if gain.any():
        real_b = apply_gain(real_b, gain, socle, span)
        for key, array in (("gain", gain), ("socle", socle), ("span", span)):
            np.save(scratch / f"ctx_{key}.npy", array)
            paths[key] = str(scratch / f"ctx_{key}.npy")
        report["gain_px"] = int((gain > 0.0).sum())
        report["real_b_max_m"] = round(float(real_b.max()), 1)
    del gain, socle, span
    fields, report_c = constrain_base(real_b, land, author, inp, inside, rcfg, cfg)
    report.update(report_c)
    for key, value in fields.items():
        np.save(scratch / f"ctx_{key}.npy", value)
        paths[key] = str(scratch / f"ctx_{key}.npy")
    log(f"contraintes : {report}")
    report["author_mass"] = str(write_author_mass(author, land, size))
    if not write:
        return report
    del real_b, fields, author, inside, inp
    gc.collect()
    if bbox is None:
        final_pyramid = (map_dir / "pyramid").resolve()
        final_e0 = map_dir / "height"
        out_dir = final_pyramid.with_name(final_pyramid.name + ".new")
        e0_dir = final_e0.with_name(final_e0.name + ".new")
    else:
        out_dir, e0_dir = scratch / "pyramid_region", scratch / "height_region"
    import shutil

    for directory in (out_dir, e0_dir):
        if directory.exists():
            shutil.rmtree(directory)
        directory.mkdir(parents=True)
    paths["e0_dir"] = str(e0_dir)
    e0_full = np.zeros((ph * 2, pw * 2), dtype=np.uint16)
    base_fine = np.zeros(shape, dtype=np.float32)
    present: dict[int, set] = {1: set(), 2: set(), 3: set()}
    tiles = [(c, r, str(out_dir)) for c, r in tiles_in_bbox(shape, bbox)]
    with ProcessPoolExecutor(
        workers, initializer=init_worker, initargs=(paths,)
    ) as pool:
        for n, res in enumerate(pool.map(tile_job, tiles, chunksize=2)):
            c, r = res["col"], res["row"]
            e0_full[
                r * TILE_PX : (r + 1) * TILE_PX, c * TILE_PX : (c + 1) * TILE_PX
            ] = res["e0"]
            base_fine[r * 256 : (r + 1) * 256, c * 256 : (c + 1) * 256] = res["base"]
            for lv, items in res["written"].items():
                present[lv].update(items)
            if n % 50 == 49:
                log(f"tuiles racines {n + 1}/{len(tiles)}")
    log("pyramide E1-E3 écrite")
    write_manifest(out_dir, present, cfg)
    report["tiles"] = {str(k): len(v) for k, v in present.items()}
    if bbox is not None:
        report["region_output"] = str(out_dir)
        report["seconds"] = round(time.time() - t0, 1)
        return report
    swap_dir(out_dir, final_pyramid)
    swap_dir(e0_dir, final_e0)
    report["outputs"] = write_map_files(
        e0_full, base_fine, land, size, cfg, map_dir, np.load(scratch / "ctx_lake.npy")
    )
    log("heightmap, E0, ombrage écrits")
    report["seconds"] = round(time.time() - t0, 1)
    return report


def write_manifest(out_dir: Path, present: dict[int, set], cfg: dict) -> None:
    """Manifeste de la pyramide composée (E0 = ``height/`` de la carte)."""
    manifest = {
        "description": "Pyramide du relief composé = MNT réel déformé (lot CC CD, geo compose-relief-real). Généré, hors git.",
        "version": 1,
        "tile_px": TILE_PX,
        "root_tile_units": erode.ROOT_TILE_UNITS,
        "root_origin_tiles": [0, 0],
        "height_min_m": terrain.HEIGHT_MIN_M,
        "height_range_m": terrain.HEIGHT_MAX_M - terrain.HEIGHT_MIN_M,
        "dir": ".",
        "pattern": "E{level}/{col}_{row}.png",
        "levels": [
            {
                "level": lv,
                "meters_per_px": round(NOMINAL_M / (1 << (lv + 1)), 3),
                "source": "MNT réel déformé (Copernicus GLO-90/30)",
                "tiles_rle": erode.rle_rows(tiles),
            }
            for lv, tiles in sorted(present.items())
            if tiles
        ],
        "counts": {str(lv): len(t) for lv, t in sorted(present.items())},
    }
    (out_dir / "relief_pyramid.json").write_text(
        json.dumps(manifest, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def write_map_files(
    e0_u16: np.ndarray,
    base_fine: np.ndarray,
    land: np.ndarray,
    size: tuple[int, int],
    cfg: dict,
    map_dir: Path,
    lake_level: np.ndarray,
    band_rows: int = 1024,
    halo: int = 64,
) -> dict:
    """``heightmap.png``, ``heightmap_render.png``, ombrage BC5 et ``map.json`` ; E0 est déjà écrit par les tuiles.

    Flux par bandes de ``band_rows`` lignes E0 (+ halo) : pic mémoire de quelques centaines de Mo au lieu
    de plusieurs rasters complets de 700 Mo.
    """
    from cent_ans_tools.geo import block_compress, relief, relief_shade

    # cuvettes de lacs : plates, niveau d'auteur (``lake_level`` > 0 ; -9000 ailleurs)
    base = np.where(
        land,
        np.maximum(base_fine, LAND_MIN_M),
        np.where(lake_level > 0.0, lake_level, np.minimum(base_fine, -0.5)),
    )
    enc = terrain.height_to_uint16(base[: size[1], : size[0]])
    terrain.write_png16(enc, map_dir / "heightmap.png")
    del enc
    rows, cols = e0_u16.shape
    scale = (terrain.HEIGHT_MAX_M - terrain.HEIGHT_MIN_M) / 65535.0
    render_full = np.zeros(land.shape, dtype=np.float32)
    shade = np.zeros((rows, cols, 2), dtype=np.uint8)
    for r0 in range(0, rows, band_rows):
        lo, hi = max(r0 - halo, 0), min(r0 + band_rows + halo, rows)
        band = e0_u16[lo:hi].astype(np.float32) * np.float32(scale) + np.float32(
            terrain.HEIGHT_MIN_M
        )
        land_half = land[lo // 2 : hi // 2]
        land_fine = np.repeat(np.repeat(land_half, 2, axis=0), 2, axis=1)
        mean = relief_shade.block_mean(band, 2)
        render = np.where(land_half, mean, base[lo // 2 : hi // 2]).astype(np.float32)
        detail = band - relief_shade.bilinear_upsample(render, 2)
        occ = relief_shade.occlusion(band, NOMINAL_M / 2)
        detail[~land_fine] = 0.0
        occ[~land_fine] = 0.0
        piece = relief_shade.encode_shade(detail, occ)
        keep0, keep1 = r0 - lo, min(r0 + band_rows, rows) - lo
        shade[r0 : r0 + (keep1 - keep0)] = piece[keep0:keep1]
        render_full[r0 // 2 : r0 // 2 + (keep1 - keep0) // 2] = render[
            keep0 // 2 : keep1 // 2
        ]
        del band, land_fine, mean, render, detail, occ, piece
    render_out = render_full[: size[1], : size[0]]
    terrain.write_png16(
        terrain.height_to_uint16(render_out), map_dir / relief_shade.RENDER_HEIGHTMAP
    )
    del render_full, render_out
    for stale in map_dir.glob(relief_shade.RELIEF_SHADE_STEM + "_[0-9]*.png"):
        stale.unlink()
    shade_meta = block_compress.write_relief_bc5(shade, map_dir).meta
    del shade
    meta_path = map_dir / "map.json"
    meta = json.loads(meta_path.read_text("utf-8"))
    meta["height_tiles"] = {
        "size_px": [land.shape[1] * 2, land.shape[0] * 2],
        "tile_px": relief.TILE_PX,
        "dir": relief.TILE_DIR,
        "pattern": relief.TILE_PATTERN,
    }
    meta["render_heightmap"] = {
        "file": relief_shade.RENDER_HEIGHTMAP,
        "composed": "relief réel déformé (CC CD) : hauteurs vraies, exagération RE à l'exécution",
        "boost": None,
    }
    meta["relief_shade"] = {
        "detail_scale_m": relief_shade.DETAIL_SCALE_M,
        "bc5": shade_meta,
    }
    meta.setdefault("sources", {})["dem"] = {
        "name": "MNT réel (ETOPO, Copernicus GLO-90/30) déformé par warp.json + contraintes d'auteur (CC CD)",
        "settings": "compose/relief_real.json",
    }
    meta_path.write_text(
        json.dumps(meta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return {"e0_tiles": int(rows // 512 * (cols // 512))}


def make_boards(
    map_dir: Path = MAP_DIR,
    before_heightmap: Path | None = None,
    out_dir: Path = erode.RAW_DIR,
    width: int = 900,
) -> list[Path]:
    """Planches 2D avant / après (Alpes, Pyrénées, Massif central), côté ``width`` px, lues dans la pyramide E1.

    ``avant`` : relief d'auteur (``before_heightmap``, ex. ``git show <rev>:data/map/heightmap.png``)
    sur-échantillonné x4 en bicubique ; ``après`` : tuiles E1 écrites par ``compose-relief-real``.
    """
    cw = ComposedWarp.load(COMPOSE_DIR)
    scale = 4
    px_m = NOMINAL_M / scale
    out_dir.mkdir(parents=True, exist_ok=True)
    before_base = None
    if before_heightmap is not None:
        before_base = terrain.uint16_to_height(
            terrain.read_png16(before_heightmap)
        ).astype(np.float32)
    paths = []
    for name, (lon, lat) in erode.SITES.items():
        x, y = cw.forward(lon, lat)
        x0 = (int(x * scale) - width // 2) // TILE_PX * TILE_PX
        y0 = (int(y * scale) - width // 2) // TILE_PX * TILE_PX
        mosaic = np.zeros((2 * TILE_PX, 2 * TILE_PX), dtype=np.float32)
        for dj in range(2):
            for di in range(2):
                tile = (
                    map_dir
                    / "pyramid"
                    / "E1"
                    / f"{x0 // TILE_PX + di}_{y0 // TILE_PX + dj}.png"
                )
                if tile.exists():
                    mosaic[
                        dj * TILE_PX : (dj + 1) * TILE_PX,
                        di * TILE_PX : (di + 1) * TILE_PX,
                    ] = terrain.uint16_to_height(terrain.read_png16(tile))
        shots = [("apres", mosaic[:width, :width])]
        if before_base is not None:
            old = erode.upsample_window(
                before_base, x0, y0, 2 * TILE_PX, 2 * TILE_PX, scale, 3
            )
            shots.insert(0, ("avant", old[:width, :width]))
        for tag, array in shots:
            path = out_dir / f"planche_reel_{name}_{tag}.png"
            erode.hillshade_m(array, px_m).save(path)
            paths.append(path)
    return paths


def rewrite_map_files_only(
    map_dir: Path = MAP_DIR, scratch: Path = erode.RAW_DIR, progress: bool = True
) -> dict:
    """Réécrit ``heightmap.png``, ``heightmap_render.png``, l'ombrage et ``map.json`` depuis les tuiles E0 déjà écrites."""
    t0 = time.time()
    cw = ComposedWarp.load(COMPOSE_DIR)
    size = (cw.size_px[0], cw.size_px[1])
    pw, ph = erode.padded_units(size)
    land = np.load(scratch / "ctx_land.npy", mmap_mode="r") > 0.5
    e0_u16 = np.zeros((ph * 2, pw * 2), dtype=np.uint16)
    for row in range(ph // 256):
        for col in range(pw // 256):
            e0_u16[
                row * TILE_PX : (row + 1) * TILE_PX, col * TILE_PX : (col + 1) * TILE_PX
            ] = terrain.read_png16(map_dir / "height" / f"h_{col}_{row}.png")
    scale = (terrain.HEIGHT_MAX_M - terrain.HEIGHT_MIN_M) / 65535.0
    base_fine = np.zeros(land.shape, dtype=np.float32)
    for r0 in range(0, ph * 2, 2048):
        band = e0_u16[r0 : r0 + 2048].astype(np.float32) * np.float32(
            scale
        ) + np.float32(terrain.HEIGHT_MIN_M)
        base_fine[r0 // 2 : (r0 + 2048) // 2] = band.reshape(1024, 2, -1, 2).mean(
            axis=(1, 3)
        )
    out = write_map_files(
        e0_u16,
        base_fine,
        np.asarray(land),
        size,
        load_config(),
        map_dir,
        np.load(scratch / "ctx_lake.npy"),
    )
    if progress:
        print(
            f"[relief-real] fichiers de carte réécrits ({time.time() - t0:.0f} s)",
            flush=True,
        )
    return out
