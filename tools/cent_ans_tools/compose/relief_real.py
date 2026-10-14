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
REAL_ENV = "CENT_ANS_REAL_RELIEF"
DERIVED_DIR = COMPOSE_DIR / "derived"


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
            if len(self._cache) > 400:
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
    r_per_unit = float(1 << (r_level + 1))
    xs = (x0 + np.arange(width) + 0.5) / per_unit
    ys = (y0 + np.arange(height) + 0.5) / per_unit
    gx, gy = np.meshgrid(xs, ys)
    rx, ry = warp.inverse_px(gx, gy)
    rx = np.nan_to_num(rx * r_per_unit - 0.5, nan=0.0)
    ry = np.nan_to_num(ry * r_per_unit - 0.5, nan=0.0)
    mx0, my0 = int(np.floor(rx.min())) - 3, int(np.floor(ry.min())) - 3
    mx1, my1 = int(np.ceil(rx.max())) + 4, int(np.ceil(ry.max())) + 4
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
    inp = author_relief.load_inputs(cw, rcfg)
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
    reach = c["band_px"] + c["fade_px"] + 2.0
    size = 2 * int(c["peak_radius_px"]) + 1
    peak = ndimage.maximum_filter(real_b, size=size)
    ceiling = np.minimum(c["floor_author_frac"] * author, c["floor_author_cap_m"])
    target = np.maximum(c["floor_real_frac"] * peak, ceiling)
    for ridge in inp.ridges:
        if ridge["kind"] != "massif":
            continue
        pts = author_relief.smooth_points(ridge["points"], 2)
        window = author_relief._window(pts, reach, author.shape)
        x0, y0, x1, y1 = window
        if x1 <= x0 or y1 <= y0:
            continue
        field = author_relief.polyline_field(pts, window)
        band = 1.0 - author_relief.smoothstep(
            (field.dist - c["band_px"]) / max(c["fade_px"], 1e-3)
        )
        view = out[y0:y1, x0:x1]
        np.maximum(view, (band * target[y0:y1, x0:x1]).astype(np.float32), out=view)
    return out


def constrain_base(
    real_b: np.ndarray,
    land: np.ndarray,
    author: np.ndarray,
    inp: author_relief.Inputs,
    inside: np.ndarray,
    rcfg: dict,
    cfg: dict,
) -> tuple[dict[str, np.ndarray], dict]:
    """Contraintes d'auteur à la résolution de la carte, sous forme de CHAMPS appliqués au relief réel fin.

    ``floor`` : plancher (rampe côtière, crêtes d'auteur) -> ``max(réel, floor)`` ;
    ``cap`` : plafond des encoches de cols (9000 hors encoche) -> ``min(., cap)`` ;
    ``plat`` : correction additive des plateformes sous les lieux (|.| <= ``platforms.max_shift_m``).
    Chaque champ s'applique au relief réel fin (E1-E3) : le détail réel est conservé partout ailleurs.
    """
    report: dict = {}
    ramp = np.minimum(author, cfg["lowland_cap_m"])
    crest = crest_floor(inp, author, real_b, cfg)
    floor = np.where(land, np.maximum(ramp, crest), -9000.0).astype(np.float32)
    report["crest_lifted_px"] = int(((crest > real_b) & land).sum())
    h1 = np.where(land, np.maximum(real_b, floor), author).astype(np.float32)
    h2 = h1.copy()
    valley_floor = ndimage.gaussian_filter(ndimage.minimum_filter(h2, 25), 12.0)
    pass_cfg = {**rcfg, "passes": {**rcfg["passes"], **cfg["passes"]}}
    report["passes"] = len(author_relief.carve_passes(h2, valley_floor, inp, pass_cfg))
    cap = np.where(h2 < h1 - 0.01, h2, 9000.0).astype(np.float32)
    h3 = h2.copy()
    report["platforms"] = author_relief.flatten_platforms(h3, land, inp.places, rcfg)
    plat = np.where(land, h3 - h2, 0.0).astype(np.float32)
    report["cap_px"] = int((cap < 9000.0).sum())
    report["plat_range_m"] = [round(float(plat.min()), 1), round(float(plat.max()), 1)]
    return {"floor": floor, "cap": cap, "plat": plat}, report


# ----------------------------------------------------------------------------
# Fenêtres fines (processus de calcul)
# ----------------------------------------------------------------------------

_CTX: dict = {}


def init_worker(ctx_paths: dict) -> None:
    """Ouvre en lecture seule (mmap) les rasters de la carte et la pyramide réelle."""
    _CTX.clear()
    _CTX["cfg"] = ctx_paths["cfg"]
    _CTX["real"] = RealPyramid(Path(ctx_paths["real_root"]))
    _CTX["warp"] = ComposedWarp.load(COMPOSE_DIR)
    for key in ("floor", "cap", "plat", "land", "author"):
        _CTX[key] = np.load(ctx_paths[key], mmap_mode="r")


def fine_window(level: int, x0: int, y0: int, width: int, height: int) -> np.ndarray:
    """Hauteur finale (m, vraie) de la fenêtre composée de l'étage ``level`` (aligné sur les multiples de l'échelle)."""
    scale = 1 << (level + 1)
    real = sample_real(_CTX["real"], _CTX["warp"], level, x0, y0, width, height)
    floor = erode.upsample_window(_CTX["floor"], x0, y0, width, height, scale, 1)
    cap = erode.upsample_window(_CTX["cap"], x0, y0, width, height, scale, 1)
    plat = erode.upsample_window(_CTX["plat"], x0, y0, width, height, scale, 3)
    land = erode.upsample_window(_CTX["land"], x0, y0, width, height, scale, 1) > 0.5
    sea = erode.upsample_window(_CTX["author"], x0, y0, width, height, scale, 1)
    land_h = np.maximum(np.minimum(np.maximum(real, floor), cap) + plat, LAND_MIN_M)
    return np.where(land, land_h, np.minimum(sea, -0.5)).astype(np.float32)


def tile_job(args: tuple[int, int, str]) -> dict:
    """Une tuile E0 (1024 x 1024 pixels E1) : écrit E1, renvoie E0 et le bloc carte, raffine E2-E3."""
    col0, row0, out_dir = args
    cfg = _CTX["cfg"]
    width = 2 * TILE_PX
    x0, y0 = col0 * width, row0 * width
    e1 = fine_window(1, x0, y0, width, width)
    e0 = e1.reshape(TILE_PX, 2, TILE_PX, 2).mean(axis=(1, 3))
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
    return {"col": col0, "row": row0, "e0": e0, "base": base_block, "written": written}


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


def compose_relief_real(
    cfg_path: Path | None = None,
    map_dir: Path = MAP_DIR,
    workers: int = 6,
    scratch: Path = erode.RAW_DIR,
    progress: bool = True,
    write: bool = True,
    reuse_base: bool = False,
) -> dict:
    """Relief réel déformé + contraintes d'auteur : base, heightmap, E0, pyramide E1-E3 ; rapport."""
    t0 = time.time()

    def log(msg: str) -> None:
        if progress:
            print(f"[relief-real] {msg} ({time.time() - t0:.0f} s)", flush=True)

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
    shape = (ph, pw)
    author = pad_to(author_raw, shape)
    land = pad_to(inp.land, shape, "constant")
    inside = pad_to(inp.inside, shape, "constant")
    log("relief d'auteur prêt")
    paths = {"cfg": cfg, "real_root": str(real_root)}
    np.save(scratch / "ctx_land.npy", land.astype(np.float32))
    np.save(scratch / "ctx_author.npy", author.astype(np.float32))
    paths.update(
        land=str(scratch / "ctx_land.npy"), author=str(scratch / "ctx_author.npy")
    )
    # passe 1 : relief réel brut à la résolution de la carte
    for key in ("floor", "cap", "plat"):
        np.save(scratch / f"ctx_{key}.npy", np.zeros(shape, dtype=np.float32))
        paths[key] = str(scratch / f"ctx_{key}.npy")
    real_b = np.zeros(shape, dtype=np.float32)
    jobs = [(c, r) for r in range(ph // 256) for c in range(pw // 256)]
    with ProcessPoolExecutor(
        workers, initializer=init_worker, initargs=(paths,)
    ) as pool:
        for col, row, block in pool.map(base_job, jobs, chunksize=4):
            real_b[row * 256 : (row + 1) * 256, col * 256 : (col + 1) * 256] = block
    log("relief réel échantillonné à la résolution de la carte")
    fields, report = constrain_base(real_b, land, author, inp, inside, rcfg, cfg)
    for key, value in fields.items():
        np.save(scratch / f"ctx_{key}.npy", value)
    np.save(scratch / "real_base.npy", real_b)
    log(f"contraintes : {report}")
    if not write:
        return report
    out_dir = map_dir / "pyramid"
    if out_dir.exists():
        import shutil

        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)
    e0_full = np.zeros((ph * 2, pw * 2), dtype=np.float32)
    base_fine = np.zeros(shape, dtype=np.float32)
    present: dict[int, set] = {1: set(), 2: set(), 3: set()}
    tiles = [(c, r, str(out_dir)) for r in range(ph // 256) for c in range(pw // 256)]
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
    report["outputs"] = write_map_files(e0_full, base_fine, land, size, cfg, map_dir)
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
    e0_full: np.ndarray,
    base_fine: np.ndarray,
    land: np.ndarray,
    size: tuple[int, int],
    cfg: dict,
    map_dir: Path,
) -> dict:
    """``heightmap.png``, ``height/`` (E0), ``heightmap_render.png``, ombrage BC5 et ``map.json``."""
    from cent_ans_tools.geo import block_compress, relief, relief_shade

    base = np.where(
        land, np.maximum(base_fine, LAND_MIN_M), np.minimum(base_fine, -0.5)
    )
    enc = terrain.height_to_uint16(base[: size[1], : size[0]])
    terrain.write_png16(enc, map_dir / "heightmap.png")
    land_fine = relief_shade.upsample_sign(
        np.where(land, 1.0, -1.0).astype(np.float32), 2
    )
    fine = relief_shade.enforce_coast(e0_full, land_fine)
    encoded = terrain.height_to_uint16(fine)
    tiles = relief.write_tiles(encoded, map_dir / relief.TILE_DIR)
    fine = terrain.uint16_to_height(encoded).astype(np.float32)
    del encoded
    render_full = relief_shade.block_mean(fine, 2)
    render_full = np.where(land, render_full, base)
    render = render_full[: size[1], : size[0]]
    terrain.write_png16(
        terrain.height_to_uint16(render), map_dir / relief_shade.RENDER_HEIGHTMAP
    )
    detail = fine - relief_shade.bilinear_upsample(render_full, 2)
    occ = relief_shade.occlusion(fine, NOMINAL_M / 2)
    detail[~land_fine] = 0.0
    occ[~land_fine] = 0.0
    shade = relief_shade.encode_shade(detail, occ)
    del detail, occ, fine
    for stale in map_dir.glob(relief_shade.RELIEF_SHADE_STEM + "_[0-9]*.png"):
        stale.unlink()
    shade_meta = block_compress.write_relief_bc5(shade, map_dir).meta
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
    return {"tiles": len(tiles)}
