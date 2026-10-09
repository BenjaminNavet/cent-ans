"""DN-TROUS: geometric hole metric for the generated campaign models.

For every model used by ``data/art/dn_campaign_models.json`` (raw TRELLIS glb of the manifest, or
``--variant`` raw glb), measures with trimesh, after merging vertices by position only:

* ``open_ratio``: length of open boundary edges / sqrt(surface area) (torn walls, missing faces);
* ``floaters``: share of the surface in connected components smaller than 3 % of the largest;
* ``sliver``: share of faces whose aspect ratio (longest edge^2 / (2 area)) exceeds 40;
* ``inverted``: share of the surface in shells with negative signed volume (turned inside out:
  back-face culling then shows torn walls and black interiors, the real cause found by DN-TROUS);
* ``score``: open_ratio + 4 * floaters + 4 * sliver + 10 * inverted (relative ranking, calibrated on the four
  towns cited by the player: Kingston, Montargis, Nuremberg, Slesvig).

Usage: ``uv run --with trimesh --with numpy --with networkx python tools/experiments/dn_holes.py
[--variant rv2] [--ids a,b] [--json out.json]``. Free, no network.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path

import numpy as np
import trimesh

REPO = Path(__file__).resolve().parents[2]
RAW = Path.home() / "dev" / "cent-ans-raw" / "dn"
CITED = ("set_kingston", "set_montargis", "set_nuremberg", "set_slesvig")
KIND_RANK = {"city": 0, "castle": 1, "town": 2, "abbey": 3, "village": 4}


def load_mesh(path: Path) -> trimesh.Trimesh:
    """Whole scene as one mesh, vertices merged by position only (UV seams are not holes)."""
    scene = trimesh.load(str(path), process=False, force="scene")
    mesh = trimesh.util.concatenate([g for g in scene.dump() if len(g.faces)])
    step = 1e-4 * float(np.linalg.norm(mesh.extents)) or 1e-6
    grid = np.round(mesh.vertices / step).astype(np.int64)
    _, first, inverse = np.unique(grid, axis=0, return_index=True, return_inverse=True)
    merged = trimesh.Trimesh(mesh.vertices[first], inverse.reshape(-1)[mesh.faces], process=False)
    merged.update_faces(merged.nondegenerate_faces())
    return merged


def measure(path: Path) -> dict:
    """Hole metrics of one glb."""
    mesh = load_mesh(path)
    area = float(mesh.area)
    scale = np.sqrt(area) or 1.0
    edges = np.sort(mesh.edges, axis=1)
    keys, counts = np.unique(edges, axis=0, return_counts=True)
    open_edges = keys[counts == 1]
    lengths = np.linalg.norm(mesh.vertices[open_edges[:, 0]] - mesh.vertices[open_edges[:, 1]], axis=1)
    open_ratio = float(lengths.sum() / scale)
    parts = trimesh.graph.connected_components(mesh.face_adjacency, nodes=np.arange(len(mesh.faces)))
    areas = np.array([mesh.area_faces[p].sum() for p in parts])
    floaters = float(areas[areas < 0.03 * areas.max()].sum() / areas.sum()) if len(areas) else 0.0
    tri = mesh.vertices[mesh.faces]
    centred = tri - mesh.bounds.mean(axis=0)  # signed volume of an open shell depends on the origin
    volumes = np.einsum("ij,ij->i", centred[:, 0], np.cross(centred[:, 1], centred[:, 2])) / 6.0
    inverted_area = 0.0
    for part, part_area in zip(parts, areas, strict=True):
        if part_area > 0.002 * area and volumes[part].sum() < 0:  # shell turned inside out
            inverted_area += part_area
    inverted = float(inverted_area / area)
    e = np.stack([np.linalg.norm(tri[:, i] - tri[:, (i + 1) % 3], axis=1) for i in range(3)], axis=1)
    aspect = e.max(axis=1) ** 2 / np.maximum(2 * mesh.area_faces, 1e-12)
    sliver = float((mesh.area_faces[aspect > 40].sum()) / area)
    return {
        "faces": int(len(mesh.faces)), "components": int(len(parts)),
        "open_ratio": round(open_ratio, 3), "floaters": round(floaters, 4),
        "sliver": round(sliver, 4), "inverted": round(inverted, 4),
        "score": round(open_ratio + 4 * floaters + 4 * sliver + 10 * inverted, 3),
    }  # fmt: skip


def usage() -> dict[str, dict]:
    """Per campaign model path: kinds, settlement count and best-visibility rank."""
    maq = json.loads((REPO / "data/art/town_maquettes.json").read_text())
    models = json.loads((REPO / "data/art/dn_campaign_models.json").read_text())
    lookup: dict[str, str] = {}
    for fam, spec in maq["families"].items():
        for field in ("cultures", "regions", "religions"):
            for value in spec.get(field, []):
                lookup.setdefault(f"{field}|{value}", fam)
    out: dict[str, dict] = {}
    prov_cache: dict[str, dict] = {}
    for file in sorted((REPO / "data/settlements").glob("*.json")):
        data = json.loads(file.read_text())
        for s in data if isinstance(data, list) else data.get("settlements", []):
            if not isinstance(s, dict) or "province" not in s:
                continue
            pid = s["province"]
            if pid not in prov_cache:
                pf = REPO / "data/provinces" / f"{pid}.json"
                prov_cache[pid] = json.loads(pf.read_text()) if pf.exists() else {}
            prov = prov_cache[pid]
            culture, region, religion = (prov.get(k, "") for k in ("culture", "region", "religion"))
            fam = maq["default_family"]
            for field, value in (("cultures", culture), ("regions", region), ("religions", religion)):
                if f"{field}|{value}" in lookup:
                    fam = lookup[f"{field}|{value}"]
                    break
            sub = ""
            for rule in models["subfamilies"]:
                if rule["family"] == fam and (
                    culture in rule.get("cultures", [])
                    or region in rule.get("regions", [])
                    or religion in rule.get("religions", [])
                ):
                    sub = rule["id"]
                    break
            kind = s.get("kind", "village")
            keys = [sub] if sub else []
            if sub:
                keys += next(r.get("also", []) for r in models["subfamilies"] if r["id"] == sub)
            keys += [fam, "*"]
            by = models["table"].get(kind, {})
            for key in keys:
                if by.get(key):
                    for entry in by[key]:
                        u = out.setdefault(entry["path"], {"kinds": Counter(), "ids": []})
                        u["kinds"][kind] += 1
                        u["ids"].append(s["id"])
                    break
    return out


def main() -> None:
    """Measure and rank every used model."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--variant", default="")
    parser.add_argument("--ids", default="")
    parser.add_argument("--json", default="")
    parser.add_argument("--lod", type=int, default=-1, help="measure the ingested lodN glb instead")
    args = parser.parse_args()
    manifest = json.loads((REPO / "data/art/dn_manifest.json").read_text())["assets"]
    used = usage()
    rows = []
    for path, info in used.items():
        asset = Path(path).name
        if args.ids and asset not in args.ids.split(","):
            continue
        base = RAW / asset
        glbs = (
            sorted((base / args.variant / "3d").glob("*.glb"))
            if args.variant
            else [Path(manifest[asset]["raw"])] if asset in manifest else []
        )
        if args.lod >= 0:
            glbs = sorted((REPO / "game/assets/models").glob(f"{path}_lod{args.lod}.glb"))
        if not glbs or not glbs[0].exists():
            continue
        row = {"id": asset, "glb": str(glbs[0]), "uses": sum(info["kinds"].values()),
               "kinds": dict(info["kinds"]),
               "rank": min(KIND_RANK.get(k, 9) for k in info["kinds"]),
               "cited": [i for i in info["ids"] if i in CITED], **measure(glbs[0])}  # fmt: skip
        rows.append(row)
        print(row["id"], row["score"], row["open_ratio"], row["floaters"], row["sliver"], flush=True)
    if args.json:
        Path(args.json).write_text(json.dumps(rows, indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
