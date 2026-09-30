"""GA3-L2 vegetation helpers: sheet splitting, cell fitting, normals, colour matching."""

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "blender_scripts"))

import ga3_vegetation_l2 as veg  # noqa: E402


def _sheet() -> np.ndarray:
    """4 x 2 sheet of opaque 'trees' (tall rectangles) of increasing width, plus a speck."""
    sheet = np.zeros((200, 400, 4), dtype=np.float32)
    for k in range(8):
        row, col = divmod(k, 4)
        x0 = col * 100 + 30
        y0 = row * 100 + 10
        sheet[y0 : y0 + 80, x0 : x0 + 20 + k * 2] = (0.2, 0.5, 0.1, 1.0)
    sheet[195:197, 5:7] = (0.1, 0.1, 0.1, 1.0)  # tiny shadow remnant
    return sheet


def test_split_sheet_orders_by_row_then_column() -> None:
    """Eight crops, reading order kept (widths grow with the panel index)."""
    crops = veg.split_sheet(_sheet())
    assert len(crops) == 8
    widths = [(c[..., 3] > 0.5).any(axis=0).sum() for c in crops]
    assert widths == sorted(widths)


def test_drop_specks_removes_small_blobs() -> None:
    """A detached blob far smaller than the tree is cleared and the crop tightened."""
    crop = np.zeros((100, 50, 4), dtype=np.float32)
    crop[0:80, 10:40] = (0.2, 0.5, 0.1, 1.0)
    crop[97:99, 2:4] = (0.1, 0.1, 0.1, 1.0)
    out = veg.drop_specks(crop)
    assert out.shape[0] == 80


def test_fit_cell_matches_box() -> None:
    """The fitted silhouette spans the target top/bottom and sits on the foot column."""
    crop = np.zeros((300, 120, 4), dtype=np.float32)
    crop[:, 40:80] = (0.3, 0.4, 0.1, 1.0)
    cell = veg.fit_cell(crop, (50.0, 200.0, 128.0))
    top, bottom, foot = veg.silhouette_box(cell)
    assert abs(top - 50) <= 2 and abs(bottom - 200) <= 2 and abs(foot - 128) <= 2


def test_crown_normals_dome_and_flat_outside() -> None:
    """Normals point outwards on the dome's left/right edges, flat (0.5, 0.5, 1) outside."""
    yy, xx = np.mgrid[0:64, 0:64]
    alpha = (((yy - 32) ** 2 + (xx - 32) ** 2) < 24**2).astype(np.float32)
    lum = np.full((64, 64), 0.1, dtype=np.float32)
    n = veg.crown_normals(alpha, lum)
    assert n.shape == (64, 64, 4)
    assert np.allclose(n[0, 0], (0.5, 0.5, 1.0, 1.0))
    assert (
        n[32, 12, 0] < 0.5 < n[32, 52, 0]
    )  # left edge faces left, right edge faces right


def test_match_mean_hits_target() -> None:
    """Joint opaque linear mean equals the FC2 row target after matching."""
    cell = np.zeros((16, 16, 4), dtype=np.float32)
    cell[4:12, 4:12] = (0.6, 0.7, 0.3, 1.0)
    target = np.array([0.09, 0.12, 0.046])
    out = veg.match_mean([cell, cell.copy()], target)
    lin = veg.srgb_to_linear(out[0][..., :3])[out[0][..., 3] > 0.5]
    assert np.allclose(lin.mean(axis=0), target, atol=0.005)
