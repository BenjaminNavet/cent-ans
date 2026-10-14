"""Cartogramme de diffusion de la carte composée (lot CC, C0b).

Méthode : flux rapide de Gastner, Seguy et More (2018). Une densité cible
``rho`` est posée sur une grille réduite de la carte actuelle ; elle diffuse
(équation de la chaleur, bords de Neumann, résolue par DCT) et chaque point de
la grille est advecté par ``v = -grad(phi) / phi``. Le résultat est une
déformation lisse et inversible ``réel -> carte composée``.

Conventions : la carte actuelle fait 7168 x 6144 px (719 m/px). La déformation
travaille sur l'emprise recadrée (``CROP_LONLAT``) à ``1/factor`` de cette
résolution. Les coordonnées « composées » sont en pixels pleine résolution,
origine au coin nord-ouest de l'emprise, et couvrent le même rectangle
(la déformation conserve l'aire totale).
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from scipy.fft import dctn, idctn
from scipy.interpolate import LinearNDInterpolator
from scipy.spatial import cKDTree

from cent_ans_tools.geo.project import MapGrid, grid_from_metadata
from cent_ans_tools.paths import DATA_DIR

Image.MAX_IMAGE_PIXELS = None

MAP_DIR = DATA_DIR / "map"
COMPOSE_DIR = MAP_DIR / "compose"
#: Emprise conservée (lon_min, lat_min, lon_max, lat_max) : sans Oural ni steppe.
CROP_LONLAT = (-11.0, 28.0, 45.0, 66.0)
#: Facteur de réduction de la grille de déformation (7168 / 8 = 896).
DEFAULT_FACTOR = 8
#: Kilomètres de carte par pixel pleine résolution.
KM_PER_PX = 0.7189765625

#: Zones de mesure de l'agrandissement (lon_min, lat_min, lon_max, lat_max).
ZONES: dict[str, tuple[float, float, float, float]] = {
    "France": (-4.5, 42.5, 8.0, 51.0),
    "Angleterre": (-5.5, 50.0, 1.8, 55.5),
    "Italie": (7.0, 37.0, 18.5, 46.5),
    "Ibérie": (-9.5, 36.0, 3.3, 43.8),
    "Empire": (6.0, 46.0, 19.5, 54.5),
    "Scandinavie": (5.0, 56.0, 30.0, 66.0),
    "Russie occidentale": (24.0, 50.0, 45.0, 66.0),
    "Balkans": (14.0, 38.0, 29.0, 46.0),
    "Anatolie": (26.0, 36.0, 45.0, 42.0),
    "Maghreb": (-11.0, 28.0, 12.0, 37.0),
}


# ----------------------------------------------------------------------------
# Lieux
# ----------------------------------------------------------------------------


def load_settlements() -> dict[str, dict]:
    """Tous les lieux de ``data/settlements`` par identifiant."""
    out: dict[str, dict] = {}
    for path in sorted((DATA_DIR / "settlements").glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(data, list):
            continue
        for item in data:
            if isinstance(item, dict) and "id" in item:
                out[item["id"]] = item
    return out


def _capital_places() -> list[dict]:
    places = []
    for path in sorted((DATA_DIR / "provinces").glob("prov_*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        cap = data.get("capital_city")
        if cap and "lon" in cap:
            places.append(
                {
                    "id": data["id"],
                    "lon": cap["lon"],
                    "lat": cap["lat"],
                    "kind": "city",
                }
            )
    return places


def load_places(selection_path: Path | None = None) -> tuple[list[dict], str]:
    """Lieux retenus : ``selection.json`` s'il existe, sinon les capitales.

    Returns:
        ``(lieux, origine)`` ; chaque lieu a ``id``, ``lon``, ``lat``, ``kind``.
    """
    path = selection_path or COMPOSE_DIR / "selection.json"
    if path.exists():
        try:
            places = _places_from_selection(json.loads(path.read_text("utf-8")))
            if len(places) >= 50:
                return places, "selection.json"
        except (ValueError, KeyError, TypeError):
            pass
    return _capital_places(), "capitales (repli)"


def _places_from_selection(data) -> list[dict]:
    settlements = load_settlements()
    items = data
    if isinstance(data, dict):
        for key in ("places", "settlements", "selected", "kept", "lieux"):
            if key in data:
                items = data[key]
                break
    if isinstance(items, dict):
        items = [
            {"id": k, **(v if isinstance(v, dict) else {})} for k, v in items.items()
        ]
    places = []
    for item in items:
        entry = {"id": item} if isinstance(item, str) else dict(item)
        base = settlements.get(entry.get("id", ""), {})
        lonlat = entry.get("lonlat") or base.get("lonlat")
        if lonlat is None and "lon" in entry:
            lonlat = [entry["lon"], entry["lat"]]
        if lonlat is None:
            continue
        places.append(
            {
                "id": entry.get("id", ""),
                "lon": float(lonlat[0]),
                "lat": float(lonlat[1]),
                "kind": entry.get("kind") or base.get("kind", "city"),
            }
        )
    return places


# ----------------------------------------------------------------------------
# Emprise
# ----------------------------------------------------------------------------


def map_grid() -> MapGrid:
    """Grille de la carte actuelle, lue dans ``map.json``."""
    return grid_from_metadata(json.loads((MAP_DIR / "map.json").read_text("utf-8")))


def crop_box_px(grid: MapGrid, factor: int) -> tuple[int, int, int, int]:
    """Rectangle pixel ``(x0, y0, x1, y1)`` englobant l'emprise, multiple de ``factor``."""
    lon0, lat0, lon1, lat1 = CROP_LONLAT
    t = np.linspace(0.0, 1.0, 400)
    lons = np.concatenate(
        [
            lon0 + t * (lon1 - lon0),
            np.full(400, lon1),
            lon0 + t * (lon1 - lon0),
            np.full(400, lon0),
        ]
    )
    lats = np.concatenate(
        [
            np.full(400, lat0),
            lat0 + t * (lat1 - lat0),
            np.full(400, lat1),
            lat0 + t * (lat1 - lat0),
        ]
    )
    px, py = grid.lonlat_to_pixel(lons, lats)
    x0 = int(np.floor(max(px.min(), 0) / factor)) * factor
    y0 = int(np.floor(max(py.min(), 0) / factor)) * factor
    x1 = int(np.ceil(min(px.max(), grid.width_px) / factor)) * factor
    y1 = int(np.ceil(min(py.max(), grid.height_px) / factor)) * factor
    return x0, y0, x1, y1


# ----------------------------------------------------------------------------
# Déformation
# ----------------------------------------------------------------------------


def _sample(field: np.ndarray, rows, cols) -> np.ndarray:
    rows, cols = np.asarray(rows, dtype=float), np.asarray(cols, dtype=float)
    out = ndimage.map_coordinates(
        field, [np.atleast_1d(rows), np.atleast_1d(cols)], order=1, mode="nearest"
    )
    return out.reshape(rows.shape)


@dataclass
class Warp:
    """Déformation ``réel -> composé`` (grille avant + inverse).

    Attributes:
        box: ``(x0, y0, x1, y1)`` emprise en pixels pleine résolution.
        factor: Réduction de la grille de déformation.
        fwd: ``(2, ny, nx)`` position composée (indices de cellule, centre = i)
            de chaque cellule réelle ; axe 0 = (colonne, ligne).
        inv: Même forme, position réelle de chaque cellule composée.
        params: Paramètres de fabrication (traçabilité).
    """

    box: tuple[int, int, int, int]
    factor: int
    fwd: np.ndarray
    inv: np.ndarray
    params: dict
    grid: MapGrid

    # -- conversions -------------------------------------------------------
    @property
    def size_px(self) -> tuple[int, int]:
        """Largeur et hauteur de l'emprise en pixels pleine résolution."""
        return self.box[2] - self.box[0], self.box[3] - self.box[1]

    def _to_idx(self, px: np.ndarray, py: np.ndarray):
        f = self.factor
        return (np.asarray(py) - self.box[1]) / f - 0.5, (
            np.asarray(px) - self.box[0]
        ) / f - 0.5

    def _jacobian(self, field: np.ndarray, rows, cols):
        gy_c, gx_c = np.gradient(field[0])
        gy_r, gx_r = np.gradient(field[1])
        s = lambda a: _sample(a, rows, cols)  # noqa: E731
        return s(gx_c), s(gy_c), s(gx_r), s(gy_r)

    # -- API publique -------------------------------------------------------
    def forward_px(self, px, py):
        """Pixels pleine résolution réels -> pixels composés (origine = emprise)."""
        rows, cols = self._to_idx(px, py)
        cc = _sample(self.fwd[0], rows, cols)
        rr = _sample(self.fwd[1], rows, cols)
        return (cc + 0.5) * self.factor, (rr + 0.5) * self.factor

    def forward(self, lon, lat):
        """``(lon, lat) -> (x, y)`` en pixels composés."""
        px, py = self.grid.lonlat_to_pixel(lon, lat)
        return self.forward_px(px, py)

    def inverse_px(self, x, y, newton: int = 3):
        """Pixels composés -> pixels pleine résolution réels (Newton affiné)."""
        f = self.factor
        tc = np.asarray(x, dtype=float) / f - 0.5
        tr = np.asarray(y, dtype=float) / f - 0.5
        cols = _sample(self.inv[0], tr, tc)
        rows = _sample(self.inv[1], tr, tc)
        for _ in range(newton):
            fc = _sample(self.fwd[0], rows, cols)
            fr = _sample(self.fwd[1], rows, cols)
            a, b, c, d = self._jacobian(self.fwd, rows, cols)
            det = a * d - b * c
            det = np.where(np.abs(det) < 1e-9, 1e-9, det)
            ec, er = fc - tc, fr - tr
            cols = cols - (d * ec - b * er) / det
            rows = rows - (-c * ec + a * er) / det
        px = (cols + 0.5) * f + self.box[0]
        py = (rows + 0.5) * f + self.box[1]
        return px, py

    def inverse(self, x, y):
        """``(x, y)`` composés -> ``(lon, lat)``."""
        px, py = self.inverse_px(x, y)
        return self.grid.pixel_to_lonlat(px, py)

    def jacobian_det(self) -> np.ndarray:
        """Déterminant du jacobien de ``fwd`` en cellules (aire locale composée / réelle)."""
        gy_c, gx_c = np.gradient(self.fwd[0])
        gy_r, gx_r = np.gradient(self.fwd[1])
        return gx_c * gy_r - gy_c * gx_r

    # -- sérialisation ------------------------------------------------------
    def save(self, path: Path) -> None:
        """Écrit un ``.npz`` (grilles float32 + métadonnées)."""
        path.parent.mkdir(parents=True, exist_ok=True)
        np.savez_compressed(
            path,
            fwd=self.fwd.astype(np.float32),
            inv=self.inv.astype(np.float32),
            meta=json.dumps(
                {"box": self.box, "factor": self.factor, "params": self.params}
            ),
        )

    @classmethod
    def load(cls, path: Path, grid: MapGrid | None = None) -> Warp:
        """Relit un ``.npz`` écrit par :meth:`save`."""
        data = np.load(path)
        meta = json.loads(str(data["meta"]))
        return cls(
            tuple(meta["box"]),
            meta["factor"],
            data["fwd"].astype(float),
            data["inv"].astype(float),
            meta["params"],
            grid or map_grid(),
        )

    @classmethod
    def identity(
        cls, grid: MapGrid | None = None, factor: int = DEFAULT_FACTOR
    ) -> Warp:
        """Déformation nulle sur la même emprise (carte réelle recadrée)."""
        grid = grid or map_grid()
        box = crop_box_px(grid, factor)
        ny, nx = (box[3] - box[1]) // factor, (box[2] - box[0]) // factor
        rr, cc = np.mgrid[0:ny, 0:nx].astype(float)
        field = np.stack([cc, rr])
        return cls(box, factor, field, field.copy(), {"identity": True}, grid)


def _inverse_field(fwd: np.ndarray) -> np.ndarray:
    """Grille inverse : triangulation (sous-échantillonnée) de ``fwd``."""
    _, ny, nx = fwd.shape
    step = 2
    rr, cc = np.mgrid[0:ny:step, 0:nx:step]
    pts = np.stack(
        [fwd[0][::step, ::step].ravel(), fwd[1][::step, ::step].ravel()], axis=1
    )
    out = []
    gy, gx = np.mgrid[0:ny, 0:nx]
    for values in (cc.ravel().astype(float), rr.ravel().astype(float)):
        interp = LinearNDInterpolator(pts, values)
        res = interp(gx.ravel(), gy.ravel()).reshape(ny, nx)
        if np.isnan(res).any():
            nearest = cKDTree(pts)
            bad = np.isnan(res)
            _, idx = nearest.query(np.stack([gx[bad], gy[bad]], axis=1))
            res[bad] = values[idx]
        out.append(res)
    return np.stack(out)


def build_density(
    grid: MapGrid,
    box: tuple[int, int, int, int],
    factor: int,
    places: list[dict],
    strength: float,
    sigma_km: float,
    sea_density: float,
    outside_density: float = 0.08,
) -> np.ndarray:
    """Densité cible (ny, nx) : mélange uniforme / lieux, mer et hors-emprise réduits."""
    x0, y0, x1, y1 = box
    nx, ny = (x1 - x0) // factor, (y1 - y0) // factor
    cols = (np.arange(nx) + 0.5) * factor + x0
    rows = (np.arange(ny) + 0.5) * factor + y0
    lon, lat = grid.pixel_to_lonlat(*np.meshgrid(cols, rows))
    lon0, lat0, lon1, lat1 = CROP_LONLAT
    inside = (lon >= lon0) & (lon <= lon1) & (lat >= lat0) & (lat <= lat1)

    # Terre / mer à la résolution de la grille
    land_img = Image.open(MAP_DIR / "land_mask.png").crop((x0, y0, x1, y1))
    land = np.asarray(land_img.resize((nx, ny), Image.BOX), dtype=np.float32) > 127

    lon_p = np.array([p["lon"] for p in places])
    lat_p = np.array([p["lat"] for p in places])
    px, py = grid.lonlat_to_pixel(lon_p, lat_p)
    hist = np.zeros((ny, nx))
    ci = np.clip(((px - x0) / factor).astype(int), 0, nx - 1)
    ri = np.clip(((py - y0) / factor).astype(int), 0, ny - 1)
    np.add.at(hist, (ri, ci), 1.0)
    sigma_cells = max(sigma_km / KM_PER_PX / factor, 0.5)
    places_density = ndimage.gaussian_filter(hist, sigma_cells, mode="constant")
    places_density /= places_density[inside].mean()

    rho = (1.0 - strength) + strength * places_density
    rho = np.maximum(rho, 0.02)
    land_mean = rho[inside & land].mean()
    rho = np.where(
        land,
        rho,
        np.minimum(rho, sea_density * land_mean)
        if strength > 0
        else sea_density * land_mean,
    )
    rho = np.where(inside, rho, outside_density * land_mean)
    rho = ndimage.gaussian_filter(rho, 1.5, mode="nearest")
    return rho / rho.mean()


def diffuse_flow(
    rho: np.ndarray, tol: float = 0.03, max_steps: int = 4000, progress: bool = False
) -> tuple[np.ndarray, dict]:
    """Flux rapide : positions finales (2, ny, nx) des centres de cellule.

    Axe 0 = (colonne, ligne), en indices de cellule (centre de la cellule i = i).
    """
    ny, nx = rho.shape
    coeffs = dctn(rho, type=2, norm="ortho")
    ky = (np.pi * np.arange(ny) / ny) ** 2
    kx = (np.pi * np.arange(nx) / nx) ** 2
    eig = ky[:, None] + kx[None, :]

    def phi_at(t: float) -> np.ndarray:
        return idctn(coeffs * np.exp(-eig * t), type=2, norm="ortho", workers=-1)

    def velocity(t: float, cols: np.ndarray, rows: np.ndarray):
        phi = np.maximum(phi_at(t), 1e-6)
        gy, gx = np.gradient(phi)
        vx, vy = -gx / phi, -gy / phi
        vx[:, 0] = vx[:, -1] = 0.0
        vy[0, :] = vy[-1, :] = 0.0
        sample = lambda a: ndimage.map_coordinates(
            a, [rows, cols], order=1, mode="nearest"
        )  # noqa: E731
        return sample(vx), sample(vy), phi

    gy0, gx0 = np.mgrid[0:ny, 0:nx].astype(float)
    cols, rows = gx0.ravel().copy(), gy0.ravel().copy()
    t, steps = 0.0, 0
    ratio = np.inf
    while steps < max_steps:
        vx, vy, phi = velocity(t, cols, rows)
        ratio = float(phi.std() / phi.mean())
        if ratio < tol:
            break
        vmax = max(np.abs(vx).max(), np.abs(vy).max(), 1e-9)
        dt = min(0.5 / vmax, 0.15 * t + 0.05)
        mx, my, _ = velocity(
            t + dt / 2,
            np.clip(cols + vx * dt / 2, -0.5, nx - 0.5),
            np.clip(rows + vy * dt / 2, -0.5, ny - 0.5),
        )
        cols = np.clip(cols + mx * dt, -0.5, nx - 0.5)
        rows = np.clip(rows + my * dt, -0.5, ny - 0.5)
        t += dt
        steps += 1
        if progress and steps % 50 == 0:
            print(
                f"  flux: pas {steps}, t={t:.1f}, cv={ratio:.3f}, vmax={vmax:.3f}",
                flush=True,
            )
    field = np.stack([cols.reshape(ny, nx), rows.reshape(ny, nx)])
    return field, {"steps": steps, "t_end": t, "residual_cv": ratio}


def build_warp(
    strength: float = 0.5,
    sigma_km: float = 150.0,
    sea_density: float = 0.6,
    outside_density: float = 0.08,
    factor: int = DEFAULT_FACTOR,
    places: list[dict] | None = None,
    grid: MapGrid | None = None,
    progress: bool = False,
) -> Warp:
    """Construit la déformation complète (voir le module)."""
    grid = grid or map_grid()
    origin = "fourni"
    if places is None:
        places, origin = load_places()
    box = crop_box_px(grid, factor)
    rho = build_density(
        grid, box, factor, places, strength, sigma_km, sea_density, outside_density
    )
    fwd, info = diffuse_flow(rho, progress=progress)
    inv = _inverse_field(fwd)
    params = {
        "strength": strength,
        "sigma_km": sigma_km,
        "sea_density": sea_density,
        "outside_density": outside_density,
        "places": len(places),
        "places_origin": origin,
        **info,
    }
    return Warp(box, factor, fwd, inv, params, grid)


# ----------------------------------------------------------------------------
# Mesures
# ----------------------------------------------------------------------------


def nn_cv(xs: np.ndarray, ys: np.ndarray) -> float:
    """Coefficient de variation de la distance au plus proche voisin."""
    pts = np.stack([xs, ys], axis=1)
    dist, _ = cKDTree(pts).query(pts, k=2)
    d = dist[:, 1]
    return float(d.std() / d.mean())


def uniformity(warp: Warp, places: list[dict]) -> dict:
    """CV de la distance au plus proche voisin des lieux, avant / après."""
    lon = np.array([p["lon"] for p in places])
    lat = np.array([p["lat"] for p in places])
    lon0, lat0, lon1, lat1 = CROP_LONLAT
    keep = (lon >= lon0) & (lon <= lon1) & (lat >= lat0) & (lat <= lat1)
    lon, lat = lon[keep], lat[keep]
    bx, by = warp.grid.lonlat_to_pixel(lon, lat)
    ax, ay = warp.forward(lon, lat)
    return {"n": int(keep.sum()), "cv_before": nn_cv(bx, by), "cv_after": nn_cv(ax, ay)}


def zone_factors(warp: Warp) -> dict[str, float]:
    """Facteur d'agrandissement linéaire moyen (racine du rapport d'aires) par zone."""
    from shapely.geometry import Polygon

    out = {}
    t = np.linspace(0, 1, 60, endpoint=False)
    for name, (a, b, c, d) in ZONES.items():
        lons = np.concatenate(
            [a + t * (c - a), np.full(60, c), c - t * (c - a), np.full(60, a)]
        )
        lats = np.concatenate(
            [np.full(60, b), b + t * (d - b), np.full(60, d), d - t * (d - b)]
        )
        bx, by = warp.grid.lonlat_to_pixel(lons, lats)
        ax, ay = warp.forward(lons, lats)
        before = Polygon(np.stack([bx, by], 1)).area
        after = Polygon(np.stack([ax, ay], 1)).area
        out[name] = float(np.sqrt(after / before)) if before > 0 else float("nan")
    return out
