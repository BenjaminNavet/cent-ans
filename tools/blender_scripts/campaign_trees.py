"""Procedural campaign-map trees (lot V4, A1-10), exported as one glTF binary.

Run headless:  blender --background --python campaign_trees.py -- <out.glb> [seed]

Essences (height 1.0, base at the origin, Y up in Godot / Z up here):

* ``oak``: short thick trunk with two limbs, broad irregular crown of many lumps
  (pedunculate oak of lowland forests, hedgerows and field trees);
* ``beech``: straighter slender grey trunk, taller rounder crown with fewer larger lumps;
* ``fir``: mountain conifer (silver fir / spruce), narrow spire of drooping jagged tiers.

Each essence has a detailed variant (crown + trunk, a few hundred triangles) and a ``_low``
variant (about twenty triangles, no trunk) for distant tiles. Objects are named
``<essence>[_low]_crown`` and ``<essence>_trunk``: Godot (``VegetationMeshes``) merges them,
paints vertex colours and inflates the crown normals, so the palette stays in the game code.
Deterministic for a given seed. Prints ``TREE <name> <triangles>`` per object and ``OK``.
"""

import math
import random
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector, noise


def reset_scene() -> None:
    """Empty the scene."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def new_object(name: str, bm: bmesh.types.BMesh) -> bpy.types.Object:
    """Link a bmesh as a new object."""
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def add_lump(bm: bmesh.types.BMesh, center: Vector, radii: Vector, subdiv: int, rng: random.Random, rough: float) -> None:
    """Noisy ellipsoid (icosphere) for a leaf mass; ``subdiv`` 1 = icosahedron (20 faces), 2 = 80."""
    result = bmesh.ops.create_icosphere(bm, subdivisions=subdiv, radius=1.0)
    offset = Vector((rng.uniform(0, 100), rng.uniform(0, 100), rng.uniform(0, 100)))
    for v in result["verts"]:
        d = v.co.normalized()
        n = noise.noise(d * 2.3 + offset)
        scale = 1.0 + rough * n
        v.co = Vector((d.x * radii.x, d.y * radii.y, d.z * radii.z)) * scale + center


def add_trunk(bm: bmesh.types.BMesh, base: Vector, top: Vector, r0: float, r1: float, sides: int = 6) -> None:
    """Tapered open cylinder from ``base`` to ``top``."""
    axis = (top - base).normalized()
    ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 1, 0))
    u = axis.cross(ref).normalized()
    w = axis.cross(u)
    rings = []
    for center, r in ((base, r0), (top, r1)):
        ring = []
        for i in range(sides):
            a = i / sides * math.tau
            ring.append(bm.verts.new(center + (u * math.cos(a) + w * math.sin(a)) * r))
        rings.append(ring)
    for i in range(sides):
        j = (i + 1) % sides
        bm.faces.new((rings[0][i], rings[0][j], rings[1][j], rings[1][i]))


def add_tier(bm: bmesh.types.BMesh, z0: float, z1: float, radius: float, points: int, rng: random.Random, droop: float) -> None:
    """One conifer tier: jagged cone (star outline) with drooping tips."""
    apex = bm.verts.new((0.0, 0.0, z1))
    center = bm.verts.new((0.0, 0.0, z0 + (z1 - z0) * 0.25))
    ring = []
    for i in range(points * 2):
        a = i / (points * 2) * math.tau + rng.uniform(-0.08, 0.08)
        r = radius * (1.0 if i % 2 == 0 else 0.62) * rng.uniform(0.9, 1.08)
        z = z0 - (droop if i % 2 == 0 else 0.0)
        ring.append(bm.verts.new((math.cos(a) * r, math.sin(a) * r, z)))
    n = len(ring)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((ring[i], ring[j], apex))
        bm.faces.new((ring[j], ring[i], center))


def oak(rng: random.Random, low: bool) -> dict:
    """Pedunculate oak: broad irregular crown."""
    crown = bmesh.new()
    if low:
        add_lump(crown, Vector((0, 0, 0.62)), Vector((0.52, 0.5, 0.36)), 1, rng, 0.22)  # 20 faces
        return {"crown": crown}
    lumps = [
        (Vector((0.0, 0.0, 0.70)), Vector((0.36, 0.36, 0.27))),
        (Vector((0.28, 0.05, 0.58)), Vector((0.27, 0.25, 0.21))),
        (Vector((-0.24, 0.16, 0.60)), Vector((0.28, 0.26, 0.21))),
        (Vector((0.02, -0.28, 0.56)), Vector((0.25, 0.25, 0.2))),
    ]
    for k, (center, radii) in enumerate(lumps):
        jitter = Vector((rng.uniform(-0.04, 0.04), rng.uniform(-0.04, 0.04), rng.uniform(-0.03, 0.03)))
        add_lump(crown, center + jitter, radii * rng.uniform(0.9, 1.1), 2 if k == 0 else 1, rng, 0.25)
    trunk = bmesh.new()
    add_trunk(trunk, Vector((0, 0, -0.05)), Vector((0, 0, 0.42)), 0.065, 0.045, 5)
    add_trunk(trunk, Vector((0, 0, 0.38)), Vector((0.2, 0.05, 0.6)), 0.035, 0.02, 4)
    return {"crown": crown, "trunk": trunk}


def beech(rng: random.Random, low: bool) -> dict:
    """Beech: tall rounded crown, fewer larger lumps, slender straight trunk."""
    crown = bmesh.new()
    if low:
        add_lump(crown, Vector((0, 0, 0.64)), Vector((0.40, 0.40, 0.40)), 1, rng, 0.15)
        return {"crown": crown}
    lumps = [
        (Vector((0.0, 0.0, 0.68)), Vector((0.36, 0.36, 0.33))),
        (Vector((0.04, 0.02, 0.88)), Vector((0.25, 0.25, 0.15))),
        (Vector((0.18, -0.1, 0.52)), Vector((0.23, 0.23, 0.2))),
        (Vector((-0.19, 0.12, 0.52)), Vector((0.23, 0.23, 0.2))),
    ]
    for k, (center, radii) in enumerate(lumps):
        add_lump(crown, center, radii * rng.uniform(0.92, 1.08), 2 if k == 0 else 1, rng, 0.14)
    trunk = bmesh.new()
    add_trunk(trunk, Vector((0, 0, -0.05)), Vector((0, 0, 0.55)), 0.045, 0.028, 5)
    return {"crown": crown, "trunk": trunk}


def fir(rng: random.Random, low: bool) -> dict:
    """Mountain conifer: narrow spire of drooping tiers."""
    crown = bmesh.new()
    if low:
        add_tier(crown, 0.08, 1.0, 0.26, 3, rng, 0.0)
        return {"crown": crown}
    tiers = 5
    for i in range(tiers):
        t = i / tiers
        z0 = 0.1 + t * 0.74
        z1 = z0 + 0.3 - t * 0.08
        radius = 0.3 * (1.0 - t * 0.82) + 0.04
        add_tier(crown, z0, min(z1, 1.0), radius, 6, rng, 0.05 * (1.0 - t))
    trunk = bmesh.new()
    add_trunk(trunk, Vector((0, 0, -0.05)), Vector((0, 0, 0.3)), 0.035, 0.025, 5)
    return {"crown": crown, "trunk": trunk}


ESSENCES = {"oak": oak, "beech": beech, "fir": fir}


def main() -> None:
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = Path(argv[0]) if argv else Path("campaign_trees.glb")
    seed = int(argv[1]) if len(argv) > 1 else 1337
    reset_scene()
    for name, build in ESSENCES.items():
        for low in (False, True):
            rng = random.Random(f"{seed}-{name}-{low}")
            parts = build(rng, low)
            for part, bm in parts.items():
                obj_name = f"{name}{'_low' if low else ''}_{part}"
                obj = new_object(obj_name, bm)
                triangles = sum(len(p.vertices) - 2 for p in obj.data.polygons)
                print(f"TREE {obj_name} {triangles}")
    out.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(out),
        export_format="GLB",
        export_yup=True,
        export_apply=True,
        use_selection=False,
        export_materials="NONE",
    )
    print("OK")


if __name__ == "__main__":
    main()
