"""Tileable copies by min-error seam cuts, ported from ground_materials (T1b)."""

from __future__ import annotations

import numpy as np
from scipy.ndimage import gaussian_filter


def _loop_cut(error: np.ndarray) -> np.ndarray:
    """Minimum-error path through ``error`` (rows x cols), one column per row.

    Steps of at most one column between rows; the path starts and ends on the same
    column (the best one of the first row) so it closes when the image wraps.
    """
    rows, cols = error.shape
    start = int(np.argmin(error[0]))
    cost = np.full(cols, np.inf)
    cost[start] = error[0, start]
    back = np.zeros((rows, cols), dtype=np.int64)
    index = np.arange(cols)
    for row in range(1, rows):
        left = np.concatenate(([np.inf], cost[:-1]))
        right = np.concatenate((cost[1:], [np.inf]))
        stacked = np.stack([left, cost, right])
        choice = np.argmin(stacked, axis=0)
        back[row] = index + choice - 1
        cost = stacked[choice, index] + error[row]
    path = np.empty(rows, dtype=np.int64)
    path[-1] = start
    for row in range(rows - 1, 0, -1):
        path[row - 1] = back[row, path[row]]
    return path


def _seam_mask(
    error: np.ndarray, centre: int, half_band: int, feather: float
) -> np.ndarray:
    """Weight (0..1) along columns: 1 between two loop cuts around column ``centre``.

    ``error`` is rows x cols; the left cut runs in ``[centre - half_band, centre - margin)``,
    the right one in ``[centre + margin, centre + half_band)``.
    """
    rows, cols = error.shape
    margin = int(np.ceil(3 * feather)) + 1  # keep the feathered cut off the seam itself
    left = (
        _loop_cut(error[:, centre - half_band : centre - margin]) + centre - half_band
    )
    right = _loop_cut(error[:, centre + margin : centre + half_band]) + centre + margin
    columns = np.arange(cols)[np.newaxis, :]
    mask = (
        (columns >= left[:, np.newaxis]) & (columns <= right[:, np.newaxis])
    ).astype(np.float64)
    if feather > 0:
        mask = gaussian_filter(mask, feather, mode="wrap")
    return mask


def make_seamless(
    image: np.ndarray, half_band: int, feather: float = 1.5
) -> np.ndarray:
    """Seamlessly tileable copy of ``image`` (H x W x 3 float) by min-error cuts.

    After a half-tile roll the original borders meet on a central cross. The vertical arm
    is replaced, between two minimum-error cuts, by a copy rolled on rows only (continuous
    across that arm), the horizontal arm by a copy rolled on columns only, and their
    crossing by the unrolled source. Cuts close on themselves so the result wraps.
    """
    height, width = image.shape[:2]
    if not 16 <= half_band < min(height, width) // 2:
        raise ValueError(f"demi-bande hors bornes : {half_band}")
    source = image.astype(np.float64)
    rolled_xy = np.roll(source, (height // 2, width // 2), axis=(0, 1))
    rolled_y = np.roll(source, height // 2, axis=0)
    rolled_x = np.roll(source, width // 2, axis=1)
    error_x = ((rolled_xy - rolled_y) ** 2).sum(axis=-1)
    weight_x = _seam_mask(error_x, width // 2, half_band, feather)
    error_y = (1 - weight_x) * ((rolled_xy - rolled_x) ** 2).sum(axis=-1) + weight_x * (
        (rolled_y - source) ** 2
    ).sum(axis=-1)
    weight_y = _seam_mask(error_y.T, height // 2, half_band, feather).T
    wx = weight_x[..., np.newaxis]
    wy = weight_y[..., np.newaxis]
    return (
        rolled_xy * (1 - wx) * (1 - wy)
        + rolled_y * wx * (1 - wy)
        + rolled_x * (1 - wx) * wy
        + source * wx * wy
    )


def low_pass(values: np.ndarray, sigma: float, mode: str) -> np.ndarray:
    """Wide Gaussian blur of a H x W (x C) image, computed at reduced resolution.

    The image is block-averaged by ``factor`` (sigma / 4, power of two), blurred there and
    brought back by linear interpolation: a sigma of ~150 px costs milliseconds.
    """
    from scipy.ndimage import zoom

    height, width = values.shape[:2]
    factor = 1
    while (
        factor * 2 <= sigma / 4
        and height % (factor * 2) == 0
        and width % (factor * 2) == 0
    ):
        factor *= 2
    extra = values.shape[2:]
    small = values.reshape(
        height // factor, factor, width // factor, factor, *extra
    ).mean(axis=(1, 3))
    sigmas = (sigma / factor, sigma / factor) + (0,) * len(extra)
    small = gaussian_filter(small, sigmas, mode=mode)
    if factor == 1:
        return small
    zoom_mode = "grid-wrap" if mode == "wrap" else "nearest"
    factors = (factor, factor) + (1,) * len(extra)
    return zoom(small, factors, order=1, mode=zoom_mode, grid_mode=True)
