"""Snapping polylines onto the valley floor of a relief (lot ZG5a, ADR 0036).

Pure geometry on EPSG:3035 metres, independent of the pyramid: the caller gives a
height sampler ``sample(x, y) -> heights`` (vectorised).

- :func:`resample` spaces the vertices of a line evenly;
- :func:`snap_to_valley` moves each vertex along the normal of the (smoothed) line to
  the valley floor: a Viterbi search over lateral offsets minimising the floor
  height, a prior towards the source position and a penalty on lateral changes, so
  that the result follows the thalweg without zig-zagging;
- :func:`isotonic_decreasing` fits the water level to the floor heights under the
  constraint that it never rises downstream (pool adjacent violators);
- :func:`drape` smooths the heights of a road along its length (slopes eased,
  bumps of the relief kept within a few metres).
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

import numpy as np
from scipy import ndimage

Sampler = Callable[[np.ndarray, np.ndarray], np.ndarray]


@dataclass(frozen=True)
class SnapParams:
    """Tuning of :func:`snap_to_valley`.

    Attributes:
        radius_m: Largest lateral move in metres.
        offsets: Number of candidate offsets (odd; spread over ``±radius_m``).
        prior_m: Height penalty (metres) of a move of ``radius_m`` from the source
            position: the trust in the source geometry.
        lateral_cost: Height penalty (metres) per metre of lateral change between
            two consecutive vertices (smoothness of the path).
        max_turn: Largest lateral change per metre along the line.
        smooth_vertices: Gaussian sigma (in vertices) applied to the offsets after
            the search.
        normal_window_m: Length of the window smoothing the tangent (metres).
    """

    radius_m: float = 90.0
    offsets: int = 31
    prior_m: float = 2.0
    lateral_cost: float = 0.08
    max_turn: float = 0.7
    smooth_vertices: float = 1.5
    normal_window_m: float = 120.0


def polyline_length(points: np.ndarray) -> float:
    """Length of a polyline ``(n, 2)``."""
    if len(points) < 2:
        return 0.0
    return float(np.hypot(*np.diff(points, axis=0).T).sum())


def chainage(points: np.ndarray) -> np.ndarray:
    """Cumulated length at each vertex (0 at the first one)."""
    if len(points) < 2:
        return np.zeros(len(points))
    return np.concatenate([[0.0], np.cumsum(np.hypot(*np.diff(points, axis=0).T))])


def resample(points: np.ndarray, step: float) -> np.ndarray:
    """Evenly spaced vertices along ``points`` (both ends kept).

    Args:
        points: ``(n, 2)`` polyline.
        step: Target spacing (the last interval is spread over the others).

    Returns:
        ``(m, 2)`` polyline with ``m >= 2`` when the input has length.
    """
    points = np.asarray(points, dtype=np.float64)
    if len(points) < 2:
        return points.copy()
    dist = chainage(points)
    total = dist[-1]
    if total <= 0.0:
        return points[[0, -1]].copy()
    count = max(int(np.ceil(total / step)), 1)
    targets = np.linspace(0.0, total, count + 1)
    # Remove duplicate chainages (zero-length segments) before interpolating.
    keep = np.concatenate([[True], np.diff(dist) > 0.0])
    dist, pts = dist[keep], points[keep]
    return np.column_stack(
        [np.interp(targets, dist, pts[:, 0]), np.interp(targets, dist, pts[:, 1])]
    )


def normals(points: np.ndarray, window: int) -> np.ndarray:
    """Unit left normals of a polyline, from a tangent smoothed over ``window`` vertices."""
    if len(points) < 2:
        return np.tile([0.0, 1.0], (len(points), 1))
    tangent = np.gradient(points, axis=0)
    if window > 1:
        tangent = ndimage.uniform_filter1d(tangent, size=window, axis=0, mode="nearest")
    norm = np.hypot(tangent[:, 0], tangent[:, 1])
    norm[norm == 0.0] = 1.0
    tangent /= norm[:, None]
    return np.column_stack([-tangent[:, 1], tangent[:, 0]])


def curvature_radius(points: np.ndarray, window: int) -> np.ndarray:
    """Radius of curvature at each vertex of an evenly spaced polyline (inf if straight)."""
    if len(points) < 3:
        return np.full(len(points), np.inf)
    smooth = ndimage.uniform_filter1d(
        points, size=max(window, 1), axis=0, mode="nearest"
    )
    d1 = np.gradient(smooth, axis=0)
    d2 = np.gradient(d1, axis=0)
    cross = np.abs(d1[:, 0] * d2[:, 1] - d1[:, 1] * d2[:, 0])
    speed = np.hypot(d1[:, 0], d1[:, 1]) ** 3
    with np.errstate(divide="ignore", invalid="ignore"):
        radius = np.where(cross > 1e-12, speed / cross, np.inf)
    return radius


def viterbi_offsets(
    cost: np.ndarray, offsets: np.ndarray, lateral_cost: float, max_jump: float
) -> np.ndarray:
    """Offset index per vertex minimising ``cost`` plus lateral changes.

    Args:
        cost: ``(n, m)`` node costs (``inf`` for forbidden offsets).
        offsets: ``(m,)`` offsets in metres.
        lateral_cost: Cost per metre of offset change between consecutive vertices.
        max_jump: Largest allowed offset change between consecutive vertices.

    Returns:
        ``(n,)`` indices into ``offsets``.
    """
    n, m = cost.shape
    jump = np.abs(offsets[:, None] - offsets[None, :])
    transition = lateral_cost * jump
    transition[jump > max_jump + 1e-9] = np.inf
    total = cost[0].copy()
    back = np.zeros((n, m), dtype=np.int32)
    for i in range(1, n):
        candidates = total[None, :] + transition  # (to, from)
        best = np.argmin(candidates, axis=1)
        total = candidates[np.arange(m), best] + cost[i]
        back[i] = best
        if not np.isfinite(total).any():  # every path forbidden: restart freely
            total = cost[i].copy()
    path = np.empty(n, dtype=np.int32)
    path[-1] = int(np.argmin(total))
    for i in range(n - 1, 0, -1):
        path[i - 1] = back[i, path[i]]
    return path


def snap_to_valley(
    points: np.ndarray,
    sample: Sampler,
    params: SnapParams,
    fixed_ends: bool = False,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Move an evenly spaced polyline onto the valley floor.

    Args:
        points: ``(n, 2)`` evenly spaced vertices (see :func:`resample`).
        sample: Height sampler in metres.
        params: Search parameters.
        fixed_ends: Keep the first and last vertices in place.

    Returns:
        ``(snapped points, floor heights, offsets in metres)``.
    """
    points = np.asarray(points, dtype=np.float64)
    n = len(points)
    if n < 2:
        heights = sample(points[:, 0], points[:, 1]) if n else np.zeros(0)
        return points.copy(), np.asarray(heights, dtype=np.float64), np.zeros(n)
    step = polyline_length(points) / (n - 1)
    window = max(int(round(params.normal_window_m / max(step, 1e-6))), 1)
    normal = normals(points, window)
    offsets = np.linspace(-params.radius_m, params.radius_m, params.offsets)
    # Candidate positions (n, m): sample the relief once for all of them.
    cand_x = points[:, 0, None] + normal[:, 0, None] * offsets[None, :]
    cand_y = points[:, 1, None] + normal[:, 1, None] * offsets[None, :]
    heights = np.asarray(sample(cand_x.ravel(), cand_y.ravel()), dtype=np.float64)
    heights = heights.reshape(n, params.offsets)
    cost = heights + params.prior_m * np.abs(offsets)[None, :] / max(
        params.radius_m, 1e-6
    )
    # Inside a tight bend, a long normal would cross the other arm of the meander.
    radius = curvature_radius(points, window)
    limit = np.maximum(0.8 * radius, 2.0 * step)
    cost[np.abs(offsets)[None, :] > limit[:, None]] = np.inf
    cost[~np.isfinite(heights)] = np.inf
    centre = params.offsets // 2
    cost[:, centre] = np.where(np.isfinite(cost[:, centre]), cost[:, centre], 1e9)
    if fixed_ends:
        for index in (0, n - 1):
            cost[index, :] = np.inf
            cost[index, centre] = 0.0
    path = viterbi_offsets(cost, offsets, params.lateral_cost, params.max_turn * step)
    chosen = offsets[path]
    if params.smooth_vertices > 0 and n > 2:
        chosen = ndimage.gaussian_filter1d(
            chosen, params.smooth_vertices, mode="nearest"
        )
        if fixed_ends:
            chosen[0] = chosen[-1] = 0.0
    snapped = points + normal * chosen[:, None]
    floor = np.asarray(sample(snapped[:, 0], snapped[:, 1]), dtype=np.float64)
    return snapped, floor, chosen


def isotonic_decreasing(values: np.ndarray) -> np.ndarray:
    """Least-squares fit of ``values`` by a non-increasing sequence (PAVA).

    NaN values are first filled by linear interpolation of their neighbours.
    """
    y = np.asarray(values, dtype=np.float64).copy()
    n = len(y)
    finite = np.isfinite(y)
    if n == 0 or not finite.any():
        return y
    if not finite.all():
        index = np.arange(n)
        y[~finite] = np.interp(index[~finite], index[finite], y[finite])
    # Pool adjacent violators: blocks of (mean, count), merged while they rise.
    means: list[float] = []
    counts: list[int] = []
    for value in y:
        means.append(float(value))
        counts.append(1)
        while len(means) > 1 and means[-2] < means[-1]:
            total = counts[-2] + counts[-1]
            mean = (means[-2] * counts[-2] + means[-1] * counts[-1]) / total
            means[-2:] = [mean]
            counts[-2:] = [total]
    return np.repeat(np.asarray(means), np.asarray(counts))


def enforce_cap(values: np.ndarray, cap: float) -> np.ndarray:
    """Running minimum from the upstream end, starting at ``cap``."""
    return np.minimum.accumulate(np.concatenate([[cap], values]))[1:]


def drape(
    heights: np.ndarray,
    step: float,
    smooth_m: float = 60.0,
    max_cut_m: float = 3.0,
) -> np.ndarray:
    """Road altitudes: relief heights smoothed along the road.

    A Gaussian of ``smooth_m`` eases the slopes; the result stays within
    ``max_cut_m`` of the relief (a road does not float above a hollow nor dig a
    trench through a hump by more than an embankment or a cutting).
    """
    heights = np.asarray(heights, dtype=np.float64)
    if len(heights) < 3:
        return heights.copy()
    sigma = smooth_m / max(step, 1e-6)
    smooth = ndimage.gaussian_filter1d(heights, sigma, mode="nearest")
    return np.clip(smooth, heights - max_cut_m, heights + max_cut_m)


def simplify_indices(points: np.ndarray, tolerance: float) -> np.ndarray:
    """Indices kept by Douglas-Peucker on ``(n, 2)`` points (iterative)."""
    n = len(points)
    if n <= 2:
        return np.arange(n)
    keep = np.zeros(n, dtype=bool)
    keep[0] = keep[-1] = True
    stack = [(0, n - 1)]
    while stack:
        start, end = stack.pop()
        if end <= start + 1:
            continue
        a, b = points[start], points[end]
        seg = b - a
        length = float(np.hypot(*seg))
        mid = points[start + 1 : end]
        if length == 0.0:
            dist = np.hypot(*(mid - a).T)
        else:
            dist = (
                np.abs(seg[0] * (mid[:, 1] - a[1]) - seg[1] * (mid[:, 0] - a[0]))
                / length
            )
        index = int(np.argmax(dist))
        if dist[index] > tolerance:
            split = start + 1 + index
            keep[split] = True
            stack.append((start, split))
            stack.append((split, end))
    return np.flatnonzero(keep)


def lateral_route(
    points: np.ndarray,
    sample: Sampler,
    node_cost: Callable[[np.ndarray, np.ndarray, np.ndarray], np.ndarray],
    radius_m: float,
    offsets: int = 13,
    prior_m: float = 1.0,
    lateral_cost: float = 0.05,
    grade_cost: float = 1.0,
    max_turn: float = 0.5,
    normal_window_m: float = 150.0,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Shift a road sideways (Viterbi) to ease its grades and avoid bad ground.

    Args:
        points: Evenly spaced vertices (:func:`resample`).
        sample: Height sampler.
        node_cost: ``f(x, y, heights) -> cost`` over the ``(n, m)`` candidates
            (wet ground, river beds...), in metres of climb equivalent.
        radius_m: Largest lateral move.
        offsets: Candidate offsets.
        prior_m: Cost of a move of ``radius_m`` (trust in the traced road).
        lateral_cost: Cost per metre of lateral change between vertices.
        grade_cost: Cost per metre of height change between vertices.
        max_turn: Largest lateral change per metre along the road.
        normal_window_m: Tangent smoothing window.

    Returns:
        ``(points, heights, offsets)`` of the chosen route.
    """
    points = np.asarray(points, dtype=np.float64)
    n = len(points)
    if n < 2 or radius_m <= 0.0:
        heights = np.asarray(sample(points[:, 0], points[:, 1]), dtype=np.float64)
        return points.copy(), heights, np.zeros(n)
    step = polyline_length(points) / (n - 1)
    window = max(int(round(normal_window_m / max(step, 1e-6))), 1)
    normal = normals(points, window)
    offs = np.linspace(-radius_m, radius_m, offsets)
    cand_x = points[:, 0, None] + normal[:, 0, None] * offs[None, :]
    cand_y = points[:, 1, None] + normal[:, 1, None] * offs[None, :]
    heights = np.asarray(sample(cand_x.ravel(), cand_y.ravel()), dtype=np.float64)
    heights = heights.reshape(n, offsets)
    heights = np.where(np.isfinite(heights), heights, np.nanmean(heights))
    cost = (
        node_cost(cand_x, cand_y, heights) + prior_m * np.abs(offs)[None, :] / radius_m
    )
    radius = curvature_radius(points, window)
    limit = np.maximum(0.8 * radius, 2.0 * step)
    centre = offsets // 2
    too_far = np.abs(offs)[None, :] > limit[:, None]
    too_far[:, centre] = False
    cost[too_far] = np.inf
    jump = np.abs(offs[:, None] - offs[None, :])
    base = lateral_cost * jump
    base[jump > max_turn * step + 1e-9] = np.inf
    total = cost[0].copy()
    back = np.zeros((n, offsets), dtype=np.int32)
    for i in range(1, n):
        climb = grade_cost * np.abs(heights[i][:, None] - heights[i - 1][None, :])
        candidates = total[None, :] + base + climb
        best = np.argmin(candidates, axis=1)
        total = candidates[np.arange(offsets), best] + cost[i]
        back[i] = best
    path = np.empty(n, dtype=np.int32)
    path[-1] = int(np.argmin(total))
    for i in range(n - 1, 0, -1):
        path[i - 1] = back[i, path[i]]
    chosen = offs[path]
    routed = points + normal * chosen[:, None]
    return routed, heights[np.arange(n), path], chosen


def simplify_indices_3d(
    points: np.ndarray, heights: np.ndarray, tolerance: float, z_scale: float
) -> np.ndarray:
    """Douglas-Peucker on ``(x, y, z * z_scale)`` (keeps the relief of a draped line)."""
    stacked = np.column_stack([points, np.asarray(heights) * z_scale])
    n = len(stacked)
    if n <= 2:
        return np.arange(n)
    keep = np.zeros(n, dtype=bool)
    keep[0] = keep[-1] = True
    stack = [(0, n - 1)]
    while stack:
        start, end = stack.pop()
        if end <= start + 1:
            continue
        a, b = stacked[start], stacked[end]
        seg = b - a
        mid = stacked[start + 1 : end]
        length2 = float(seg @ seg)
        if length2 == 0.0:
            dist = np.linalg.norm(mid - a, axis=1)
        else:
            t = np.clip((mid - a) @ seg / length2, 0.0, 1.0)
            dist = np.linalg.norm(mid - (a + t[:, None] * seg), axis=1)
        index = int(np.argmax(dist))
        if dist[index] > tolerance:
            split = start + 1 + index
            keep[split] = True
            stack.append((start, split))
            stack.append((split, end))
    return np.flatnonzero(keep)
