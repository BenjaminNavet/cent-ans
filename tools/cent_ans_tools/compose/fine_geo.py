"""Fine geography in the composed frame (Bevy lot ``geofine2``).

The fine layer (``geo hydro-fine``: snapped rivers; ``geo anchors-fine``: draped roads,
refined settlement / hamlet / crossing anchors) was built on the REAL map (EPSG:3035
pixels, 7168 x 6144) before the composed map (lot CC, ADR 0355). ``cent-ans geo
compose-fine`` carries it into the composed frame with the composition's own
transform (:meth:`ComposedWarp.forward_px`, as :func:`geo.project.composed_geometry`
does for rivers and coasts: clip to the crop box, densify every 8 real pixels, forward
warp). No new projection, no re-snapping:

- CAFV tiles (rivers, roads): every polyline is decoded, warped, re-cut along the E2
  tiles of the composed frame (:func:`fine_tiles.split_by_tiles`); ``z`` (metres, the
  composed relief is the real DEM warped) and ``w`` (metres) are kept;
- settlements and hamlets: rule positions of the composed map stay authoritative (the
  editor may have moved them), the anchor keeps its real offset carried by the warp:
  ``composed_rule + fwd(anchor_real) - fwd(rule_real)``;
- crossings: on their fine river, i.e. ``fwd(anchor_real)``, direction through the
  local Jacobian; written in the order of the composed ``crossings_px.json``.

Outputs: tiles under ``<out>/pyramid/{hydro_fine,roads_fine}/E2`` (outside git) and the
versioned manifests ``data/map/rivers_fine.json`` and ``data/map/fine_anchors.json``.
Rendering data only: no game rule reads these files.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path

import numpy as np

from cent_ans_tools.compose.contract import COMPOSE_DIR, ComposedWarp
from cent_ans_tools.geo import fine_tiles
from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"
REAL_ROOT = Path.home() / "dev" / "cent-ans-raw" / "pyramid-reelle"
#: Commit whose rule files (settlements, hamlets, crossings) produced the real
#: ``fine_anchors.json`` (generated 02/10, committed in 918b04e52).
REAL_RULES_REV = "918b04e52"
#: Densification step before the warp, in real pixels (``composed_geometry``).
DENSIFY_PX = 8.0
#: Margin kept inside the crop box (``composed_geometry``).
CROP_MARGIN_PX = 2.0
LAYER_DIRS = {fine_tiles.LAYER_RIVERS: "hydro_fine", fine_tiles.LAYER_ROADS: "roads_fine"}


@dataclass
class LayerStats:
    """Counts of one warped layer."""

    lines_in: int = 0
    points_in: int = 0
    pieces_out: int = 0
    points_out: int = 0
    points_dropped: int = 0
    tiles: int = 0
    total_bytes: int = 0


@dataclass
class Result:
    """Summary of :func:`build`."""

    layers: dict[str, LayerStats] = field(default_factory=dict)
    settlements: int = 0
    settlements_missing: list[str] = field(default_factory=list)
    hamlets: int = 0
    hamlets_matched: int = 0
    crossings: int = 0
    crossings_missing: list[str] = field(default_factory=list)
    anchor_offset_px: dict[str, float] = field(default_factory=dict)


# ------------------------------------------------------------------ geometry


def densify(xy: np.ndarray, attrs: np.ndarray, step: float = DENSIFY_PX):
    """Insert vertices so that no segment is longer than ``step`` (attributes interpolated)."""
    xy = np.asarray(xy, dtype=np.float64)
    attrs = np.asarray(attrs, dtype=np.float64).reshape(len(xy), -1)
    if len(xy) < 2:
        return xy, attrs
    seg = np.hypot(*(xy[1:] - xy[:-1]).T)
    counts = np.maximum(1, np.ceil(seg / step).astype(np.int64))
    if np.all(counts == 1):
        return xy, attrs
    out_xy, out_attr = [], []
    for i, n in enumerate(counts):
        t = np.arange(n)[:, None] / n
        out_xy.append(xy[i] + (xy[i + 1] - xy[i]) * t)
        out_attr.append(attrs[i] + (attrs[i + 1] - attrs[i]) * t)
    out_xy.append(xy[-1:])
    out_attr.append(attrs[-1:])
    return np.concatenate(out_xy), np.concatenate(out_attr)


def warp_polyline(
    warp: ComposedWarp, xy: np.ndarray, attrs: np.ndarray
) -> list[tuple[np.ndarray, np.ndarray]]:
    """Real-pixel polyline -> composed pieces (split where it leaves the crop box).

    Returns:
        ``(xy, attrs)`` runs of at least two vertices.
    """
    xy, attrs = densify(xy, attrs)
    x0, y0, x1, y1 = warp.crop_box_px
    m = CROP_MARGIN_PX
    inside = (
        (xy[:, 0] >= x0 + m)
        & (xy[:, 0] <= x1 - m)
        & (xy[:, 1] >= y0 + m)
        & (xy[:, 1] <= y1 - m)
    )
    cx, cy = warp.forward_px(xy[:, 0], xy[:, 1])
    out = np.column_stack([cx, cy])
    ok = inside & np.isfinite(out).all(axis=1)
    runs = []
    start = None
    for i in range(len(ok) + 1):
        if i < len(ok) and ok[i]:
            if start is None:
                start = i
        elif start is not None:
            if i - start >= 2:
                runs.append((out[start:i], attrs[start:i]))
            start = None
    return runs


def jacobian(warp: ComposedWarp, x: float, y: float, eps: float = 2.0) -> np.ndarray:
    """2x2 Jacobian of the forward warp at real pixel ``(x, y)`` (central differences)."""
    fx = warp.forward_px(np.array([x + eps, x - eps, x, x]), np.array([y, y, y + eps, y - eps]))
    px, py = np.asarray(fx[0]), np.asarray(fx[1])
    return np.array(
        [
            [(px[0] - px[1]) / (2 * eps), (px[2] - px[3]) / (2 * eps)],
            [(py[0] - py[1]) / (2 * eps), (py[2] - py[3]) / (2 * eps)],
        ]
    )


def forward_point(warp: ComposedWarp, x: float, y: float) -> np.ndarray:
    """One real pixel -> composed pixel (NaN outside)."""
    cx, cy = warp.forward_px(np.array([x]), np.array([y]))
    return np.array([float(cx[0]), float(cy[0])])


# ------------------------------------------------------------------ tiles


def warp_layer(
    warp: ComposedWarp, src_dir: Path, layer: int
) -> tuple[dict[tuple[int, int], fine_tiles.TileLines], LayerStats]:
    """Warp every tile of ``src_dir`` (``E2/*.bin``) into composed E2 tiles."""
    side = fine_tiles.tile_units(fine_tiles.TILE_LEVEL)
    tiles: dict[tuple[int, int], fine_tiles.TileLines] = {}
    stats = LayerStats()
    for path in sorted(src_dir.glob("*.bin")):
        decoded = fine_tiles.decode(path.read_bytes())
        if decoded["layer"] != layer:
            raise ValueError(f"{path}: layer {decoded['layer']} instead of {layer}")
        for line in decoded["lines"]:
            stats.lines_in += 1
            stats.points_in += len(line["xy"])
            attrs = np.column_stack([line["z"], line["w"]])
            kept = 0
            for xy, att in warp_polyline(warp, line["xy"], attrs):
                kept += len(xy)
                for col, row, pxy, patt in fine_tiles.split_by_tiles(xy, att, side):
                    tiles.setdefault((col, row), fine_tiles.TileLines()).add(
                        line["feature"], line["flags"], pxy, patt[:, 0], patt[:, 1]
                    )
                    stats.pieces_out += 1
                    stats.points_out += len(pxy)
            if kept == 0:
                stats.points_dropped += len(line["xy"])
    stats.tiles = len(tiles)
    return tiles, stats


# ------------------------------------------------------------------ anchors


def _git_json(rev: str, path: str) -> object:
    """``git show rev:path`` parsed as JSON (rule files of the real map)."""
    out = subprocess.run(
        ["git", "show", f"{rev}:{path}"],
        cwd=DATA_DIR.parent,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(out)


def carry_offset(
    warp: ComposedWarp, rule_composed, rule_real, anchor_real
) -> np.ndarray:
    """Composed anchor: composed rule position plus the warped real offset."""
    a = forward_point(warp, *anchor_real)
    r = forward_point(warp, *rule_real)
    delta = a - r
    if not np.isfinite(delta).all():
        delta = np.zeros(2)
    return np.asarray(rule_composed, dtype=np.float64) + delta


def height_sampler(map_dir: Path):
    """Composed terrain height (m) at composed pixels, bilinear on ``heightmap.png``."""
    from PIL import Image

    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    lo, hi = float(meta["height_min_m"]), float(meta["height_max_m"])
    img = np.asarray(Image.open(map_dir / "heightmap.png"), dtype=np.float64)
    scale = img.shape[1] / float(meta["size_px"][0])
    h, w = img.shape

    def sample(x: float, y: float) -> float:
        u = np.clip(x * scale - 0.5, 0, w - 1.001)
        v = np.clip(y * scale - 0.5, 0, h - 1.001)
        i, j = int(u), int(v)
        tu, tv = u - i, v - j
        val = (img[j, i] * (1 - tu) + img[j, i + 1] * tu) * (1 - tv) + (
            img[j + 1, i] * (1 - tu) + img[j + 1, i + 1] * tu
        ) * tv
        return lo + val / 65535.0 * (hi - lo)

    return sample


def warp_anchors(
    warp: ComposedWarp,
    real_anchors: dict,
    real_rules: dict,
    composed: dict,
    height=None,  # noqa: ANN001
) -> tuple[dict, Result]:
    """Settlements, hamlets and crossings of ``real_anchors`` in the composed frame.

    Args:
        warp: Composition transform.
        real_anchors: ``fine_anchors.json`` of the real map.
        real_rules: ``settlements`` (id -> px) and ``hamlets`` (list) of the real
            map when the anchors were made.
        composed: Same keys for the composed map (``settlements_px.json``,
            ``hamlets.json``, ``crossings_px.json`` crossings list).
        height: Composed terrain height sampler for hamlets without a real anchor.
    """
    result = Result()
    out: dict = {}

    settlements = {}
    for sid, rule_c in composed["settlements"].items():
        anchor = real_anchors["settlements"].get(sid)
        rule_r = real_rules["settlements"].get(sid)
        if anchor is None or rule_r is None:
            result.settlements_missing.append(sid)
            continue
        px = carry_offset(warp, rule_c, rule_r, anchor["px"])
        entry = {"px": [round(float(px[0]), 4), round(float(px[1]), 4)], "z": anchor["z"]}
        for key in ("moved_m", "reason"):
            if key in anchor:
                entry[key] = anchor[key]
        settlements[sid] = entry
    result.settlements = len(settlements)
    out["settlements"] = settlements

    items_r = real_anchors["hamlets"]["items"]
    by_key: dict[tuple[str, str], list[int]] = {}
    for i, ham in enumerate(real_rules["hamlets"][: len(items_r)]):
        by_key.setdefault((ham["name"], ham.get("province", "")), []).append(i)
    items = []
    for ham in composed["hamlets"]:
        candidates = by_key.get((ham["name"], ham.get("province", "")))
        rule_c = np.asarray(ham["px"], dtype=np.float64)
        if candidates:
            i = candidates.pop(0)
            x, y, z, moved = items_r[i]
            px = carry_offset(warp, rule_c, real_rules["hamlets"][i]["px"], (x, y))
            result.hamlets_matched += 1
        else:
            px = rule_c
            z = round(height(*rule_c), 2) if height else 0.0
            moved = 0.0
        items.append([round(float(px[0]), 4), round(float(px[1]), 4), z, moved])
    result.hamlets = len(items)
    out["hamlets"] = {**real_anchors["hamlets"], "items": items}

    real_crossings = {c["id"]: c for c in real_anchors["crossings"]}
    crossings = []
    for rule in composed["crossings"]:
        anchor = real_crossings.get(rule["id"])
        if anchor is None:
            result.crossings_missing.append(rule["id"])
            crossings.append({"id": rule["id"], "px": rule["px"], "snapped": False})
            continue
        entry = dict(anchor)
        if anchor.get("snapped"):
            px = forward_point(warp, *anchor["px"])
        else:
            px = np.full(2, np.nan)
        if not np.isfinite(px).all():
            px = np.asarray(rule["px"], dtype=np.float64)
            entry["snapped"] = False
        entry["px"] = [round(float(px[0]), 4), round(float(px[1]), 4)]
        if "dir" in anchor:
            d = jacobian(warp, *anchor["px"]) @ np.asarray(anchor["dir"], dtype=np.float64)
            n = float(np.hypot(*d))
            if np.isfinite(n) and n > 1e-9:
                entry["dir"] = [round(float(d[0] / n), 4), round(float(d[1] / n), 4)]
        crossings.append(entry)
    result.crossings = len(crossings)
    out["crossings"] = crossings

    offsets = [
        float(np.hypot(*(np.asarray(settlements[s]["px"]) - composed["settlements"][s])))
        for s in settlements
    ]
    if offsets:
        result.anchor_offset_px = {
            "median": round(float(np.median(offsets)), 3),
            "p95": round(float(np.percentile(offsets, 95)), 3),
            "max": round(float(np.max(offsets)), 3),
        }
    return out, result


# ------------------------------------------------------------------ build


def _manifest_tiles(manifest: dict, index: list[dict], stats: LayerStats) -> dict:
    out = dict(manifest)
    out["tiles"] = index
    old = dict(manifest.get("stats", {}))
    old.update(
        {
            "tiles": len(index),
            "points": stats.points_out,
            "total_bytes": sum(t["bytes"] for t in index),
        }
    )
    out["stats"] = old
    return out


def build(
    real_root: Path = REAL_ROOT,
    out_root: Path | None = None,
    map_dir: Path = MAP_DIR,
    real_manifest_rev: str = REAL_RULES_REV,
) -> Result:
    """Warp the real fine layer into the composed frame (tiles + manifests).

    Args:
        real_root: Folder holding the real ``hydro_fine/`` and ``roads_fine/``.
        out_root: Folder receiving ``pyramid/hydro_fine`` and ``pyramid/roads_fine``.
        map_dir: ``data/map`` (manifests are rewritten there).
        real_manifest_rev: Commit of the real manifests and rule files.
    """
    out_root = out_root or (COMPOSE_DIR / "derived" / "fine_geo")
    warp = ComposedWarp.load(map_dir / "compose")
    stamp = datetime.now(UTC).replace(microsecond=0).isoformat()
    result = Result()

    real_rivers = _git_json(real_manifest_rev, "data/map/rivers_fine.json")
    real_anchors = _git_json(real_manifest_rev, "data/map/fine_anchors.json")

    index_by_layer = {}
    for layer, sub in LAYER_DIRS.items():
        tiles, stats = warp_layer(warp, real_root / sub / "E2", layer)
        dest = out_root / "pyramid" / sub / "E2"
        index = fine_tiles.write_tiles(dest, layer, fine_tiles.TILE_LEVEL, tiles)
        stats.total_bytes = sum(t["bytes"] for t in index)
        result.layers[sub] = stats
        index_by_layer[layer] = index
    features = real_root / "hydro_fine" / "features.json"
    (out_root / "pyramid" / "hydro_fine" / "features.json").write_bytes(
        features.read_bytes()
    )

    note = (
        " Carte composée (lot Bevy geofine2) : réseau réel transporté par la déformation de "
        "la composition (`cent-ans geo compose-fine`), pas de nouveau recalage."
    )
    rivers = _manifest_tiles(
        real_rivers,
        index_by_layer[fine_tiles.LAYER_RIVERS],
        result.layers["hydro_fine"],
    )
    rivers["description"] = real_rivers["description"].split(" Carte composée")[0] + note
    rivers["coordinates"] = "pixels de la carte composée (7080 x 6104), origine nord-ouest"
    rivers["generated_at"] = stamp
    (map_dir / "rivers_fine.json").write_text(
        json.dumps(rivers, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )

    real_rules = {
        "settlements": _git_json(real_manifest_rev, "data/map/settlements_px.json"),
        "hamlets": _git_json(real_manifest_rev, "data/map/hamlets.json"),
    }
    composed = {
        "settlements": json.loads(
            (map_dir / "settlements_px.json").read_text(encoding="utf-8")
        ),
        "hamlets": json.loads((map_dir / "hamlets.json").read_text(encoding="utf-8")),
        "crossings": json.loads(
            (map_dir / "crossings_px.json").read_text(encoding="utf-8")
        )["crossings"],
    }
    anchors, anchor_result = warp_anchors(
        warp, real_anchors, real_rules, composed, height_sampler(map_dir)
    )
    for key in (
        "settlements",
        "settlements_missing",
        "hamlets",
        "hamlets_matched",
        "crossings",
        "crossings_missing",
        "anchor_offset_px",
    ):
        setattr(result, key, getattr(anchor_result, key))
    payload = dict(real_anchors)
    payload["description"] = real_anchors["description"].split(" Carte composée")[0] + note
    payload["generated_at"] = stamp
    payload.update(anchors)
    payload["roads"] = _manifest_tiles(
        real_anchors["roads"],
        index_by_layer[fine_tiles.LAYER_ROADS],
        result.layers["roads_fine"],
    )
    payload["roads"]["feature"] = (
        "index de l'entité dans roads.geojson de la carte réelle (918b04e52)"
    )
    (map_dir / "fine_anchors.json").write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    return result
