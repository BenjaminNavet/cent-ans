"""Road polylines of the settlement graph edges (lot C7b).

The movement graph (``settlement_graph.json``) stores straight edges between
settlements; an edge flagged ``road`` follows a road of ``roads.geojson`` on
average. For display only (army path preview in Godot), this step traces the
actual road between both ends of every ``road`` edge.

Method: the roads are densified to :data:`DENSIFY_PX` and turned into a
network whose nodes are the densified points; consecutive points are linked,
and so are points of any road closer than :data:`JOIN_PX` (crossings and small
gaps between Itiner-e segments). For each ``road`` edge, a Dijkstra search
starts from the road points within :data:`ATTACH_PX` of the first settlement
(cost of the off-road access x :data:`OFFROAD_FACTOR`) and ends on the points
near the second one. A route longer than :data:`MAX_DETOUR` x the straight
line (plus :data:`DETOUR_SLACK_PX`) is rejected: the game then draws the
straight edge.

Output: ``data/map/settlement_edge_paths.json``,
``{"edges": [{"from", "to", "points": [[x, y], ...]}]}`` in map pixels
(1 decimal), ``from < to`` as in the graph, points from ``from`` to ``to``,
first and last points on the settlement positions. Read by
``game/scripts/map/settlement_data.gd``; no game rule depends on it.
"""

from __future__ import annotations

import heapq
import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy.sparse import coo_matrix
from scipy.spatial import cKDTree
from shapely.geometry import LineString

EDGE_PATHS_FILE = "settlement_edge_paths.json"
DENSIFY_PX = 1.0
JOIN_PX = 3.0
# Joining two roads costs a little more than following one.
JOIN_FACTOR = 1.5
ATTACH_PX = 6.0
ATTACH_RATIO = 0.15
MAX_ATTACH_PX = 16.0
OFFROAD_FACTOR = 3.0
MAX_DETOUR = 1.6
DETOUR_SLACK_PX = 3.0
SIMPLIFY_PX = 0.15


@dataclass
class RoadNetwork:
    """Densified road points and their CSR adjacency."""

    points: np.ndarray
    indptr: np.ndarray
    indices: np.ndarray
    weights: np.ndarray
    tree: cKDTree


def densify(coords: np.ndarray, step: float = DENSIFY_PX) -> np.ndarray:
    """Points every ``step`` px at most along a polyline (vertices kept)."""
    out = [coords[:1]]
    for start, end in zip(coords[:-1], coords[1:], strict=True):
        count = max(1, int(np.ceil(float(np.hypot(*(end - start))) / step)))
        t = np.arange(1, count + 1, dtype=np.float64)[:, None] / count
        out.append(start * (1 - t) + end * t)
    return np.concatenate(out)


def build_network(lines: list[np.ndarray]) -> RoadNetwork:
    """Road network of densified ``lines`` (``(n, 2)`` pixel arrays)."""
    parts = [
        densify(np.asarray(line, dtype=np.float64)) for line in lines if len(line) >= 2
    ]
    if not parts:
        empty = np.zeros(0, dtype=np.int64)
        return RoadNetwork(
            np.zeros((0, 2)),
            np.zeros(1, dtype=np.int64),
            empty,
            np.zeros(0),
            cKDTree(np.zeros((1, 2))),
        )
    points = np.concatenate(parts)
    rows: list[np.ndarray] = []
    cols: list[np.ndarray] = []
    weights: list[np.ndarray] = []
    offset = 0
    for part in parts:
        a = np.arange(offset, offset + len(part) - 1)
        rows.append(a)
        cols.append(a + 1)
        weights.append(np.hypot(*(part[1:] - part[:-1]).T))
        offset += len(part)
    tree = cKDTree(points)
    pairs = tree.query_pairs(JOIN_PX, output_type="ndarray")
    if len(pairs):
        rows.append(pairs[:, 0])
        cols.append(pairs[:, 1])
        weights.append(
            JOIN_FACTOR * np.hypot(*(points[pairs[:, 0]] - points[pairs[:, 1]]).T)
        )
    row = np.concatenate(rows)
    col = np.concatenate(cols)
    weight = np.maximum(np.concatenate(weights), 1e-6)
    count = len(points)
    matrix = coo_matrix(
        (
            np.concatenate([weight, weight]),
            (np.concatenate([row, col]), np.concatenate([col, row])),
        ),
        shape=(count, count),
    ).tocsr()
    return RoadNetwork(points, matrix.indptr, matrix.indices, matrix.data, tree)


def trace(
    network: RoadNetwork, start: tuple[float, float], end: tuple[float, float]
) -> np.ndarray | None:
    """Road polyline from ``start`` to ``end`` (both included), or ``None``.

    ``None`` when no road point lies near both ends, or when the route is
    longer than :data:`MAX_DETOUR` x the straight line plus
    :data:`DETOUR_SLACK_PX`.
    """
    if len(network.points) == 0:
        return None
    a = np.asarray(start, dtype=np.float64)
    b = np.asarray(end, dtype=np.float64)
    straight = float(np.hypot(*(b - a)))
    radius = min(MAX_ATTACH_PX, max(ATTACH_PX, ATTACH_RATIO * straight))
    sources = network.tree.query_ball_point(a, radius)
    targets = network.tree.query_ball_point(b, radius)
    if not sources or not targets:
        return None
    exit_cost = {
        int(n): OFFROAD_FACTOR * float(np.hypot(*(network.points[n] - b)))
        for n in targets
    }
    limit = MAX_DETOUR * straight + DETOUR_SLACK_PX + OFFROAD_FACTOR * 2.0 * radius
    best = np.inf
    best_node = -1
    distance: dict[int, float] = {}
    parent: dict[int, int] = {}
    heap: list[tuple[float, int, int]] = []
    for n in sources:
        cost = OFFROAD_FACTOR * float(np.hypot(*(network.points[n] - a)))
        heapq.heappush(heap, (cost, int(n), -1))
    while heap:
        cost, node, previous = heapq.heappop(heap)
        if node in distance:
            continue
        if cost >= best or cost > limit:
            break
        distance[node] = cost
        parent[node] = previous
        if node in exit_cost and cost + exit_cost[node] < best:
            best = cost + exit_cost[node]
            best_node = node
        for k in range(network.indptr[node], network.indptr[node + 1]):
            neighbour = int(network.indices[k])
            if neighbour not in distance:
                heapq.heappush(
                    heap, (cost + float(network.weights[k]), neighbour, node)
                )
    if best_node < 0:
        return None
    chain = []
    node = best_node
    while node >= 0:
        chain.append(node)
        node = parent[node]
    route = np.vstack([a, network.points[chain[::-1]], b])
    length = float(np.hypot(*np.diff(route, axis=0).T).sum())
    if length > MAX_DETOUR * straight + DETOUR_SLACK_PX:
        return None
    return np.asarray(LineString(route).simplify(SIMPLIFY_PX).coords)


def load_road_lines(roads_path: Path) -> list[np.ndarray]:
    """``roads.geojson`` LineStrings as pixel arrays (empty without the file)."""
    if not roads_path.exists():
        return []
    collection = json.loads(roads_path.read_text(encoding="utf-8"))
    return [
        np.asarray(f["geometry"]["coordinates"], dtype=np.float64)
        for f in collection["features"]
        if f["geometry"]["type"] == "LineString"
    ]


def edge_paths(
    edges: list[dict],
    positions: dict[str, tuple[float, float]],
    lines: list[np.ndarray],
) -> list[dict]:
    """One entry per ``road`` edge that a road can be traced along (see module docstring)."""
    network = build_network(lines)
    result = []
    for edge in edges:
        if not edge["road"] or edge["sea"]:
            continue
        route = trace(network, positions[edge["from"]], positions[edge["to"]])
        if route is None:
            continue
        result.append(
            {
                "from": edge["from"],
                "to": edge["to"],
                "points": [[round(float(x), 1), round(float(y), 1)] for x, y in route],
            }
        )
    return result


def write(
    map_dir: Path,
    edges: list[dict],
    positions: dict[str, tuple[float, float]],
    roads_file: str,
) -> tuple[Path, int]:
    """Write :data:`EDGE_PATHS_FILE`; returns its path and the number of traced edges."""
    paths = edge_paths(edges, positions, load_road_lines(map_dir / roads_file))
    path = map_dir / EDGE_PATHS_FILE
    path.write_text(
        json.dumps({"edges": paths}, separators=(",", ":")) + "\n", encoding="utf-8"
    )
    return path, len(paths)
