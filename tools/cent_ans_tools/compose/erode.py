"""Érosion hors ligne du relief composé et pyramide fine (lot CC CD) : ``cent-ans geo compose-erode``.

Pourquoi : la carte composée n'a que le raster procédural lisse de ``compose-relief``
(719 m/px, tuiles E0 à 359 m). De près, aspect « pâte ». Ce module fabrique, hors ligne et à
0 $, un relief fin plausible :

1. ``heightmap.png`` est sur-échantillonné (bicubique) à ``work_scale`` x (4 : étage E1, 180 m) ;
2. un bruit fractal « ridged », d'amplitude guidée par la pente et nulle en plaine, casse les
   formes lisses et amorce un réseau de drainage ;
3. érosion fluviale par loi de puissance implicite (Braun et Willett, numba) : ravines, talwegs,
   vallées ramifiées et crêtes vives entre elles ; érosion thermique (éboulis, talus) ;
4. ancrage : la composante basse fréquence du relief d'auteur est rétablie (hauteurs des massifs,
   règle ``massif_impassable`` du navgrid inchangée : ``heightmap.png`` n'est jamais réécrit) ;
5. les cols de ``crossings.json`` sont protégés (relief d'auteur quasi intact autour) ;
6. pyramide : E0 (moyenne 2x2 de E1), E1, puis E2-E3 synthétisés par tuile là où le relief le
   justifie (bruit guidé par la pente + talus) ; masques roche / éboulis / neige.

Tout est dans ``data/map/compose/erode.json`` ; la sortie va dans ``data/map/pyramid/`` (hors git).
Rejouable : la commande relit ``heightmap.png`` à chaque lancement.
"""

from __future__ import annotations

import json
import time
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
from numba import njit
from PIL import Image
from scipy import ndimage

from cent_ans_tools.compose.contract import COMPOSE_DIR
from cent_ans_tools.compose.warp import MAP_DIR
from cent_ans_tools.geo import terrain

Image.MAX_IMAGE_PIXELS = None

ERODE_FILE = "erode.json"
PYRAMID_DIR = MAP_DIR / "pyramid"
RAW_DIR = Path.home() / "dev" / "cent-ans-raw" / "compose" / "cd"
ROOT_TILE_UNITS = 256
TILE_PX = 512
NOMINAL_M = 719.0
LAND_MIN_M = 0.5

# ----------------------------------------------------------------------------
# Réglages, entrées
# ----------------------------------------------------------------------------


def load_config(path: Path | None = None) -> dict:
    """``data/map/compose/erode.json``."""
    return json.loads((path or COMPOSE_DIR / ERODE_FILE).read_text("utf-8"))


def smoothstep(x: np.ndarray | float) -> np.ndarray:
    """Marche douce bornée à [0, 1]."""
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3.0 - 2.0 * x)


def padded_units(size_px: tuple[int, int]) -> tuple[int, int]:
    """Canevas complété à un multiple de 256 pixels de carte."""
    w, h = size_px
    return -(-w // ROOT_TILE_UNITS) * ROOT_TILE_UNITS, -(
        -h // ROOT_TILE_UNITS
    ) * ROOT_TILE_UNITS


def load_base(map_dir: Path = MAP_DIR) -> tuple[np.ndarray, np.ndarray]:
    """Relief composé (m) et masque de terre au pixel de carte, complétés au multiple de 256."""
    h = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png")).astype(
        np.float32
    )
    land = np.asarray(Image.open(map_dir / "land_mask.png")) > 0
    pw, ph = padded_units((h.shape[1], h.shape[0]))
    pad = ((0, ph - h.shape[0]), (0, pw - h.shape[1]))
    return np.pad(h, pad, mode="edge"), np.pad(land, pad, mode="edge")


def activation_from_height(h: np.ndarray, cfg: dict, radius_px: int) -> np.ndarray:
    """Part d'érosion (0 plaine, 1 relief) : altitude et amplitude locale du relief."""
    a = cfg["activation"]
    by_height = smoothstep((h - a["floor_m"]) / (a["full_m"] - a["floor_m"]))
    size = max(3, int(radius_px) * 2 + 1)
    local = ndimage.maximum_filter(h, size=size) - ndimage.minimum_filter(h, size=size)
    by_relief = smoothstep(
        (local - a["relief_floor_m"]) / (a["relief_full_m"] - a["relief_floor_m"])
    )
    act = np.maximum(by_height * 0.6, by_relief) * by_height.clip(0.25, 1.0)
    return ndimage.gaussian_filter(act, 2.0).astype(np.float32)


def pass_mask(shape: tuple[int, int], points: list[tuple[float, float]], radius: float):
    """Proximité (1 sur le col, 0 au-delà de ``radius``) des cols, en pixels de carte."""
    if not points:
        return np.zeros(shape, dtype=np.float32)
    seed = np.ones(shape, dtype=bool)
    for x, y in points:
        i, j = int(round(x)), int(round(y))
        if 0 <= i < shape[1] and 0 <= j < shape[0]:
            seed[j, i] = False
    dist = ndimage.distance_transform_edt(seed)
    return (1.0 - smoothstep(dist / radius)).astype(np.float32)


def load_passes() -> list[tuple[float, float]]:
    """Cols (pixels composés) : cols libres de ``crossings.json`` et cols rattachés aux crêtes."""
    from cent_ans_tools.compose import compile as composer
    from cent_ans_tools.compose.contract import ComposedWarp

    cw = ComposedWarp.load(COMPOSE_DIR)
    points = [tuple(p["at"]) for p in composer._load_passes(cw)]
    try:
        doc = json.loads((COMPOSE_DIR / "ridges.json").read_text("utf-8"))
        for ridge in doc.get("ridges", []):
            for p in ridge.get("passes", []):
                at = p.get("at") if isinstance(p, dict) else None
                if at:
                    points.append((float(at[0]), float(at[1])))
    except (OSError, ValueError):
        pass
    return points


# ----------------------------------------------------------------------------
# Bruit global (coordonnées absolues : jointures invisibles entre blocs)
# ----------------------------------------------------------------------------


@njit(cache=True)
def _hash_unit(ix: int, iy: int, seed: int) -> float:
    h = (ix * 374761393 + iy * 668265263 + seed * 1442695041) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    h = h ^ (h >> 16)
    return (h & 0xFFFF) / 32767.5 - 1.0


@njit(cache=True)
def value_noise_global(
    height: int, width: int, x0: int, y0: int, wavelength: float, seed: int
) -> np.ndarray:
    """Bruit de valeurs [-1, 1] aux pixels globaux ``(x0 + i, y0 + j)``."""
    out = np.empty((height, width), dtype=np.float32)
    for j in range(height):
        v = (y0 + j + 0.5) / wavelength
        iv = int(np.floor(v))
        fv = v - iv
        sv = fv * fv * fv * (fv * (fv * 6.0 - 15.0) + 10.0)
        for i in range(width):
            u = (x0 + i + 0.5) / wavelength
            iu = int(np.floor(u))
            fu = u - iu
            su = fu * fu * fu * (fu * (fu * 6.0 - 15.0) + 10.0)
            a = _hash_unit(iu, iv, seed)
            b = _hash_unit(iu + 1, iv, seed)
            c = _hash_unit(iu, iv + 1, seed)
            d = _hash_unit(iu + 1, iv + 1, seed)
            top = a + (b - a) * su
            bottom = c + (d - c) * su
            out[j, i] = top + (bottom - top) * sv
    return out


def fractal_noise(
    shape: tuple[int, int],
    origin: tuple[int, int],
    wavelengths: list[float],
    weights: list[float],
    ridged: float,
    seed: int,
) -> np.ndarray:
    """Somme d'octaves ridged/lisses dans [-1, 1] (arêtes vives si ``ridged`` > 0)."""
    out = np.zeros(shape, dtype=np.float32)
    total = float(sum(weights[: len(wavelengths)]))
    for k, (wave, weight) in enumerate(zip(wavelengths, weights, strict=False)):
        n = value_noise_global(shape[0], shape[1], origin[0], origin[1], wave, seed + k)
        sharp = 1.0 - 2.0 * np.abs(n)
        out += (weight / total) * ((1.0 - ridged) * n + ridged * sharp)
    return out


# ----------------------------------------------------------------------------
# Noyaux d'érosion
# ----------------------------------------------------------------------------

_DX = np.array([-1, 0, 1, -1, 1, -1, 0, 1], dtype=np.int64)
_DY = np.array([-1, -1, -1, 0, 0, 1, 1, 1], dtype=np.int64)
_DD = np.array(
    [1.41421356, 1.0, 1.41421356, 1.0, 1.0, 1.41421356, 1.0, 1.41421356],
    dtype=np.float64,
)


@njit(cache=True)
def priority_flood(h: np.ndarray, seeds: np.ndarray, eps: float) -> np.ndarray:
    """Comble les cuvettes (Barnes, avec pente ``eps`` m/px) : tout pixel draine vers un bord ou la mer."""
    ny, nx = h.shape
    n = ny * nx
    filled = h.copy()
    done = np.zeros((ny, nx), dtype=np.bool_)
    heap_h = np.empty(n, dtype=np.float32)
    heap_i = np.empty(n, dtype=np.int64)
    size = 0
    for j in range(ny):
        for i in range(nx):
            if seeds[j, i]:
                done[j, i] = True
                k = size
                size += 1
                heap_h[k] = h[j, i]
                heap_i[k] = j * nx + i
                while k > 0:
                    parent = (k - 1) // 2
                    if heap_h[parent] <= heap_h[k]:
                        break
                    heap_h[parent], heap_h[k] = heap_h[k], heap_h[parent]
                    heap_i[parent], heap_i[k] = heap_i[k], heap_i[parent]
                    k = parent
    while size > 0:
        top_h = heap_h[0]
        top_i = heap_i[0]
        size -= 1
        if size > 0:
            heap_h[0] = heap_h[size]
            heap_i[0] = heap_i[size]
            k = 0
            while True:
                left = 2 * k + 1
                if left >= size:
                    break
                small = left
                if left + 1 < size and heap_h[left + 1] < heap_h[left]:
                    small = left + 1
                if heap_h[k] <= heap_h[small]:
                    break
                heap_h[small], heap_h[k] = heap_h[k], heap_h[small]
                heap_i[small], heap_i[k] = heap_i[k], heap_i[small]
                k = small
        cj = top_i // nx
        ci = top_i % nx
        for d in range(8):
            jj = cj + _DY[d]
            ii = ci + _DX[d]
            if jj < 0 or jj >= ny or ii < 0 or ii >= nx or done[jj, ii]:
                continue
            done[jj, ii] = True
            level = h[jj, ii]
            if level < top_h + eps:
                level = top_h + eps
            filled[jj, ii] = level
            k = size
            size += 1
            heap_h[k] = level
            heap_i[k] = jj * nx + ii
            while k > 0:
                parent = (k - 1) // 2
                if heap_h[parent] <= heap_h[k]:
                    break
                heap_h[parent], heap_h[k] = heap_h[k], heap_h[parent]
                heap_i[parent], heap_i[k] = heap_i[k], heap_i[parent]
                k = parent
    return filled


@njit(cache=True)
def _receivers(h: np.ndarray, fixed: np.ndarray) -> np.ndarray:
    ny, nx = h.shape
    rec = np.full(ny * nx, -1, dtype=np.int64)
    for j in range(1, ny - 1):
        for i in range(1, nx - 1):
            if fixed[j, i]:
                continue
            best = 0.0
            hij = h[j, i]
            for k in range(8):
                jj = j + _DY[k]
                ii = i + _DX[k]
                slope = (
                    (hij - h[jj, ii])
                    / _DD[k]
                    * (1.0 + 0.3 * _hash_unit(i * 8 + k, j, 91))
                )
                if slope > best:
                    best = slope
                    rec[j * nx + i] = jj * nx + ii
    return rec


@njit(cache=True)
def _stream_power_step(
    h: np.ndarray,
    rec: np.ndarray,
    order_desc: np.ndarray,
    k_field: np.ndarray,
    k_dt: float,
    exponent: float,
    step_cap: float,
    total_cap: float,
    incision: np.ndarray,
    area: np.ndarray,
) -> None:
    """Un pas implicite de loi de puissance : ``h_i = (h_i + f h_rec) / (1 + f)``."""
    ny, nx = h.shape
    hf = h.reshape(ny * nx)
    kf = k_field.reshape(ny * nx)
    inc = incision.reshape(ny * nx)
    af = area.reshape(ny * nx)
    af[:] = 1.0
    for idx in range(order_desc.size):
        i = order_desc[idx]
        r = rec[i]
        if r >= 0:
            af[r] += af[i]
    for idx in range(order_desc.size - 1, -1, -1):
        i = order_desc[idx]
        r = rec[i]
        if r < 0 or kf[i] <= 0.0:
            continue
        di = abs((i % nx) - (r % nx)) + abs((i // nx) - (r // nx))
        dist = 1.41421356 if di == 2 else 1.0
        f = kf[i] * k_dt * af[i] ** exponent / dist
        new = (hf[i] + f * hf[r]) / (1.0 + f)
        drop = hf[i] - new
        room = total_cap - inc[i]
        if drop > step_cap:
            drop = step_cap
        if drop > room:
            drop = room
        if drop <= 0.0:
            continue
        new = hf[i] - drop
        floor = hf[r] + 0.002 * dist
        if new < floor:
            drop = hf[i] - floor
            if drop <= 0.0:
                continue
            new = floor
        hf[i] = new
        inc[i] += drop


@njit(cache=True)
def thermal(
    h: np.ndarray, fixed: np.ndarray, weight: np.ndarray, talus: float, rate: float
) -> None:
    """Un passage d'érosion thermique : l'excédent de pente (> ``talus`` m par pixel) glisse vers le bas."""
    ny, nx = h.shape
    excess = np.zeros(8, dtype=np.float64)
    for j in range(1, ny - 1):
        for i in range(1, nx - 1):
            if fixed[j, i] or weight[j, i] <= 0.0:
                continue
            hij = h[j, i]
            total = 0.0
            biggest = 0.0
            for k in range(8):
                jj = j + _DY[k]
                ii = i + _DX[k]
                d = hij - h[jj, ii] - talus * _DD[k]
                if d < 0.0 or fixed[jj, ii]:
                    d = 0.0
                excess[k] = d
                total += d
                if d > biggest:
                    biggest = d
            if total <= 0.0:
                continue
            move = rate * weight[j, i] * 0.5 * biggest
            h[j, i] -= move
            for k in range(8):
                if excess[k] > 0.0:
                    h[j + _DY[k], i + _DX[k]] += move * excess[k] / total


def erode_array(
    h: np.ndarray, fixed: np.ndarray, act: np.ndarray, cfg: dict, px_m: float
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Érode ``h`` (float32, copié) ; renvoie ``(h, incision, aire)``. Les bords sont fixes."""
    e = cfg["erosion"]
    h = np.ascontiguousarray(h, dtype=np.float32).copy()
    fixed = fixed.copy()
    fixed[0, :] = fixed[-1, :] = True
    fixed[:, 0] = fixed[:, -1] = True
    h = priority_flood(h, fixed, float(e["fill_eps_m"]))
    incision = np.zeros_like(h)
    area = np.ones_like(h)
    k_field = np.ascontiguousarray(act, dtype=np.float32) * (~fixed)
    iterations = int(e["iterations"])
    step_cap = float(e["max_incision_m"]) * 2.5 / iterations
    talus = float(np.tan(np.radians(e["talus_deg"])) * px_m)
    for it in range(iterations):
        rec = _receivers(h, fixed)
        order = np.argsort(-h.ravel(), kind="stable")
        _stream_power_step(
            h,
            rec,
            order,
            k_field,
            float(e["k_dt"]),
            float(e["area_exponent"]),
            step_cap,
            float(e["max_incision_m"]),
            incision,
            area,
        )
        if it % int(e.get("thermal_every", 1)) == 0:
            thermal(h, fixed, act, talus, float(e["thermal_rate"]))
    for _ in range(int(e["thermal_passes"])):
        thermal(h, fixed, act, talus, float(e["thermal_rate"]))
    return h, incision, area


# ----------------------------------------------------------------------------
# Fenêtre de travail : sur-échantillonnage, bruit, érosion, ancrage
# ----------------------------------------------------------------------------

_SHARED: dict = {}


def init_worker(shared: dict) -> None:
    """Initialiseur des processus : tableaux grossiers communs (base, terre, activation, cols)."""
    _SHARED.clear()
    _SHARED.update(shared)


def upsample_window(
    coarse: np.ndarray, x0: int, y0: int, width: int, height: int, scale: int, order=3
) -> np.ndarray:
    """Fenêtre ``[x0, x0+width) x [y0, y0+height)`` (px de travail, multiples de ``scale``)."""
    margin = 4
    cx0, cy0 = x0 // scale, y0 // scale
    cx1, cy1 = -(-(x0 + width) // scale), -(-(y0 + height) // scale)
    sx0, sy0 = max(cx0 - margin, 0), max(cy0 - margin, 0)
    sx1, sy1 = min(cx1 + margin, coarse.shape[1]), min(cy1 + margin, coarse.shape[0])
    sub = coarse[sy0:sy1, sx0:sx1].astype(np.float32)
    big = ndimage.zoom(sub, scale, order=order, mode="nearest", grid_mode=True)
    ox, oy = (x0 - sx0 * scale), (y0 - sy0 * scale)
    return np.ascontiguousarray(big[oy : oy + height, ox : ox + width])


def slope_field(h: np.ndarray, px_m: float, sigma: float = 3.0) -> np.ndarray:
    """Pente sans dimension (lissée) d'un relief en mètres."""
    gy, gx = np.gradient(ndimage.gaussian_filter(h, sigma))
    return (np.hypot(gx, gy) / px_m).astype(np.float32)


def prenoise(
    base_up: np.ndarray,
    act: np.ndarray,
    origin: tuple[int, int],
    cfg: dict,
    px_m: float,
) -> np.ndarray:
    """Relief + bruit fractal guidé par la pente (nul en plaine)."""
    p = cfg["prenoise"]
    slope = slope_field(base_up, px_m)
    # pente utile : seuil bas pour laisser les plaines intactes
    drive = np.clip(slope - 0.03, 0.0, 0.8)
    # un plateau d'auteur de haute altitude est presque plat : plancher lié à l'altitude
    floor = p["height_drive"] * smoothstep(
        (base_up - p["height_drive_from_m"]) / 1200.0
    )
    drive = np.maximum(drive, floor).astype(np.float32)
    out = base_up.copy()
    for k, (wave, weight) in enumerate(
        zip(p["wavelength_px"], p["weights"], strict=False)
    ):
        n = fractal_noise(
            base_up.shape, origin, [wave], [1.0], p["ridged"], cfg["seed"] + 17 * k
        )
        amp = np.minimum(p["slope_gain"] * drive * wave * px_m, p["max_amp_m"])
        out += (weight * amp * n * act).astype(np.float32)
    return out


def process_window(job: tuple[int, int, int, int]) -> dict:
    """Traite une fenêtre de travail ``(x0, y0, w, h)`` ; renvoie la hauteur finale et les cartes."""
    x0, y0, width, height = job
    cfg = _SHARED["cfg"]
    scale = int(cfg["work_scale"])
    px_m = NOMINAL_M / scale
    base_up = upsample_window(_SHARED["base"], x0, y0, width, height, scale, 3)
    soft_land = upsample_window(_SHARED["base"], x0, y0, width, height, scale, 1)
    water = soft_land <= 0.0
    act = upsample_window(_SHARED["act"], x0, y0, width, height, scale, 1)
    act = np.clip(act, 0.0, 1.0)
    act[water] = 0.0
    shape = base_up.shape
    if float(act.max()) < 0.02:
        return {
            "job": job,
            "h": base_up,
            "inc": np.zeros(shape, np.float16),
            "area": np.ones(shape, np.float16),
        }
    pre = prenoise(base_up, act, (x0, y0), cfg, px_m)
    pre[water] = base_up[water]
    eroded, incision, area = erode_array(pre, water, act, cfg, px_m)
    # ancrage basse fréquence sur le relief d'auteur
    a = cfg["anchor"]
    sigma = a["sigma_base_px"] * scale
    delta = ndimage.gaussian_filter(base_up, sigma) - ndimage.gaussian_filter(
        eroded, sigma
    )
    final = eroded + a["strength"] * act * delta
    smooth = ndimage.gaussian_filter(final, 0.8)
    final = final + np.clip(act * 2.0, 0.0, 1.0) * (smooth - final)
    # cols d'auteur : relief d'auteur quasi intact
    p = upsample_window(_SHARED["passes"], x0, y0, width, height, scale, 1)
    keep = np.clip(p, 0.0, 1.0) * (1.0 - cfg["protect"]["pass_strength"])
    final = final * (1.0 - keep) + base_up * keep
    final = np.where(
        water,
        base_up,
        np.maximum(final, np.where(soft_land > 0, LAND_MIN_M, final)),
    )
    return {
        "job": job,
        "h": final.astype(np.float32),
        "inc": incision.astype(np.float16),
        "area": np.log2(np.maximum(area, 1.0)).astype(np.float16),
    }


def taper_weights(
    job: tuple[int, int, int, int], canvas: tuple[int, int], halo: int
) -> np.ndarray:
    """Poids de mélange d'une fenêtre : 1 au cœur, rampe sur le halo (sauf au bord du canevas)."""
    x0, y0, w, h = job
    ramp = np.linspace(0.0, 1.0, halo + 2, dtype=np.float32)[1:-1]
    wx = np.ones(w, dtype=np.float32)
    wy = np.ones(h, dtype=np.float32)
    if x0 > 0:
        wx[:halo] = ramp
    if x0 + w < canvas[0]:
        wx[-halo:] = ramp[::-1]
    if y0 > 0:
        wy[:halo] = ramp
    if y0 + h < canvas[1]:
        wy[-halo:] = ramp[::-1]
    return smoothstep(wy[:, None] * wx[None, :]) + 1e-4


def windows(
    canvas: tuple[int, int], block: int, halo: int
) -> list[tuple[int, int, int, int]]:
    """Fenêtres (cœur ``block`` px + halo) couvrant le canevas de travail."""
    out = []
    for cy in range(0, canvas[1], block):
        for cx in range(0, canvas[0], block):
            x0, y0 = max(cx - halo, 0), max(cy - halo, 0)
            x1, y1 = (
                min(cx + block + halo, canvas[0]),
                min(cy + block + halo, canvas[1]),
            )
            out.append((x0, y0, x1 - x0, y1 - y0))
    return out


def erode_canvas(
    base: np.ndarray,
    land: np.ndarray,
    cfg: dict,
    passes: list[tuple[float, float]],
    workers: int = 6,
    block: int | None = None,
    halo: int | None = None,
    log=print,
) -> dict:
    """Érode tout le canevas ; renvoie ``h`` (px de travail), ``inc``, ``area`` et les stats."""
    scale = int(cfg["work_scale"])
    e = cfg["erosion"]
    block = int(block or e["block_px"])
    halo = int(halo or e["halo_px"])
    canvas = (base.shape[1] * scale, base.shape[0] * scale)
    act = activation_from_height(base, cfg, cfg["activation"]["relief_radius_px"])
    act[~land & (base <= 0)] = 0.0
    pmask = pass_mask(base.shape, passes, float(cfg["protect"]["pass_radius_base_px"]))
    shared = {"cfg": cfg, "base": base, "act": act, "passes": pmask}
    jobs = windows(canvas, block, halo)
    active = [j for j in jobs]
    acc = np.zeros((canvas[1], canvas[0]), dtype=np.float32)
    wsum = np.zeros_like(acc)
    inc = np.zeros((canvas[1], canvas[0]), dtype=np.float16)
    area = np.zeros((canvas[1], canvas[0]), dtype=np.float16)
    t0 = time.time()

    def merge(result: dict) -> None:
        x0, y0, w, h = result["job"]
        wt = taper_weights(result["job"], canvas, halo)
        acc[y0 : y0 + h, x0 : x0 + w] += result["h"] * wt
        wsum[y0 : y0 + h, x0 : x0 + w] += wt
        better = wt > 0.5
        sl = (slice(y0, y0 + h), slice(x0, x0 + w))
        inc[sl] = np.where(better, result["inc"], inc[sl])
        area[sl] = np.where(better, result["area"], area[sl])

    if workers <= 1:
        init_worker(shared)
        for n, job in enumerate(active):
            merge(process_window(job))
            log(f"fenêtre {n + 1}/{len(active)} ({time.time() - t0:.0f} s)")
    else:
        with ProcessPoolExecutor(
            workers, initializer=init_worker, initargs=(shared,)
        ) as pool:
            for n, result in enumerate(pool.map(process_window, active)):
                merge(result)
                log(f"fenêtre {n + 1}/{len(active)} ({time.time() - t0:.0f} s)")
    acc /= wsum
    return {"h": acc, "inc": inc, "area": area, "windows": len(active), "act": act}


# ----------------------------------------------------------------------------
# Raffinement E2-E3, tuiles, manifeste
# ----------------------------------------------------------------------------


def refine_level(
    parent: np.ndarray,
    origin: tuple[int, int],
    level: int,
    cfg: dict,
    px_m: float,
) -> np.ndarray:
    """Un étage plus fin (x2) : bicubique + bruit guidé par la pente + talus."""
    r = cfg["refine"]
    idx = r["levels"].index(level)
    fine = ndimage.zoom(parent, 2, order=3, mode="nearest", grid_mode=True).astype(
        np.float32
    )
    act = activation_from_height(
        fine, cfg, max(2, int(cfg["activation"]["relief_radius_px"]))
    )
    slope = slope_field(fine, px_m, 1.5)
    drive = np.clip(slope - 0.04, 0.0, 0.9)
    waves = r["wavelength_px"][idx]
    weights = [0.65, 0.35][: len(waves)]
    for k, (wave, weight) in enumerate(zip(waves, weights, strict=False)):
        n = fractal_noise(
            fine.shape,
            origin,
            [wave],
            [1.0],
            r["ridged"],
            cfg["seed"] + 101 * level + k,
        )
        amp = np.minimum(r["slope_gain"] * drive * wave * px_m, r["max_amp_m"][idx])
        fine += (weight * amp * n * act).astype(np.float32)
    talus = float(np.tan(np.radians(cfg["erosion"]["talus_deg"] + 4.0)) * px_m)
    fixed = fine <= 0.0
    for _ in range(int(r["thermal_passes"])):
        thermal(fine, fixed, act, talus, 0.35)
    return np.where(parent_mask(parent, fine), fine, fine).astype(np.float32)


def parent_mask(parent: np.ndarray, fine: np.ndarray) -> np.ndarray:
    """Tout est valide (le masque de mer est géré par le plancher de terre)."""
    return np.ones(fine.shape, dtype=bool)


def rle_rows(present: set[tuple[int, int]]) -> list[dict]:
    """Tuiles présentes -> ``tiles_rle`` du manifeste."""
    rows: dict[int, list[int]] = {}
    for col, row in present:
        rows.setdefault(row, []).append(col)
    out = []
    for row in sorted(rows):
        cols = sorted(rows[row])
        runs, start, prev = [], cols[0], cols[0]
        for c in cols[1:]:
            if c != prev + 1:
                runs.append([start, prev - start + 1])
                start = c
            prev = c
        runs.append([start, prev - start + 1])
        out.append({"row": row, "runs": runs})
    return out


def write_tile(path: str, block: np.ndarray) -> None:
    """Écrit une tuile 16 bits."""
    terrain.write_png16(terrain.height_to_uint16(block), Path(path))


def _refine_job(args: tuple) -> tuple[int, int, int, set]:
    """Raffine une tuile E1 en E2/E3 et écrit les tuiles utiles."""
    col, row, e1_path, out_dir, levels = args
    cfg = _SHARED["cfg"]
    scale = int(cfg["work_scale"])
    px_m = NOMINAL_M / scale
    e1 = np.load(e1_path, mmap_mode="r")
    halo = 16
    x0, y0 = col * TILE_PX - halo, row * TILE_PX - halo
    sx0, sy0 = max(x0, 0), max(y0, 0)
    sx1, sy1 = (
        min(x0 + TILE_PX + 2 * halo, e1.shape[1]),
        min(y0 + TILE_PX + 2 * halo, e1.shape[0]),
    )
    parent = np.asarray(e1[sy0:sy1, sx0:sx1], dtype=np.float32)
    padl, padt = sx0 - x0, sy0 - y0
    parent = np.pad(
        parent,
        ((padt, y0 + TILE_PX + 2 * halo - sy1), (padl, x0 + TILE_PX + 2 * halo - sx1)),
        mode="edge",
    )
    written: dict[int, set] = {lv: set() for lv in levels}
    current, cur_origin, cur_halo = parent, (x0, y0), halo
    px = px_m
    for level in sorted(levels):
        px /= 2.0
        cur_origin = (cur_origin[0] * 2, cur_origin[1] * 2)
        current = refine_level(current, cur_origin, level, cfg, px)
        cur_halo *= 2
        n = 1 << (level - 1)
        inner = current[
            cur_halo : cur_halo + TILE_PX * n, cur_halo : cur_halo + TILE_PX * n
        ]
        for dj in range(n):
            for di in range(n):
                block = inner[
                    dj * TILE_PX : (dj + 1) * TILE_PX, di * TILE_PX : (di + 1) * TILE_PX
                ]
                if float(block.max()) <= 0.0:
                    continue
                c, r = col * n + di, row * n + dj
                directory = Path(out_dir) / f"E{level}"
                directory.mkdir(parents=True, exist_ok=True)
                write_tile(str(directory / f"{c}_{r}.png"), block)
                written[level].add((c, r))
    return col, row, 0, written


def write_pyramid(
    e1: np.ndarray,
    cfg: dict,
    out_dir: Path,
    scratch: Path,
    workers: int,
    shared: dict,
    log=print,
) -> dict:
    """Écrit E0 (moyenne 2x2), E1 et les étages raffinés ; manifeste ``relief_pyramid.json``."""
    t0 = time.time()
    out_dir.mkdir(parents=True, exist_ok=True)
    cols1, rows1 = e1.shape[1] // TILE_PX, e1.shape[0] // TILE_PX
    e0 = e1.reshape(e1.shape[0] // 2, 2, e1.shape[1] // 2, 2).mean(axis=(1, 3))
    e0_dir = out_dir / "E0"
    e0_dir.mkdir(exist_ok=True)
    jobs = []
    for r in range(rows1 // 2):
        for c in range(cols1 // 2):
            jobs.append(
                (
                    str(e0_dir / f"h_{c}_{r}.png"),
                    e0[
                        r * TILE_PX : (r + 1) * TILE_PX, c * TILE_PX : (c + 1) * TILE_PX
                    ],
                )
            )
    e1_dir = out_dir / "E1"
    e1_dir.mkdir(exist_ok=True)
    present1: set = set()
    for r in range(rows1):
        for c in range(cols1):
            block = e1[r * TILE_PX : (r + 1) * TILE_PX, c * TILE_PX : (c + 1) * TILE_PX]
            if float(block.max()) <= 0.0:
                continue
            present1.add((c, r))
            jobs.append((str(e1_dir / f"{c}_{r}.png"), block))
    with ProcessPoolExecutor(workers) as pool:
        list(pool.map(write_tile_pair, jobs, chunksize=8))
    log(f"E0/E1 écrits : {len(jobs)} tuiles ({time.time() - t0:.0f} s)")
    levels = list(cfg["refine"]["levels"])
    present: dict[int, set] = {1: present1}
    for lv in levels:
        present[lv] = set()
    if levels:
        e1_path = scratch / "e1.npy"
        np.save(e1_path, e1)
        rcfg = cfg["refine"]
        todo = []
        for c, r in sorted(present1):
            block = e1[r * TILE_PX : (r + 1) * TILE_PX, c * TILE_PX : (c + 1) * TILE_PX]
            if float(block.max() - max(block.min(), 0.0)) >= rcfg["min_relief_m"]:
                todo.append((c, r, str(e1_path), str(out_dir), levels))
        log(f"raffinement : {len(todo)} tuiles E1 sur {len(present1)}")
        with ProcessPoolExecutor(
            workers, initializer=init_worker, initargs=(shared,)
        ) as pool:
            for _, _, _, written in pool.map(_refine_job, todo, chunksize=2):
                for lv, tiles in written.items():
                    present[lv] |= tiles
        log(f"raffinement fini ({time.time() - t0:.0f} s)")
    scale = int(cfg["work_scale"])
    manifest = {
        "description": "Pyramide du relief composé érodé (lot CC CD, geo compose-erode). Généré, hors git.",
        "version": 1,
        "tile_px": TILE_PX,
        "root_tile_units": ROOT_TILE_UNITS,
        "root_origin_tiles": [0, 0],
        "height_min_m": terrain.HEIGHT_MIN_M,
        "height_range_m": terrain.HEIGHT_MAX_M - terrain.HEIGHT_MIN_M,
        "dir": ".",
        "pattern": "E{level}/{col}_{row}.png",
        "e0_dir": "E0",
        "e0_pattern": "h_{col}_{row}.png",
        "levels": [
            {
                "level": lv,
                "meters_per_px": round(NOMINAL_M / scale / (1 << (lv - 1)), 3),
                "source": "compose-erode",
                "tiles_rle": rle_rows(tiles),
            }
            for lv, tiles in sorted(present.items())
            if tiles
        ],
        "counts": {str(lv): len(t) for lv, t in sorted(present.items())},
    }
    (out_dir / "relief_pyramid.json").write_text(
        json.dumps(manifest, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return manifest["counts"]


def write_tile_pair(item: tuple[str, np.ndarray]) -> None:
    """Écrit une tuile ``(chemin, bloc)``."""
    write_tile(item[0], item[1])


# ----------------------------------------------------------------------------
# Orchestration
# ----------------------------------------------------------------------------


def compose_erode(
    cfg_path: Path | None = None,
    map_dir: Path = MAP_DIR,
    out_dir: Path | None = None,
    workers: int = 6,
    write_pyramid_files: bool = True,
    scratch: Path = RAW_DIR,
    progress: bool = True,
    reuse: bool = False,
) -> dict:
    """Érosion + pyramide ; renvoie le rapport (temps compris). ``reuse`` relit ``e1_eroded.npy``."""
    t0 = time.time()

    def log(msg: str) -> None:
        if progress:
            print(f"[erode] {msg} ({time.time() - t0:.0f} s)", flush=True)

    cfg = load_config(cfg_path)
    scratch.mkdir(parents=True, exist_ok=True)
    base, land = load_base(map_dir)
    passes = load_passes()
    log(f"relief d'entrée {base.shape[1]}x{base.shape[0]}, {len(passes)} cols")
    cached = scratch / "e1_eroded.npy"
    if reuse and cached.exists():
        result = {"h": np.load(cached), "windows": 0}
    else:
        result = erode_canvas(base, land, cfg, passes, workers, log=log)
    h = result["h"]
    log(
        f"érosion finie, E1 {h.shape[1]}x{h.shape[0]}, {float(h.min()):.0f}..{float(h.max()):.0f} m"
    )
    if "inc" in result:
        np.save(cached, h)
        np.save(scratch / "incision.npy", result["inc"])
        np.save(scratch / "area.npy", result["area"])
    report: dict = {
        "windows": result["windows"],
        "erode_seconds": round(time.time() - t0, 1),
    }
    if write_pyramid_files:
        shared = {"cfg": cfg}
        report["tiles"] = write_pyramid(
            h, cfg, out_dir or map_dir / "pyramid", scratch, workers, shared, log
        )
    report["seconds"] = round(time.time() - t0, 1)
    return report


# ----------------------------------------------------------------------------
# Planches 2D de contrôle (hors dépôt)
# ----------------------------------------------------------------------------


def hillshade_m(h: np.ndarray, px_m: float, exaggeration: float = 2.0) -> Image.Image:
    """Ombrage 2D multi-directions + hypsométrie (contrôle visuel)."""
    gy, gx = np.gradient(h.astype(np.float32), px_m)
    gx, gy = gx * exaggeration, gy * exaggeration
    norm = np.sqrt(gx**2 + gy**2 + 1.0)
    shade = np.zeros(h.shape, dtype=np.float32)
    for az, weight in ((315.0, 0.65), (20.0, 0.35)):
        a = np.radians(az)
        lx, ly, lz = np.sin(a) * 0.62, -np.cos(a) * 0.62, 0.78
        shade += weight * np.clip((-gx * lx - gy * ly + lz) / norm, 0, 1)
    t = np.clip(h / 3600.0, 0.0, 1.0)[..., None]
    low, mid, high = (
        np.array(c)
        for c in ((0.58, 0.70, 0.42), (0.66, 0.56, 0.38), (0.95, 0.94, 0.92))
    )
    color = np.where(
        t < 0.45,
        low + (mid - low) * (t / 0.45),
        mid + (high - mid) * np.clip((t - 0.45) / 0.55, 0, 1),
    )
    rgb = color * (0.25 + 0.85 * shade[..., None])
    sea = np.array([0.55, 0.72, 0.88]) * (
        1.0 - 0.4 * np.clip(-h / 150.0, 0, 1)[..., None]
    )
    rgb = np.where((h > 0)[..., None], rgb, sea)
    return Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8))


def before_after(
    base: np.ndarray, after: np.ndarray, scale: int, out_stem: Path, width: int = 900
) -> list[Path]:
    """Planches avant / après d'un même cadre (``after`` en pixels de travail), côté ``width`` px."""
    px_m = NOMINAL_M / scale
    before = upsample_window(
        base, 0, 0, base.shape[1] * scale, base.shape[0] * scale, scale, 3
    )
    paths = []
    for name, array in (("avant", before), ("apres", after)):
        cy, cx = array.shape[0] // 2, array.shape[1] // 2
        crop = array[
            max(cy - width // 2, 0) : cy + width // 2,
            max(cx - width // 2, 0) : cx + width // 2,
        ]
        path = out_stem.parent / f"{out_stem.name}_{name}.png"
        hillshade_m(crop, px_m).save(path)
        paths.append(path)
    return paths


# ----------------------------------------------------------------------------
# Masques roche / éboulis / neige (pente + incision)
# ----------------------------------------------------------------------------

DERIVED_DIR = COMPOSE_DIR / "derived"
EROSION_MASKS = "erosion_masks.png"


def erosion_masks(
    h: np.ndarray, incision: np.ndarray, cfg: dict, px_m: float, block: int = 1024
) -> np.ndarray:
    """Masques ``(rows, cols, 3)`` uint8 : R roche, G éboulis, B neige, au pixel de travail.

    Roche : pente forte, et lit des ravines (incision cumulée sur pente moyenne). Éboulis : pente
    moyenne au pied (jusqu'à ``scree_below_rock_px``) de la roche. Neige : au-dessus de la limite des
    neiges, retirée des parois. Calculé par blocs avec recouvrement (le canevas fait des Gpx).
    """
    m = cfg["masks"]
    out = np.zeros((*h.shape, 3), dtype=np.uint8)
    rock_lo, rock_hi = m["rock_slope_deg"]
    scree_lo, scree_hi = m["scree_slope_deg"]
    pad = int(m["scree_below_rock_px"]) + 2
    for y0 in range(0, h.shape[0], block):
        for x0 in range(0, h.shape[1], block):
            ys, ye = max(y0 - pad, 0), min(y0 + block + pad, h.shape[0])
            xs, xe = max(x0 - pad, 0), min(x0 + block + pad, h.shape[1])
            tile = np.asarray(h[ys:ye, xs:xe], dtype=np.float32)
            if float(tile.max()) < 120.0:
                continue
            inc = np.asarray(incision[ys:ye, xs:xe], dtype=np.float32)
            gy, gx = np.gradient(tile, px_m)
            deg = np.degrees(np.arctan(np.hypot(gx, gy)))
            rock = smoothstep((deg - rock_lo) / (rock_hi - rock_lo))
            gully = smoothstep(inc / m["incision_full_m"]) * smoothstep(
                (deg - 12.0) / 16.0
            )
            rock = np.maximum(rock, 0.85 * gully)
            near_rock = ndimage.maximum_filter(
                rock, size=2 * int(m["scree_below_rock_px"]) + 1
            )
            scree = (
                smoothstep((deg - scree_lo) / (scree_hi - scree_lo))
                * near_rock
                * (1.0 - rock)
            )
            snow = smoothstep((tile - m["snow_line_m"]) / m["snow_ramp_m"]) * (
                1.0
                - smoothstep(
                    (deg - m["snow_slope_max_deg"] * 0.7)
                    / (m["snow_slope_max_deg"] * 0.3)
                )
            )
            core = (
                slice(y0 - ys, y0 - ys + min(block, h.shape[0] - y0)),
                slice(x0 - xs, x0 - xs + min(block, h.shape[1] - x0)),
            )
            target = out[y0 : y0 + block, x0 : x0 + block]
            for channel, field in enumerate((rock, scree, snow)):
                target[..., channel] = np.rint(np.clip(field[core], 0, 1) * 255).astype(
                    np.uint8
                )
    return out


def write_masks(
    h: np.ndarray, incision: np.ndarray, cfg: dict, map_size: tuple[int, int]
) -> Path:
    """Écrit ``compose/derived/erosion_masks.png`` au pixel de carte (moyenne par blocs)."""
    scale = int(cfg["work_scale"])
    full = erosion_masks(h, incision, cfg, NOMINAL_M / scale)
    rows, cols = full.shape[0] // scale, full.shape[1] // scale
    small = (
        full[: rows * scale, : cols * scale]
        .reshape(rows, scale, cols, scale, 3)
        .astype(np.float32)
        .mean(axis=(1, 3))
    )
    small = small[: map_size[1], : map_size[0]]
    DERIVED_DIR.mkdir(parents=True, exist_ok=True)
    path = DERIVED_DIR / EROSION_MASKS
    Image.fromarray(np.rint(small).astype(np.uint8), mode="RGB").save(
        path, compress_level=9
    )
    return path
