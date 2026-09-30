"""Lot FK2: props of the living campaign map (folk, scenes, merchant carts), as glTF (.glb).

Run headless from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/folk_props.py -- \
        game/assets/models/folk [name ...]

Spec: ``docs/design/2026-09-29-carte-vivante-folk.md`` (sections 2.3 and 3). Every prop is
built procedurally (no third-party asset, CC0 like the rest of the generated models).

Conventions (shared with ``models.py`` and ``campaign_fleet.py``, whose helpers and palette
are reused):

* **Metric scale**, like the skinned figures of ``battle_skinned`` / ``battle_fine`` (a man
  is 1.8 m tall): the renderer applies the same factor to both (``ArmyFigures.FIGURE_SCALE``).
* Built Z up and facing **+X** in Blender; the glTF exporter converts to Godot (Y up, +X
  forward): Blender (x, y, z) -> Godot (x, z, -y). The skinned figures face +Z in Godot, so a
  figure walking with a prop turns by +PI/2 about Y, like ``ArmyFigures._add_slot``.
* Origin on the ground (z = 0) at the prop's anchor: centre of the axle for carts, feet of
  the ploughman for the plough, grip of the hand for the held props (``pitchfork``, ``torch``,
  whose shaft runs up +Z), foot of the pole for the processional cross and banner.
* One joined mesh per model with PBR materials. ``Banner`` surfaces may be tinted by Godot
  (procession banner); ``Fire`` is emissive (torch, pyre).

``manifest.json`` (written next to the models) lists each model: file, triangles, bounds
(Godot space) and named slots (Godot space, metres), e.g. where the carter walks or where the
porters hold the dead cart's handles.
"""

import json
import math
import sys
from pathlib import Path

from mathutils import Matrix, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import models  # noqa: E402
from models import box, cone, cylinder, mesh_object, sphere  # noqa: E402

# Palette additions (linear RGB, roughness, metallic).
models.PALETTE.update(
    {
        "CartWood": ((0.23, 0.15, 0.09), 0.85, 0.0),
        "OldWood": ((0.16, 0.12, 0.09), 0.9, 0.0),
        "Iron": ((0.13, 0.13, 0.14), 0.5, 0.7),
        "Linen": ((0.62, 0.58, 0.50), 0.95, 0.0),
        "Shroud": ((0.55, 0.53, 0.48), 0.95, 0.0),
        "Awning": ((0.42, 0.08, 0.05), 0.9, 0.0),
        "Wool": ((0.62, 0.58, 0.49), 1.0, 0.0),
        "SheepFace": ((0.08, 0.07, 0.06), 0.8, 0.0),
        "Cow": ((0.25, 0.14, 0.07), 0.8, 0.0),
        "CowPale": ((0.52, 0.44, 0.33), 0.85, 0.0),
        "Ox": ((0.34, 0.26, 0.17), 0.8, 0.0),
        "Horn": ((0.60, 0.54, 0.42), 0.5, 0.0),
        "Hoof": ((0.06, 0.05, 0.04), 0.6, 0.0),
        "Pitch": ((0.05, 0.04, 0.03), 0.7, 0.0),
        "Charred": ((0.05, 0.04, 0.035), 0.95, 0.0),
        "Straw": ((0.55, 0.43, 0.20), 0.95, 0.0),
        "Goods": ((0.20, 0.25, 0.40), 0.9, 0.0),
        "Goods2": ((0.45, 0.35, 0.12), 0.9, 0.0),
        "Bread": ((0.50, 0.30, 0.12), 0.9, 0.0),
        "Rope": ((0.30, 0.24, 0.15), 0.95, 0.0),
    }
)
models.EMISSIVE["Fire"] = 4.0


# --- Helpers ------------------------------------------------------------------------------


def rod(p0, p1, radius, mat, vertices=6):
    """Cylinder from ``p0`` to ``p1``."""
    a, b = Vector(p0), Vector(p1)
    direction = b - a
    rotation = Vector((0, 0, 1)).rotation_difference(direction.normalized()).to_euler()
    return cylinder(
        radius, direction.length, tuple((a + b) / 2), mat, vertices, rotation=rotation
    )


def slab(p0, p1, width, thickness, mat, up=(0, 0, 1)):
    """Plank from ``p0`` to ``p1`` (``width`` across, ``thickness`` along ``up``)."""
    a, b = Vector(p0), Vector(p1)
    axis = (b - a).normalized()
    up_v = Vector(up)
    side = axis.cross(up_v).normalized()
    up_v = side.cross(axis).normalized()
    half = (b - a).length / 2
    centre = (a + b) / 2
    verts = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                verts.append(
                    tuple(
                        centre
                        + axis * half * sx
                        + side * width / 2 * sy
                        + up_v * thickness / 2 * sz
                    )
                )
    faces = [
        (0, 1, 3, 2),
        (4, 6, 7, 5),
        (0, 4, 5, 1),
        (2, 3, 7, 6),
        (0, 2, 6, 4),
        (1, 5, 7, 3),
    ]
    return mesh_object("slab", verts, faces, mat)


def loft(name, sections, mat, ring=8, close=True):
    """Closed body through ``(x, half_width, z_centre, half_height)`` elliptic sections."""
    verts = []
    for x, hw, zc, hh in sections:
        for k in range(ring):
            a = 2 * math.pi * k / ring
            verts.append((x, hw * math.cos(a), zc + hh * math.sin(a)))
    faces = []
    for s in range(len(sections) - 1):
        for k in range(ring):
            a = s * ring + k
            b = s * ring + (k + 1) % ring
            faces.append((a, a + ring, b + ring, b))
    if close:
        faces.append(tuple(range(ring - 1, -1, -1)))
        last = (len(sections) - 1) * ring
        faces.append(tuple(range(last, last + ring)))
    return mesh_object(name, verts, faces, mat)


def wheel(x, y, radius, width=0.07, spokes=6, mat="CartWood", segments=12):
    """Spoked cart wheel in the XZ plane at (x, y), axle at height ``radius``."""
    parts = []
    inner = radius * 0.86
    verts = []
    for side in (-1, 1):
        for r in (radius, inner):
            for k in range(segments):
                a = 2 * math.pi * k / segments
                verts.append(
                    (
                        x + r * math.cos(a),
                        y + side * width / 2,
                        radius + r * math.sin(a),
                    )
                )
    n = segments
    faces = []
    for k in range(n):
        j = (k + 1) % n
        o0, i0, o1, i1 = 0, n, 2 * n, 3 * n
        faces.append((o0 + k, o0 + j, o1 + j, o1 + k))  # tread
        faces.append((i0 + k, i1 + k, i1 + j, i0 + j))  # inside of the felloe
        faces.append((o0 + k, i0 + k, i0 + j, o0 + j))  # face
        faces.append((o1 + k, o1 + j, i1 + j, i1 + k))  # face
    parts.append(mesh_object("felloe", verts, faces, mat))
    parts.append(
        cylinder(
            radius * 0.16, width * 1.8, (x, y, radius), mat, 8, (math.pi / 2, 0, 0)
        )
    )
    for k in range(spokes):
        a = 2 * math.pi * k / spokes + 0.2
        tip = (x + inner * math.cos(a), y, radius + inner * math.sin(a))
        parts.append(rod((x, y, radius), tip, 0.022, mat, 4))
    return parts


def quadruped(
    x,
    y,
    heading,
    length,
    withers,
    girth,
    coat,
    head_len,
    horns=False,
    ears=True,
    leg_mat=None,
    head_mat=None,
    neck_up=0.2,
):
    """Standing four-legged animal facing ``heading`` (radians about Z), feet at z = 0.

    ``length`` is the body length (chest to rump), ``withers`` the height of the back and
    ``girth`` the half width of the barrel.
    """
    parts = []
    leg = withers * 0.55
    belly = leg
    back = withers
    zc = (belly + back) / 2
    hh = (back - belly) / 2 + 0.02
    half = length / 2
    body = loft(
        "body",
        [
            (-half, girth * 0.45, zc + 0.02, hh * 0.55),
            (-half * 0.8, girth * 0.95, zc, hh * 0.95),
            (-half * 0.2, girth * 1.05, zc - 0.02, hh * 1.05),
            (half * 0.45, girth, zc, hh),
            (half * 0.85, girth * 0.8, zc + 0.03, hh * 0.85),
            (half, girth * 0.4, zc + 0.06, hh * 0.45),
        ],
        coat,
    )
    parts.append(body)
    # Neck and head, forward and up.
    neck_base = Vector((half * 0.8, 0, zc + hh * 0.3))
    head_c = Vector((half + head_len * 0.55, 0, back + neck_up))
    parts.append(rod(neck_base, head_c, girth * 0.42, coat, 6))
    snout = head_c + Vector((head_len * 0.75, 0, -head_len * 0.45))
    hm = head_mat or coat
    parts.append(
        loft(
            "head",
            [
                (0.0, girth * 0.32, 0.0, girth * 0.36),
                (head_len * 0.5, girth * 0.28, -0.02, girth * 0.3),
                (head_len, girth * 0.2, -0.04, girth * 0.2),
            ],
            hm,
            ring=6,
        )
    )
    head_obj = parts[-1]
    tilt = math.atan2(snout.z - head_c.z, snout.x - head_c.x)
    head_obj.rotation_euler = (0, -tilt, 0)
    head_obj.location = head_c
    if ears:
        for s in (-1, 1):
            parts.append(
                box(
                    (0.05, girth * 0.35, 0.03),
                    tuple(head_c + Vector((0.0, s * girth * 0.42, girth * 0.25))),
                    hm,
                    rotation=(s * 0.4, 0, 0),
                )
            )
    if horns:
        for s in (-1, 1):
            base = head_c + Vector((0.02, s * girth * 0.25, girth * 0.3))
            tip = base + Vector((0.05, s * girth * 0.55, girth * 0.35))
            parts.append(rod(base, tip, 0.025, "Horn", 4))
    # Legs (hooves dark).
    lm = leg_mat or coat
    for dx in (half * 0.72, -half * 0.72):
        for s in (-1, 1):
            top = Vector((dx, s * girth * 0.55, belly + 0.05))
            foot = Vector((dx, s * girth * 0.55, 0.06))
            parts.append(rod(top, foot, girth * 0.16, lm, 5))
            parts.append(
                cylinder(
                    girth * 0.17, 0.06, tuple(foot - Vector((0, 0, 0.03))), "Hoof", 5
                )
            )
    # Tail.
    parts.append(
        rod((-half, 0, back - 0.05), (-half - 0.08, 0, belly - 0.05), 0.025, coat, 4)
    )
    rotate_parts(parts, heading, Vector((0, 0, 0)))
    for p in parts:
        p.location.x += x
        p.location.y += y
    return parts


def rotate_parts(parts, angle, pivot):
    """Rotate objects about the vertical through ``pivot`` (object transforms)."""
    if abs(angle) < 1e-6:
        return
    turn = Matrix.Rotation(angle, 3, "Z")
    for p in parts:
        p.location = pivot + turn @ (p.location - pivot)
        p.rotation_euler = (turn @ p.rotation_euler.to_matrix()).to_euler()


def draft_horse(x, y=0.0, heading=0.0):
    """Draft horse or mule in harness (collar), facing +X."""
    parts = quadruped(
        x,
        y,
        heading,
        1.45,
        1.45,
        0.26,
        "Horse",
        0.55,
        leg_mat="Horse",
        neck_up=0.25,
    )
    # Collar.
    collar = cylinder(
        0.24,
        0.08,
        (x + 0.72, y, 1.35),
        "Leather",
        10,
        rotation=(0, math.radians(60), 0),
    )
    parts.append(collar)
    return parts


def ox(x, y=0.0, heading=0.0):
    """Ox (horned, heavy), facing +X."""
    return quadruped(x, y, heading, 1.6, 1.4, 0.34, "Ox", 0.5, horns=True, neck_up=-0.1)


def cart_bed(length, width, height, sides, mat="CartWood"):
    """Flat bed on two longitudinal beams, low plank sides; origin under the axle."""
    parts = []
    parts.append(box((length, width, 0.05), (0, 0, height), mat))
    for s in (-1, 1):
        parts.append(
            box((length, 0.04, sides), (0, s * width / 2, height + sides / 2), mat)
        )
    for sx in (-1, 1):
        parts.append(
            box(
                (0.04, width, sides * 0.8),
                (sx * length / 2, 0, height + sides * 0.4),
                mat,
            )
        )
    # Stakes of the sides.
    for k in range(4):
        px = -length / 2 + length * (k + 0.5) / 4
        for s in (-1, 1):
            parts.append(
                box(
                    (0.05, 0.05, sides + 0.12),
                    (px, s * (width / 2 + 0.03), height + sides / 2),
                    mat,
                )
            )
    return parts


def shafts(x0, x1, width, z0, z1, mat="CartWood"):
    """Two shafts from the bed (``x0``, ``z0``) to the hitch (``x1``, ``z1``)."""
    return [
        rod((x0, s * width / 2, z0), (x1, s * width / 2, z1), 0.035, mat, 5)
        for s in (-1, 1)
    ]


# --- Models -------------------------------------------------------------------------------


def build_merchant_cart():
    """Covered merchant cart (charrette bâchée): two wheels, canvas tilt on hoops, horse."""
    parts = []
    radius = 0.62
    bed_z = radius + 0.08
    length, width = 2.2, 1.2
    parts += cart_bed(length, width, bed_z, 0.35)
    for s in (-1, 1):
        parts += wheel(0.0, s * (width / 2 + 0.12), radius)
    parts.append(
        rod(
            (0, -width / 2 - 0.16, radius),
            (0, width / 2 + 0.16, radius),
            0.04,
            "CartWood",
        )
    )
    # Canvas tilt: half-cylinder shell along X over hoops.
    ring = 8
    verts = []
    hw = width / 2 + 0.04
    top = bed_z + 0.35
    for x in (-length / 2 - 0.05, length / 2 + 0.05):
        for k in range(ring + 1):
            a = math.pi * k / ring
            verts.append((x, hw * math.cos(a), top + 0.75 * math.sin(a)))
    faces = [(k, k + 1, ring + 2 + k, ring + 1 + k) for k in range(ring)]
    # Back closed, front open.
    faces.append(tuple(range(ring, -1, -1)))
    parts.append(mesh_object("tilt", verts, faces, "Canvas"))
    # Inner face so that the open front does not show a hollow shell.
    verts2 = [(v[0], v[1] * 0.98, top + (v[2] - top) * 0.98) for v in verts]
    faces2 = [tuple(reversed(f)) for f in faces[:-1]]
    parts.append(mesh_object("tilt_in", verts2, faces2, "Linen"))
    for k in range(3):
        px = -length / 2 + length * (k + 0.5) / 3
        parts.append(box((0.05, width + 0.1, 0.05), (px, 0, top + 0.02), "CartWood"))
    # Goods at the open front: barrel and bales.
    parts.append(
        cylinder(0.22, 0.5, (length / 2 - 0.35, 0.25, bed_z + 0.28), "OldWood", 8)
    )
    parts.append(
        box((0.4, 0.45, 0.35), (length / 2 - 0.35, -0.25, bed_z + 0.2), "Goods2")
    )
    # Shafts to a horse.
    parts += shafts(length / 2, 2.6, 0.75, bed_z, 1.05)
    parts += draft_horse(3.0)
    return parts


def build_dead_cart():
    """Plague cart: open hand cart with shrouded bodies, drawn by two porters by the handles."""
    parts = []
    radius = 0.5
    bed_z = radius + 0.06
    length, width = 1.9, 1.1
    parts += cart_bed(length, width, bed_z, 0.3, mat="OldWood")
    for s in (-1, 1):
        parts += wheel(0.0, s * (width / 2 + 0.1), radius, mat="OldWood", spokes=6)
    parts.append(
        rod(
            (0, -width / 2 - 0.14, radius),
            (0, width / 2 + 0.14, radius),
            0.04,
            "OldWood",
        )
    )
    # Shrouded bodies lying along the bed.
    for k, (dy, dz, ang) in enumerate(
        ((-0.28, 0.0, 0.05), (0.02, 0.02, -0.04), (0.3, 0.0, 0.08), (-0.1, 0.2, 0.3))
    ):
        body = loft(
            "shroud",
            [
                (-0.85, 0.08, 0.0, 0.07),
                (-0.6, 0.15, 0.0, 0.1),
                (0.0, 0.2, 0.0, 0.12),
                (0.45, 0.17, 0.0, 0.11),
                (0.65, 0.1, 0.02, 0.1),
                (0.85, 0.09, 0.02, 0.09),
            ],
            "Shroud",
            ring=6,
        )
        body.location = (0.0 if k < 3 else 0.1, dy, bed_z + 0.14 + dz)
        body.rotation_euler = (0, 0, ang)
        parts.append(body)
    # Handles forward (the porters walk between them).
    parts += shafts(length / 2, 2.1, 0.8, bed_z, 0.85, mat="OldWood")
    parts.append(rod((1.9, -0.4, 0.83), (1.9, 0.4, 0.83), 0.03, "OldWood", 5))
    return parts


def build_stone_cart():
    """Stone cart: open cart loaded with dressed blocks, drawn by an ox."""
    parts = []
    radius = 0.55
    bed_z = radius + 0.08
    length, width = 2.0, 1.15
    parts += cart_bed(length, width, bed_z, 0.22)
    for s in (-1, 1):
        parts += wheel(0.0, s * (width / 2 + 0.12), radius, spokes=8)
    parts.append(
        rod(
            (0, -width / 2 - 0.16, radius),
            (0, width / 2 + 0.16, radius),
            0.045,
            "CartWood",
        )
    )
    blocks = [
        (-0.55, -0.25, 0.0, 0.5, 0.4, 0.3),
        (-0.55, 0.25, 0.0, 0.5, 0.4, 0.3),
        (0.05, -0.2, 0.0, 0.55, 0.45, 0.32),
        (0.05, 0.28, 0.0, 0.45, 0.4, 0.3),
        (0.6, 0.0, 0.0, 0.45, 0.6, 0.3),
        (-0.3, 0.0, 0.3, 0.5, 0.45, 0.28),
    ]
    for bx, by, bz, sx, sy, sz in blocks:
        parts.append(
            box((sx, sy, sz), (bx, by, bed_z + 0.03 + bz + sz / 2), "StoneLight")
        )
    parts += shafts(length / 2, 2.5, 0.8, bed_z, 0.95)
    parts += ox(3.0)
    # Yoke over the neck.
    parts.append(box((0.12, 0.9, 0.1), (3.75, 0, 1.3), "CartWood"))
    return parts


def build_market_stall():
    """Market stall (étal): trestle table under a striped awning, goods, barrels, baskets."""
    parts = []
    length, depth = 2.4, 1.1
    table_z = 0.85
    # Posts.
    for sx in (-1, 1):
        for sy, height in ((-1, 2.2), (1, 1.8)):
            parts.append(
                cylinder(
                    0.05,
                    height,
                    (sx * length / 2, sy * depth / 2, height / 2),
                    "OldWood",
                    6,
                )
            )
    # Table on trestles.
    parts.append(box((length - 0.1, depth * 0.8, 0.06), (0, 0, table_z), "CartWood"))
    for sx in (-0.8, 0.8):
        for sy in (-1, 1):
            parts.append(
                rod(
                    (sx, sy * 0.3, 0),
                    (sx, sy * 0.1, table_z - 0.03),
                    0.03,
                    "OldWood",
                    4,
                )
            )
    # Sloped awning, stripes alternating canvas / red.
    stripes = 6
    for k in range(stripes):
        x0 = -length / 2 - 0.1 + (length + 0.2) * k / stripes
        x1 = x0 + (length + 0.2) / stripes
        verts = [
            (x0, -depth / 2 - 0.25, 2.25),
            (x1, -depth / 2 - 0.25, 2.25),
            (x1, depth / 2 + 0.35, 1.72),
            (x0, depth / 2 + 0.35, 1.72),
        ]
        mat = "Awning" if k % 2 else "Linen"
        parts.append(mesh_object("awning", verts, [(0, 1, 2, 3), (3, 2, 1, 0)], mat))
    # Valance along the front edge.
    parts.append(box((length + 0.2, 0.02, 0.18), (0, depth / 2 + 0.35, 1.64), "Awning"))
    # Goods on the table: cloth bolts, loaves, cheeses, a basket.
    for k, mat in enumerate(("Goods", "Goods2", "Awning", "Linen")):
        parts.append(
            cylinder(
                0.07,
                0.5,
                (-0.8 + k * 0.18, 0.1, table_z + 0.1),
                mat,
                6,
                (math.pi / 2, 0, 0),
            )
        )
    for k in range(4):
        parts.append(
            sphere(
                0.07,
                (0.05 + k * 0.14, 0.2, table_z + 0.07),
                "Bread",
                1,
                (1.2, 1.0, 0.6),
            )
        )
    parts.append(cylinder(0.14, 0.08, (0.35, -0.15, table_z + 0.07), "Horn", 8))
    parts.append(
        cone(0.2, 0.2, (0.8, 0.0, table_z + 0.13), "Straw", 8, radius_top=0.24)
    )
    # Barrels and a sack beside the stall.
    parts.append(cylinder(0.25, 0.6, (length / 2 + 0.35, 0.2, 0.3), "OldWood", 8))
    parts.append(cylinder(0.22, 0.55, (length / 2 + 0.3, -0.35, 0.27), "OldWood", 8))
    parts.append(
        sphere(0.25, (-length / 2 - 0.3, 0.35, 0.25), "Linen", 1, (0.9, 0.9, 1.1))
    )
    return parts


def build_pyre():
    """Pyre (bûcher): log crib with brushwood, burning at the base (``Fire`` emissive)."""
    parts = []
    log_r = 0.08
    for layer in range(6):
        z = log_r + layer * 2 * log_r * 0.95
        along_x = layer % 2 == 0
        span = 1.6 - layer * 0.08
        for k in range(4):
            off = -span / 2 + span * (k + 0.5) / 4
            if along_x:
                p0, p1 = (-span / 2, off, z), (span / 2, off, z)
            else:
                p0, p1 = (off, -span / 2, z), (off, span / 2, z)
            parts.append(
                rod(p0, p1, log_r, "OldWood" if (layer + k) % 3 else "Charred", 6)
            )
    # Brushwood heaped on top.
    parts.append(cone(0.7, 0.55, (0, 0, 1.2), "Straw", 8))
    # Flames licking at the base and through the crib.
    for k in range(7):
        a = 2 * math.pi * k / 7
        parts.append(
            cone(0.18, 0.7, (0.55 * math.cos(a), 0.55 * math.sin(a), 0.35), "Fire", 5)
        )
    parts.append(cone(0.35, 1.1, (0, 0, 0.9), "Fire", 6))
    # Ash and charred ground.
    parts.append(cylinder(1.25, 0.02, (0, 0, 0.01), "Charred", 12))
    return parts


def build_pitchfork():
    """Pitchfork held in the hand: origin at the right fist, shaft up +Z, tines at the top."""
    parts = [rod((0, 0, -0.75), (0, 0, 1.05), 0.018, "CartWood", 6)]
    parts.append(rod((0, -0.06, 1.05), (0, 0.06, 1.05), 0.012, "Iron", 4))
    for s in (-1, 1):
        parts.append(rod((0, s * 0.055, 1.05), (0.02, s * 0.06, 1.4), 0.008, "Iron", 4))
    parts.append(rod((0, 0, 1.0), (0, 0, 1.08), 0.022, "Iron", 6))
    return parts


def build_torch():
    """Torch held in the hand: origin at the fist, pitch-soaked head and flame at the top."""
    parts = [rod((0, 0, -0.25), (0, 0, 0.45), 0.02, "CartWood", 6)]
    parts.append(cylinder(0.04, 0.18, (0, 0, 0.52), "Pitch", 7))
    parts.append(rod((0, 0, 0.46), (0, 0, 0.47), 0.045, "Iron", 7))
    parts.append(cone(0.06, 0.3, (0, 0, 0.74), "Fire", 6))
    parts.append(cone(0.035, 0.2, (0.02, 0.01, 0.86), "Fire", 5))
    return parts


def build_sheep():
    """Sheep: woolly barrel, dark face and legs; facing +X."""
    parts = []
    parts.append(sphere(0.3, (0, 0, 0.5), "Wool", 1, (1.45, 0.95, 0.85)))
    parts.append(sphere(0.22, (-0.12, 0, 0.58), "Wool", 1, (1.2, 1.0, 0.8)))
    head = loft(
        "head",
        [(0.0, 0.08, 0.0, 0.09), (0.12, 0.07, -0.02, 0.07), (0.22, 0.045, -0.04, 0.05)],
        "SheepFace",
        ring=6,
    )
    head.location = (0.42, 0, 0.62)
    head.rotation_euler = (0, math.radians(30), 0)
    parts.append(head)
    parts.append(sphere(0.1, (0.4, 0, 0.68), "Wool", 1, (1.0, 1.1, 0.8)))
    for s in (-1, 1):
        parts.append(
            box(
                (0.04, 0.12, 0.03),
                (0.44, s * 0.1, 0.66),
                "SheepFace",
                rotation=(s * 0.5, 0, 0),
            )
        )
    for dx in (0.25, -0.25):
        for s in (-1, 1):
            parts.append(
                rod((dx, s * 0.11, 0.35), (dx, s * 0.11, 0.0), 0.025, "SheepFace", 4)
            )
    parts.append(rod((-0.44, 0, 0.55), (-0.5, 0, 0.35), 0.03, "Wool", 4))
    return parts


def build_cow():
    """Cow (pale dun, short horns), facing +X."""
    return quadruped(
        0,
        0,
        0.0,
        1.55,
        1.3,
        0.3,
        "CowPale",
        0.48,
        horns=True,
        head_mat="Cow",
        neck_up=-0.05,
    )


def build_ox():
    """Ox alone (herds, draught), facing +X."""
    return ox(0.0)


def build_horse():
    """Draft horse in collar, facing +X (carts, farm work)."""
    return draft_horse(0.0)


def build_procession_cross():
    """Processional cross: gilt cross with a knop on a tall staff; origin at the staff's foot."""
    parts = [rod((0, 0, 0), (0, 0, 2.55), 0.02, "CartWood", 6)]
    parts.append(sphere(0.05, (0, 0, 2.55), "Gold", 1))
    parts.append(box((0.03, 0.05, 0.62), (0, 0, 2.9), "Gold"))
    parts.append(box((0.03, 0.42, 0.05), (0, 0, 3.02), "Gold"))
    for p in ((0, 0, 3.22), (0, 0.22, 3.02), (0, -0.22, 3.02)):
        parts.append(sphere(0.035, p, "Gold", 1))
    parts.append(box((0.035, 0.06, 0.1), (0, 0, 3.0), "Linen"))
    return parts


def build_procession_banner():
    """Processional banner (gonfanon): T staff, cloth with lappets, gilt cross; ``Banner``."""
    parts = [rod((0, 0, 0), (0, 0, 3.1), 0.022, "CartWood", 6)]
    parts.append(rod((0, -0.45, 2.95), (0, 0.45, 2.95), 0.018, "CartWood", 5))
    parts.append(sphere(0.045, (0, 0, 3.12), "Gold", 1))
    # Cloth hanging from the crossbar: 0.85 wide, three lappets at the bottom.
    verts = []
    faces = []
    top, bottom = 2.93, 1.95
    cols = 6
    for r, z in enumerate((top, (top + bottom) / 2, bottom)):
        for c in range(cols + 1):
            y = -0.42 + 0.84 * c / cols
            verts.append((0.03 * math.sin(c * 1.3 + r), y, z))
    for r in range(2):
        for c in range(cols):
            a = r * (cols + 1) + c
            faces.append((a, a + 1, a + cols + 2, a + cols + 1))
    base = len(verts)
    for k in range(3):
        y0 = -0.42 + 0.84 * k / 3
        y1 = y0 + 0.84 / 3
        verts += [
            (0, y0 + 0.02, bottom),
            (0, y1 - 0.02, bottom),
            (0, (y0 + y1) / 2, bottom - 0.28),
        ]
        faces.append((base + 3 * k, base + 3 * k + 1, base + 3 * k + 2))
    faces += [tuple(reversed(f)) for f in list(faces)]
    parts.append(mesh_object("cloth", verts, faces, "Banner"))
    # Gilt cross on both faces.
    for dx in (-0.03, 0.06):
        parts.append(box((0.01, 0.07, 0.5), (dx, 0, 2.47), "Gold"))
        parts.append(box((0.01, 0.34, 0.07), (dx, 0, 2.6), "Gold"))
    # Fringe.
    parts.append(box((0.03, 0.86, 0.04), (0, 0, top - 0.01), "Gold"))
    return parts


def build_scaffold():
    """Timber scaffold against a half-built stone wall, ladder, hoist and block pile.

    Origin at the foot of the scaffold's middle; the wall runs along Y behind it (-X side),
    the working face towards +X.
    """
    parts = []
    width, depth = 4.2, 1.3
    levels = (1.9, 3.8, 5.6)
    # Half-built wall behind (ragged top).
    wall_x = -0.35
    steps = 7
    for k in range(steps):
        y0 = -width / 2 - 0.3 + (width + 0.6) * k / steps
        y1 = y0 + (width + 0.6) / steps
        h = 6.2 - 1.6 * abs(k - 2.5) / 3.5 - (0.5 if k % 2 else 0.0)
        parts.append(
            box(
                (0.7, y1 - y0, h),
                (wall_x - 0.35, (y0 + y1) / 2, h / 2 - 0.2),
                "StoneLight",
            )
        )
    # Standards (poles) in two rows.
    for k in range(4):
        y = -width / 2 + width * k / 3
        for x in (0.05, depth):
            parts.append(rod((x, y, -0.2), (x, y, 6.6), 0.055, "OldWood", 6))
    # Ledgers and putlogs, planks at each level.
    for z in levels:
        for x in (0.05, depth):
            parts.append(
                rod(
                    (x, -width / 2 - 0.15, z),
                    (x, width / 2 + 0.15, z),
                    0.045,
                    "OldWood",
                    5,
                )
            )
        for k in range(4):
            y = -width / 2 + width * k / 3
            parts.append(
                rod(
                    (wall_x, y, z + 0.06),
                    (depth + 0.1, y, z + 0.06),
                    0.04,
                    "OldWood",
                    5,
                )
            )
        for k in range(4):
            x = 0.2 + (depth - 0.2) * (k + 0.5) / 4
            parts.append(box((0.26, width + 0.2, 0.04), (x, 0, z + 0.12), "CartWood"))
    # Diagonal braces.
    for x in (depth,):
        parts.append(rod((x, -width / 2, 0.0), (x, 0.0, levels[1]), 0.04, "OldWood", 5))
        parts.append(rod((x, width / 2, 0.0), (x, 0.0, levels[1]), 0.04, "OldWood", 5))
    # Ladder on the front.
    lx, ly = depth + 0.35, width / 2 - 0.5
    for s in (-0.2, 0.2):
        parts.append(
            rod(
                (lx + 0.45, ly + s, 0),
                (lx - 0.05, ly + s, levels[0] + 0.9),
                0.03,
                "CartWood",
                4,
            )
        )
    for k in range(8):
        t = (k + 0.5) / 8
        px = lx + 0.45 - 0.5 * t
        pz = (levels[0] + 0.9) * t
        parts.append(rod((px, ly - 0.2, pz), (px, ly + 0.2, pz), 0.018, "CartWood", 4))
    # Hoist: gin pole with a pulley and a rope down to a hanging block.
    parts.append(
        rod(
            (depth - 0.05, -width / 2 + 0.2, levels[2]),
            (depth + 0.9, -width / 2 + 0.2, 7.1),
            0.05,
            "OldWood",
            5,
        )
    )
    parts.append(
        cylinder(
            0.1,
            0.05,
            (depth + 0.9, -width / 2 + 0.2, 7.05),
            "Iron",
            8,
            (math.pi / 2, 0, 0),
        )
    )
    parts.append(
        rod(
            (depth + 0.98, -width / 2 + 0.2, 7.0),
            (depth + 0.98, -width / 2 + 0.2, 3.1),
            0.012,
            "Rope",
            4,
        )
    )
    parts.append(
        box((0.5, 0.35, 0.3), (depth + 0.98, -width / 2 + 0.2, 2.95), "StoneLight")
    )
    # Block pile and mortar tub on the ground.
    for bx, by, bz in (
        (2.3, -0.6, 0.15),
        (2.3, -0.05, 0.15),
        (2.8, -0.35, 0.15),
        (2.55, -0.35, 0.45),
    ):
        parts.append(box((0.5, 0.4, 0.3), (bx, by, bz), "StoneLight"))
    parts.append(cylinder(0.3, 0.3, (2.4, 0.9, 0.15), "OldWood", 8))
    parts.append(cylinder(0.26, 0.02, (2.4, 0.9, 0.29), "Plaster", 8))
    return parts


def build_plough():
    """Heavy wheeled plough behind a yoked pair of oxen; origin at the ploughman's feet.

    The ploughman (clip ``plough``) stands at the origin facing +X with his hands on the two
    stilts (handles) about 0.55 m ahead, 0.85 m up.
    """
    parts = []
    # Stilts (handles) from the hands down to the share.
    for s in (-1, 1):
        parts.append(
            rod((0.5, s * 0.22, 0.86), (1.25, s * 0.08, 0.2), 0.025, "CartWood", 5)
        )
    # Sole, share, mouldboard and coulter.
    parts.append(box((0.8, 0.12, 0.08), (1.45, 0.0, 0.05), "CartWood"))
    parts.append(
        cone(0.07, 0.3, (1.95, 0.0, 0.05), "Iron", 4, rotation=(0, math.pi / 2, 0))
    )
    parts.append(
        slab((1.25, 0.08, 0.06), (1.7, 0.2, 0.3), 0.3, 0.03, "CartWood", up=(0, 1, 0))
    )
    parts.append(rod((1.85, 0, 0.1), (1.95, 0, 0.75), 0.025, "Iron", 4))
    # Beam to the wheeled fore-carriage.
    parts.append(rod((1.25, 0, 0.3), (2.9, 0, 0.72), 0.06, "CartWood", 6))
    for s in (-1, 1):
        parts += wheel(2.95, s * 0.42, 0.38, width=0.05, spokes=6)
    parts.append(rod((2.95, -0.45, 0.38), (2.95, 0.45, 0.38), 0.035, "CartWood"))
    # Draught chain / pole to the yoke.
    parts.append(rod((3.0, 0, 0.72), (4.4, 0, 1.12), 0.035, "Iron", 4))
    for s in (-1, 1):
        parts += ox(5.1, s * 0.45)
    parts.append(box((0.14, 1.6, 0.12), (5.85, 0, 1.3), "CartWood"))
    # A strip of turned earth under the plough.
    parts.append(box((1.8, 0.35, 0.04), (1.0, 0.25, 0.015), "Dirt"))
    return parts


MODELS = {
    "merchant_cart": build_merchant_cart,
    "dead_cart": build_dead_cart,
    "stone_cart": build_stone_cart,
    "market_stall": build_market_stall,
    "pyre": build_pyre,
    "pitchfork": build_pitchfork,
    "torch": build_torch,
    "sheep": build_sheep,
    "cow": build_cow,
    "ox": build_ox,
    "horse": build_horse,
    "procession_cross": build_procession_cross,
    "procession_banner": build_procession_banner,
    "scaffold": build_scaffold,
    "plough": build_plough,
}

# Named slots (Blender space, metres; converted to Godot in the manifest) and roles.
SLOTS = {
    "merchant_cart": {
        "carter": (2.2, -0.7, 0.0),
        "merchant": (-0.6, -0.95, 0.0),
        "guard": (-1.8, 0.0, 0.0),
    },
    "dead_cart": {"porter_l": (1.95, 0.25, 0.0), "porter_r": (1.95, -0.25, 0.0)},
    "stone_cart": {"carter": (3.3, -0.75, 0.0)},
    "market_stall": {"seller": (0.0, -0.9, 0.0), "buyer": (0.0, 1.2, 0.0)},
    "pyre": {},
    "pitchfork": {"grip": (0.0, 0.0, 0.0)},
    "torch": {"grip": (0.0, 0.0, 0.0), "flame": (0.0, 0.0, 0.8)},
    "sheep": {},
    "cow": {},
    "ox": {},
    "horse": {},
    "procession_cross": {"grip": (0.0, 0.0, 1.2), "top": (0.0, 0.0, 3.25)},
    "procession_banner": {"grip": (0.0, 0.0, 1.2), "top": (0.0, 0.0, 3.1)},
    "scaffold": {
        "level_1": (0.7, 0.0, 2.0),
        "level_2": (0.7, 0.0, 3.9),
        "ladder_foot": (1.85, 1.6, 0.0),
        "pile": (2.5, -0.35, 0.0),
    },
    "plough": {"ploughman": (0.0, 0.0, 0.0), "drover": (5.4, -1.0, 0.0)},
}

ROLES = {
    "merchant_cart": "cart",
    "dead_cart": "cart",
    "stone_cart": "cart",
    "market_stall": "static",
    "pyre": "static",
    "pitchfork": "held",
    "torch": "held",
    "sheep": "animal",
    "cow": "animal",
    "ox": "animal",
    "horse": "animal",
    "procession_cross": "carried",
    "procession_banner": "carried",
    "scaffold": "static",
    "plough": "static",
}


def to_godot(p):
    """Blender (x, y, z) -> Godot (x, z, -y), rounded."""
    return [round(p[0], 3), round(p[2], 3), round(-p[1], 3)]


def bounds():
    """Bounds of the exported object (Godot space): (min, max)."""
    import bpy

    obj = bpy.context.active_object
    pts = [obj.matrix_world @ v.co for v in obj.data.vertices]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    a, b = to_godot(lo), to_godot(hi)
    return [min(a[i], b[i]) for i in range(3)], [max(a[i], b[i]) for i in range(3)]


def main() -> None:
    """Parse ``-- <out_dir> [names...]``, export the models and update the manifest."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(args[0]) if args else Path.cwd() / "folk"
    names = args[1:] or list(MODELS)
    models.MODELS.update(MODELS)
    manifest_path = out_dir / "manifest.json"
    manifest = {"models": {}}
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
    for name in names:
        triangles = models.export_model(name, out_dir)
        lo, hi = bounds()
        manifest["models"][name] = {
            "file": f"{name}.glb",
            "role": ROLES[name],
            "tris": triangles,
            "bounds": [lo, hi],
            "slots": {k: to_godot(v) for k, v in SLOTS[name].items()},
        }
        print(f"MODEL {name} {triangles}")
    manifest["source"] = "tools/blender_scripts/folk_props.py (procedural, CC0)"
    manifest["units"] = (
        "metres, Godot space (Y up, +X forward), same scale as battle figures"
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print("OK")


if __name__ == "__main__":
    main()
