"""Contrat de la déformation de la carte composée (lot CC, C1).

Format versionné lu par Python, Rust et GDScript (un seul lecteur par langage) :

- ``data/map/compose/warp.json`` : paramètres, ``real_grid`` (ancienne grille
  EPSG:3035), ``crop_box_px``, ``factor``, ``size_px`` et les deux fichiers ;
- ``warp_forward.f32`` : float32 little-endian, forme ``(ny, nx, 2)``, ligne 0 =
  nord. Nœud ``(j, i)`` = centre de la cellule réelle ``(x0 + (i + 0.5) * factor,
  y0 + (j + 0.5) * factor)`` (pixels réels pleine résolution) ; valeur = position
  composée ``(x, y)`` en pixels pleine résolution ;
- ``warp_inverse.f32`` : même format ; nœud ``(j, i)`` = centre de la cellule
  composée ``((i + 0.5) * factor, (j + 0.5) * factor)`` ; valeur = position réelle
  ``(x, y)`` en pixels pleine résolution de l'ancienne grille.

Pixels continus : le centre du pixel ``(i, j)`` est ``(i + 0.5, j + 0.5)``.
Interpolation bilinéaire ; au-delà du dernier centre de nœud mais dans
l'emprise, la dernière cellule est prolongée linéairement ; hors emprise le
résultat est NaN.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from cent_ans_tools.compose.warp import Warp
from cent_ans_tools.geo.project import MapGrid
from cent_ans_tools.paths import DATA_DIR

COMPOSE_DIR = DATA_DIR / "map" / "compose"
WARP_JSON = "warp.json"
FORWARD_FILE = "warp_forward.f32"
INVERSE_FILE = "warp_inverse.f32"
FORMAT_VERSION = 1


def bilinear(grid: np.ndarray, u: np.ndarray, v: np.ndarray) -> np.ndarray:
    """Interpolation bilinéaire de ``grid`` ``(ny, nx, 2)`` aux indices flottants ``(u, v)``.

    ``u`` = colonne, ``v`` = ligne (centre du nœud ``i`` = ``i``). Les indices hors
    de ``[0, n - 1]`` prolongent linéairement la dernière cellule.
    """
    ny, nx = grid.shape[:2]
    u = np.asarray(u, dtype=np.float64)
    v = np.asarray(v, dtype=np.float64)
    i0 = np.clip(np.floor(u).astype(np.int64), 0, nx - 2)
    j0 = np.clip(np.floor(v).astype(np.int64), 0, ny - 2)
    tu = (u - i0)[..., None]
    tv = (v - j0)[..., None]
    g00 = grid[j0, i0]
    g01 = grid[j0, i0 + 1]
    g10 = grid[j0 + 1, i0]
    g11 = grid[j0 + 1, i0 + 1]
    return (g00 * (1 - tu) + g01 * tu) * (1 - tv) + (g10 * (1 - tu) + g11 * tu) * tv


@dataclass
class ComposedWarp:
    """Déformation ``réel -> composé`` telle que stockée dans le contrat.

    Attributes:
        real_grid: Bloc ``real_grid`` de ``warp.json`` (CRS, bornes, taille).
        crop_box_px: ``(x0, y0, x1, y1)`` emprise en pixels réels pleine résolution.
        factor: Réduction des grilles (pixels pleine résolution par nœud).
        size_px: ``(largeur, hauteur)`` du canevas composé.
        forward_grid: ``(ny, nx, 2)`` float32.
        inverse_grid: ``(ny, nx, 2)`` float32.
        params: Paramètres de fabrication.
    """

    real_grid: dict
    crop_box_px: tuple[int, int, int, int]
    factor: int
    size_px: tuple[int, int]
    forward_grid: np.ndarray = field(repr=False)
    inverse_grid: np.ndarray = field(repr=False)
    params: dict = field(default_factory=dict)

    # -- conversions pixels --------------------------------------------------
    def forward_px(self, px, py) -> tuple[np.ndarray, np.ndarray]:
        """Pixels réels pleine résolution -> pixels composés (NaN hors emprise)."""
        x0, y0, x1, y1 = self.crop_box_px
        px = np.asarray(px, dtype=np.float64)
        py = np.asarray(py, dtype=np.float64)
        u = (px - x0) / self.factor - 0.5
        v = (py - y0) / self.factor - 0.5
        out = bilinear(self.forward_grid, u, v)
        outside = (px < x0) | (px >= x1) | (py < y0) | (py >= y1)
        out = np.where(outside[..., None], np.nan, out)
        return out[..., 0], out[..., 1]

    def inverse_px(self, x, y) -> tuple[np.ndarray, np.ndarray]:
        """Pixels composés -> pixels réels pleine résolution (NaN hors canevas)."""
        w, h = self.size_px
        x = np.asarray(x, dtype=np.float64)
        y = np.asarray(y, dtype=np.float64)
        out = bilinear(self.inverse_grid, x / self.factor - 0.5, y / self.factor - 0.5)
        outside = (x < 0) | (x >= w) | (y < 0) | (y >= h)
        out = np.where(outside[..., None], np.nan, out)
        return out[..., 0], out[..., 1]

    def refine_inverse(self, iterations: int = 4) -> None:
        """Newton sur les nœuds de l'inverse : ``forward(inverse(nœud)) == nœud``.

        Le contrat est lu avec la même interpolation bilinéaire dans les trois
        langages ; on rend donc l'inverse exact par rapport à l'aller *stocké*.
        """
        f = self.factor
        ny, nx = self.inverse_grid.shape[:2]
        jj, ii = np.mgrid[0:ny, 0:nx]
        tx, ty = (ii + 0.5) * f, (jj + 0.5) * f
        inv = self.inverse_grid.astype(np.float64)
        eps = 0.5
        for _ in range(iterations):
            px, py = inv[..., 0], inv[..., 1]
            fx, fy = self.forward_px(px, py)
            ax, ay = self.forward_px(px + eps, py)
            bx, by = self.forward_px(px, py + eps)
            a, c = (ax - fx) / eps, (ay - fy) / eps
            b, d = (bx - fx) / eps, (by - fy) / eps
            det = a * d - b * c
            det = np.where(np.abs(det) < 1e-9, 1e-9, det)
            ex, ey = fx - tx, fy - ty
            step_x = (d * ex - b * ey) / det
            step_y = (-c * ex + a * ey) / det
            ok = np.isfinite(step_x) & np.isfinite(step_y)
            inv[..., 0] = np.where(ok, px - step_x, px)
            inv[..., 1] = np.where(ok, py - step_y, py)
        self.inverse_grid = inv.astype("<f4")

    @property
    def box(self) -> tuple[int, int, int, int]:
        """Alias de ``crop_box_px`` (compatibilité avec :class:`Warp`)."""
        return self.crop_box_px

    @property
    def grid(self) -> MapGrid:
        """Alias de :meth:`real_map_grid` (compatibilité avec :class:`Warp`)."""
        return self.real_map_grid()

    def forward(self, lon, lat) -> tuple[np.ndarray, np.ndarray]:
        """``(lon, lat) -> (x, y)`` en pixels composés."""
        px, py = self.grid.lonlat_to_pixel(lon, lat)
        return self.forward_px(px, py)

    def inverse(self, x, y) -> tuple[np.ndarray, np.ndarray]:
        """``(x, y)`` composés -> ``(lon, lat)``."""
        px, py = self.inverse_px(x, y)
        return self.grid.pixel_to_lonlat(px, py)

    # -- (de)sérialisation ---------------------------------------------------
    @classmethod
    def from_warp(cls, warp: Warp) -> ComposedWarp:
        """Convertit un :class:`Warp` du cartogramme (indices de cellule) en contrat."""
        f = warp.factor
        x0, y0 = warp.box[:2]
        forward = np.stack([(warp.fwd[0] + 0.5) * f, (warp.fwd[1] + 0.5) * f], axis=-1)
        inverse = np.stack(
            [(warp.inv[0] + 0.5) * f + x0, (warp.inv[1] + 0.5) * f + y0], axis=-1
        )
        grid = warp.grid
        real_grid = {
            "crs": "EPSG:3035",
            "bounds_projected": [float(b) for b in grid.bounds],
            "size_px": [grid.width_px, grid.height_px],
            "meters_per_px": grid.meters_per_px,
            "origin_xy_m": [float(grid.bounds[0]), float(grid.bounds[3])],
        }
        return cls(
            real_grid,
            tuple(int(v) for v in warp.box),
            int(f),
            tuple(int(v) for v in warp.size_px),
            forward.astype("<f4"),
            inverse.astype("<f4"),
            dict(warp.params),
        )

    def save(self, directory: Path = COMPOSE_DIR) -> None:
        """Écrit ``warp.json`` et les deux grilles ``.f32``."""
        directory.mkdir(parents=True, exist_ok=True)
        (directory / FORWARD_FILE).write_bytes(
            np.ascontiguousarray(self.forward_grid, dtype="<f4").tobytes()
        )
        (directory / INVERSE_FILE).write_bytes(
            np.ascontiguousarray(self.inverse_grid, dtype="<f4").tobytes()
        )
        doc = {
            "version": FORMAT_VERSION,
            "params": _jsonable(self.params),
            "real_grid": self.real_grid,
            "crop_box_px": list(self.crop_box_px),
            "factor": self.factor,
            "size_px": list(self.size_px),
            "forward": {
                "file": FORWARD_FILE,
                "shape": list(self.forward_grid.shape),
                "dtype": "<f4",
            },
            "inverse": {
                "file": INVERSE_FILE,
                "shape": list(self.inverse_grid.shape),
                "dtype": "<f4",
            },
        }
        (directory / WARP_JSON).write_text(
            json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )

    @classmethod
    def load(cls, directory: Path = COMPOSE_DIR) -> ComposedWarp:
        """Relit le contrat écrit par :meth:`save`."""
        doc = json.loads((directory / WARP_JSON).read_text(encoding="utf-8"))

        def read(entry: dict) -> np.ndarray:
            data = np.frombuffer((directory / entry["file"]).read_bytes(), dtype="<f4")
            return data.reshape(entry["shape"])

        return cls(
            doc["real_grid"],
            tuple(doc["crop_box_px"]),
            int(doc["factor"]),
            tuple(doc["size_px"]),
            read(doc["forward"]),
            read(doc["inverse"]),
            doc.get("params", {}),
        )

    # -- accès aux grilles ---------------------------------------------------
    def real_map_grid(self) -> MapGrid:
        """L'ancienne grille EPSG:3035 (lecture des rasters réels)."""
        rg = self.real_grid
        return MapGrid(
            tuple(rg["bounds_projected"]), rg["size_px"][0], rg["size_px"][1]
        )


def _jsonable(value):
    """Convertit les scalaires numpy d'un dictionnaire de paramètres en JSON."""
    if isinstance(value, dict):
        return {k: _jsonable(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [_jsonable(v) for v in value]
    if isinstance(value, np.generic):
        return value.item()
    return value
