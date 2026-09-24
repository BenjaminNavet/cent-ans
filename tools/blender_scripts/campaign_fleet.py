"""Campaign-map fleets and bivouac (lot CV2), exported as glTF binaries (.glb).

Run headless:
    blender -b --python tools/blender_scripts/campaign_fleet.py -- game/assets/models/fleet [name ...]

Models (built Z up, X forward in Blender; Y up, +X forward in Godot):

* ``cog``: 14th-century cog -- flat-bottomed clinker hull, straight raked stem and sternpost,
  fore and stern castles, single mast with a top (crow's nest), square sail, shrouds, rudder;
* ``nef``: rounder, longer hull with curved ends, taller castles and a bowsprit;
* ``bivouac``: field camp -- a striped pavilion, two ridge tents, a campfire with logs.

The sail (material ``Sail``) is a subdivided plane with UVs covering the whole sail (top-left
(0, 0) in Godot): Godot replaces the material with ``campaign_sail.gdshader`` (faction colour
and arms, wind belly). Surfaces named ``Banner`` are tinted with the faction colour. Mast top
at z = 2.05 (flag of the army marker), stern flagstaff foot at ``STERN_STAFF``. Reuses the
helpers and palette of ``models.py``.
"""

import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import models  # noqa: E402

from models import box, cone, cylinder, mesh_object  # noqa: E402

models.PALETTE["Sail"] = ((0.66, 0.61, 0.49), 0.9, 0.0)
models.PALETTE["Hull"] = ((0.14, 0.09, 0.055), 0.85, 0.0)
models.PALETTE["Rope"] = ((0.30, 0.24, 0.15), 0.95, 0.0)

MAST_TOP = 2.05
STERN_STAFF = (-0.9, 0.0, 0.95)


def rod(p0, p1, radius, mat, vertices=4):
    """Thin cylinder from ``p0`` to ``p1``."""
    a, b = Vector(p0), Vector(p1)
    direction = b - a
    rotation = Vector((0, 0, 1)).rotation_difference(direction.normalized()).to_euler()
    return cylinder(
        radius, direction.length, tuple((a + b) / 2), mat, vertices, rotation=rotation
    )


def lofted_hull(sections, ring=8, mat="Hull"):
    """Hull lofted through ``(x, half_width, top, keel)`` sections, closed at both ends."""
    verts = []
    for x, half_width, top, keel in sections:
        for index in range(ring + 1):
            angle = math.pi * index / ring
            # Flanc presque vertical en haut, fond plat arrondi aux bouchains.
            side = math.sin(angle) ** 0.6
            verts.append((x, -half_width * math.cos(angle), top - (top - keel) * side))
    faces = []
    per = ring + 1
    for s in range(len(sections) - 1):
        for index in range(ring):
            a = s * per + index
            faces.append((a, a + 1, a + per + 1, a + per))
    last = (len(sections) - 1) * per
    faces.append(tuple(range(per - 1, -1, -1)))
    faces.append(tuple(range(last, last + per)))
    return mesh_object("hull", verts, faces, mat)


def clinker_strakes(sections, count, mat="Wood"):
    """Planking lines: thin ribbons along the hull at a few heights (clinker look)."""
    parts = []
    for level in range(1, count + 1):
        t = level / (count + 1)
        for side in (-1, 1):
            verts = []
            for x, half_width, top, keel in sections:
                z = top - (top - keel) * t * 0.85
                y = side * (half_width * (1.0 - 0.25 * t * t) + 0.004)
                verts.append((x, y, z))
                verts.append((x, y, z - 0.025))
            faces = [
                (2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2) for i in range(len(sections) - 1)
            ]
            if side > 0:
                faces = [tuple(reversed(face)) for face in faces]
            parts.append(mesh_object("strake", verts, faces, mat))
    return parts


def sail(width, top, bottom, x, rows=6, cols=6):
    """Square sail in the YZ plane with UVs over the whole sail (Godot: top-left = (0, 0))."""
    verts = []
    uvs = []
    for r in range(rows + 1):
        v = r / rows
        z = top - (top - bottom) * v
        for c in range(cols + 1):
            u = c / cols
            verts.append((x, -width / 2 + width * u, z))
            uvs.append((u, 1.0 - v))
    faces = []
    for r in range(rows):
        for c in range(cols):
            a = r * (cols + 1) + c
            faces.append((a, a + cols + 1, a + cols + 2, a + 1))
    obj = mesh_object("sail", verts, faces, "Sail")
    layer = obj.data.uv_layers.new(name="UVMap")
    for poly in obj.data.polygons:
        for loop_index in poly.loop_indices:
            layer.data[loop_index].uv = uvs[obj.data.loops[loop_index].vertex_index]
    return obj


def rigging(parts, mast_x, deck, beam, stays_to):
    """Mast, yard, top, shrouds and fore/back stays."""
    parts.append(cylinder(0.035, MAST_TOP - deck + 0.1, (mast_x, 0, (MAST_TOP + deck) / 2), "Wood", 6))
    parts.append(cylinder(0.09, 0.1, (mast_x, 0, MAST_TOP - 0.18), "Wood", 8))  # hune
    parts.append(box((0.02, 1.0, 0.035), (mast_x + 0.04, 0, 1.72), "Wood"))  # vergue
    for side in (-1, 1):
        for dx in (-0.12, 0.0, 0.12):
            parts.append(rod((mast_x, 0, MAST_TOP - 0.22), (mast_x + dx - 0.08, side * beam, deck + 0.05), 0.006, "Rope"))
    for x_end, z_end in stays_to:
        parts.append(rod((mast_x, 0, MAST_TOP - 0.2), (x_end, 0, z_end), 0.006, "Rope"))


def castle(parts, x, length, width, deck, height, mat="Wood"):
    """Castle platform with a crenellated parapet (merlons on the rail)."""
    parts.append(box((length, width, height), (x, 0, deck + height / 2), mat))
    parts.append(box((length + 0.04, width + 0.04, 0.03), (x, 0, deck + height + 0.015), "Wood"))
    for side in (-1, 1):
        parts.append(box((length + 0.02, 0.02, 0.07), (x, side * (width / 2 + 0.01), deck + height + 0.06), "Banner"))
    for end in (-1, 1):
        parts.append(box((0.02, width, 0.07), (x + end * (length / 2 + 0.01), 0, deck + height + 0.06), "Banner"))


def build_cog():
    """Cog: high flat-sided hull, straight raked stem and sternpost."""
    sections = [
        (-0.98, 0.04, 0.78, 0.30),
        (-0.86, 0.22, 0.74, 0.12),
        (-0.55, 0.33, 0.64, 0.03),
        (-0.1, 0.36, 0.60, 0.0),
        (0.35, 0.35, 0.61, 0.0),
        (0.7, 0.27, 0.66, 0.05),
        (0.92, 0.12, 0.74, 0.18),
        (1.04, 0.03, 0.80, 0.34),
    ]
    parts = [lofted_hull(sections)]
    parts += clinker_strakes(sections, 3)
    parts.append(box((1.6, 0.62, 0.03), (0.0, 0, 0.5), "Wood"))  # pont
    castle(parts, -0.72, 0.42, 0.52, 0.62, 0.2)
    castle(parts, 0.8, 0.3, 0.36, 0.64, 0.14)
    parts.append(box((0.2, 0.025, 0.36), (-1.02, 0, 0.36), "Wood", rotation=(0, math.radians(8), 0)))  # gouvernail
    rigging(parts, 0.0, 0.5, 0.36, [(1.04, 0.8), (-0.9, 0.95)])
    parts.append(sail(0.92, 1.7, 0.78, 0.07))
    parts.append(cylinder(0.012, 0.5, (STERN_STAFF[0], 0, STERN_STAFF[2] + 0.25), "Wood", 4))
    return parts


def build_nef():
    """Nef: longer, rounder hull with curved ends, taller castles and a bowsprit."""
    sections = [
        (-1.12, 0.03, 0.86, 0.46),
        (-1.02, 0.18, 0.76, 0.2),
        (-0.72, 0.31, 0.62, 0.06),
        (-0.2, 0.37, 0.56, 0.0),
        (0.3, 0.37, 0.56, 0.0),
        (0.75, 0.29, 0.61, 0.07),
        (1.02, 0.15, 0.72, 0.22),
        (1.14, 0.03, 0.86, 0.45),
    ]
    parts = [lofted_hull(sections)]
    parts += clinker_strakes(sections, 4)
    parts.append(box((1.8, 0.64, 0.03), (0.0, 0, 0.47), "Wood"))
    castle(parts, -0.84, 0.46, 0.5, 0.62, 0.26)
    castle(parts, 0.92, 0.34, 0.34, 0.66, 0.2)
    parts.append(rod((1.0, 0, 0.8), (1.45, 0, 1.0), 0.02, "Wood", 6))  # beaupré
    parts.append(box((0.22, 0.025, 0.4), (-1.15, 0, 0.42), "Wood", rotation=(0, math.radians(10), 0)))
    rigging(parts, 0.08, 0.47, 0.37, [(1.45, 1.0), (-1.0, 1.0)])
    parts.append(sail(0.98, 1.72, 0.74, 0.15))
    parts.append(cylinder(0.012, 0.5, (STERN_STAFF[0], 0, STERN_STAFF[2] + 0.25), "Wood", 4))
    return parts


def ridge_tent(x, y, angle, length, width, height, mat):
    """Ridge tent (triangular prism) resting on the ground."""
    return models.gable_roof(length, width, height, (x, y, 0.0), mat, angle=angle, overhang=0.0)


def build_bivouac():
    """Field camp: striped pavilion, two ridge tents, campfire with logs; hearth at origin."""
    parts = list(models.ground_patch(1.25, "Dirt", 14))
    # Pavillon du chef : tambour de toile, toit conique aux couleurs, mât et pennon.
    parts.append(cylinder(0.55, 0.7, (-0.55, 0.45, 0.35), "Canvas", 12))
    parts.append(cone(0.62, 0.6, (-0.55, 0.45, 1.0), "Banner", 12))
    for index in range(6):
        angle = index * math.tau / 6
        parts.append(box((0.03, 0.2, 0.68), (-0.55 + 0.56 * math.cos(angle), 0.45 + 0.56 * math.sin(angle), 0.35), "Banner", rotation=(0, 0, angle)))
    parts.append(cylinder(0.015, 0.4, (-0.55, 0.45, 1.45), "Wood", 4))
    parts.append(box((0.02, 0.22, 0.1), (-0.55, 0.56, 1.58), "Banner"))
    parts.append(ridge_tent(0.55, 0.65, 0.4, 0.9, 0.62, 0.6, "Canvas"))
    parts.append(ridge_tent(-0.2, -0.75, -0.25, 0.8, 0.58, 0.55, "Canvas"))
    # Feu de camp.
    parts.append(cone(0.1, 0.2, (0.2, -0.05, 0.1), "Fire", 5))
    for angle in (0.0, 1.05, 2.1):
        parts.append(box((0.3, 0.045, 0.045), (0.2, -0.05, 0.03), "Wood", rotation=(0, 0, angle)))
    for index in range(7):
        angle = index * math.tau / 7
        parts.append(box((0.06, 0.06, 0.05), (0.2 + 0.19 * math.cos(angle), -0.05 + 0.19 * math.sin(angle), 0.025), "DarkStone"))
    # Faisceau de lances et tonneaux.
    for dx in (-0.05, 0.05):
        parts.append(rod((0.8 + dx, -0.3, 0.0), (0.8, -0.3, 0.8), 0.012, "Wood"))
    parts.append(cylinder(0.09, 0.2, (0.95, 0.15, 0.1), "Wood", 8))
    parts.append(cylinder(0.09, 0.2, (1.05, 0.0, 0.1), "Wood", 8))
    return parts


MODELS = {"cog": build_cog, "nef": build_nef, "bivouac": build_bivouac}


def main() -> None:
    """Parse ``-- <out_dir> [names...]`` and export the requested models."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(args[0]) if args else Path.cwd() / "fleet"
    names = args[1:] or list(MODELS)
    models.MODELS.update(MODELS)
    for name in names:
        print(f"MODEL {name} {models.export_model(name, out_dir)}")
    print("OK")


if __name__ == "__main__":
    main()
