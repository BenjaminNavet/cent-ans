"""Tests du cartogramme de la carte composée (lot CC, C0b) sur un jeu jouet."""

from __future__ import annotations

import numpy as np

from cent_ans_tools.compose.warp import Warp, diffuse_flow, nn_cv
from cent_ans_tools.geo.project import default_grid


def _toy_warp() -> tuple[Warp, np.ndarray]:
    """Densité à deux foyers sur une grille 96x80 ; grille de carte fictive."""
    ny, nx = 80, 96
    yy, xx = np.mgrid[0:ny, 0:nx].astype(float)
    rho = (
        0.3
        + 3.0 * np.exp(-((xx - 30) ** 2 + (yy - 30) ** 2) / (2 * 8**2))
        + 2.0 * np.exp(-((xx - 70) ** 2 + (yy - 50) ** 2) / (2 * 6**2))
    )
    rho /= rho.mean()
    fwd, info = diffuse_flow(rho, tol=0.03)
    assert info["residual_cv"] < 0.05
    from cent_ans_tools.compose.warp import _inverse_field

    warp = Warp((0, 0, nx * 8, ny * 8), 8, fwd, _inverse_field(fwd), {}, default_grid())
    return warp, rho


def test_no_fold() -> None:
    """La déformation ne replie pas la grille (jacobien positif)."""
    warp, _ = _toy_warp()
    assert warp.jacobian_det().min() > 0.0


def test_inverse_of_forward_is_identity() -> None:
    """L'inverse composé avec l'aller rend le point de départ."""
    warp, _ = _toy_warp()
    rng = np.random.default_rng(1)
    px = rng.uniform(40, 96 * 8 - 40, 300)
    py = rng.uniform(40, 80 * 8 - 40, 300)
    x, y = warp.forward_px(px, py)
    bx, by = warp.inverse_px(x, y)
    # moins d'un pixel de la grille de déformation (8 px pleine résolution)
    assert np.hypot(bx - px, by - py).max() < 8.0


def test_uniformity_improves() -> None:
    """Les lieux sont plus régulièrement espacés après déformation."""
    warp, rho = _toy_warp()
    rng = np.random.default_rng(2)
    p = (rho / rho.sum()).ravel()
    idx = rng.choice(p.size, size=250, replace=False, p=p)
    rows, cols = np.divmod(idx, rho.shape[1])
    px, py = (cols + 0.5) * 8.0, (rows + 0.5) * 8.0
    x, y = warp.forward_px(px, py)
    assert nn_cv(x, y) < nn_cv(px, py)
