"""Procedural 14th-century equipment for the skinned battle figures (lot V2).

Every builder takes a `Context` (armature at rest, level of detail, material factory) and
returns new mesh objects in Blender world space, already bound to bones through vertex
groups (rigid: one bone at weight 1; blended: weights by position). Sizes are taken from the
rest pose of the imported Quaternius parts (`Context.part_bbox`).
"""

import math

import bmesh
import bpy
from mathutils import Vector

# Material codes (COLOR.a in the exported mesh, read by `battle_soldier_skinned.gdshader`).
C_LIVERY = 0
C_TRIM = 1
C_PLATE = 2
C_MAIL = 3
C_CLOTH = 4
C_EXACT = 5
C_ARMS = 6
C_SKIN = 7
C_LEATHER = 8
C_HAIR = 9
C_COAT = 10
C_WOOD = 11
C_QUILT = 12

STEEL = (0.50, 0.51, 0.53)
MAIL = (0.34, 0.34, 0.35)
WOOD = (0.26, 0.16, 0.08)
WOOD_LIGHT = (0.42, 0.29, 0.15)
LEATHER = (0.20, 0.11, 0.05)
IRON = (0.16, 0.16, 0.17)
STRING = (0.70, 0.66, 0.55)
LINEN = (0.62, 0.57, 0.46)
WHITE = (0.85, 0.85, 0.85)

# Quaternius material names -> (code, linear colour), unless a figure overrides them.
DEFAULT_COLORS = {
    "Skin": (C_SKIN, (0.50, 0.33, 0.21)),
    "Eye": (C_EXACT, (0.02, 0.015, 0.01)),
    "Eyebrows": (C_HAIR, (0.05, 0.035, 0.02)),
    "Hair": (C_HAIR, (0.06, 0.03, 0.012)),
    "Hair_White": (C_HAIR, (0.35, 0.33, 0.30)),
    "Metal": (C_PLATE, STEEL),
    "Metal_Dark": (C_PLATE, (0.30, 0.30, 0.31)),
    "Black": (C_LEATHER, (0.04, 0.035, 0.03)),
    "Grey": (C_LEATHER, (0.07, 0.06, 0.05)),
    "Brown": (C_LEATHER, (0.14, 0.09, 0.045)),
    "Brown2": (C_LEATHER, (0.09, 0.06, 0.03)),
    "DarkBrown": (C_LEATHER, (0.07, 0.035, 0.015)),
    "LightBrown": (C_LEATHER, (0.18, 0.09, 0.035)),
    "Gold": (C_TRIM, (0.6, 0.45, 0.15)),
    "Green": (C_CLOTH, (0.16, 0.14, 0.07)),
    "LightGreen": (C_CLOTH, (0.24, 0.22, 0.12)),
    "Blue": (C_CLOTH, (0.07, 0.09, 0.14)),
    "LightBlue": (C_CLOTH, (0.12, 0.10, 0.08)),
    "Beige": (C_CLOTH, LINEN),
    "Red": (C_CLOTH, (0.30, 0.04, 0.03)),
    "White": (C_EXACT, WHITE),
}


def srgb_preview(rgb):
    """Linear colour to a displayable viewport colour (previews only)."""
    return tuple(min(1.0, c ** (1 / 2.2)) for c in rgb)


class Context:
    """Rest-pose armature, level of detail and material factory for the builders."""

    def __init__(self, arm, level, material, bone_world):
        """Store the rest-pose armature, level of detail and the material/bone helpers."""
        self.arm = arm
        self.level = level
        self.material = material
        self._bone_world = bone_world
        self._bbox = {}

    def seg(self, full, medium, far):
        """Segment count for the current level of detail."""
        return (full, medium, far)[self.level]

    def head(self, bone):
        """World position of a bone head (rest)."""
        return self._bone_world(self.arm, bone).to_translation()

    def tail(self, bone):
        """World position of a bone's tail (rest)."""
        return self.arm.matrix_world @ self.arm.data.bones[bone].tail_local

    def frame(self, bone):
        """World matrix of a bone (rest)."""
        return self._bone_world(self.arm, bone)

    def part_bbox(self, bone):
        """World bounding box (min, max) of the part vertices dominated by `bone`."""
        if bone in self._bbox:
            return self._bbox[bone]
        mn = Vector((1e9, 1e9, 1e9))
        mx = -mn
        for obj in self.arm.children:
            if obj.type != "MESH" or bone not in obj.vertex_groups:
                continue
            gi = obj.vertex_groups[bone].index
            mw = obj.matrix_world
            for v in obj.data.vertices:
                best = max(v.groups, key=lambda g: g.weight, default=None)
                if best is not None and best.group == gi and best.weight > 0.5:
                    w = mw @ v.co
                    mn = Vector(map(min, mn, w))
                    mx = Vector(map(max, mx, w))
        self._bbox[bone] = (mn, mx)
        return mn, mx


# --- Mesh helpers -----------------------------------------------------------------------


def to_object(name, bm, materials):
    """Mesh object from a bmesh (faces keep their material_index)."""
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    for m in materials:
        me.materials.append(m)
    return obj


def bind_rigid(obj, bone):
    """Every vertex follows `bone`."""
    g = obj.vertex_groups.new(name=bone)
    g.add(range(len(obj.data.vertices)), 1.0, "REPLACE")


def bind_by(obj, weigh):
    """Weights per vertex: `weigh(world_pos) -> {bone: weight}`."""
    mw = obj.matrix_world
    groups = {}
    for v in obj.data.vertices:
        for bone, w in weigh(mw @ v.co).items():
            if w <= 0.0:
                continue
            if bone not in groups:
                groups[bone] = obj.vertex_groups.new(name=bone)
            groups[bone].add([v.index], w, "REPLACE")


def smoothstep(a, b, x):
    """Smooth Hermite interpolation of `x` between `a` and `b`, clamped to [0, 1]."""
    t = min(max((x - a) / (b - a), 0.0), 1.0)
    return t * t * (3 - 2 * t)


def ring(bm, center, rx, ry, n, z_of=None, start=0.0, arc=2 * math.pi, closed=True):
    """Ring of vertices around `center` (horizontal ellipse)."""
    verts = []
    count = n if closed else n + 1
    for i in range(count):
        a = start + arc * i / n
        p = Vector((center.x + rx * math.cos(a), center.y + ry * math.sin(a), center.z))
        if z_of:
            p.z = z_of(p, a)
        verts.append(bm.verts.new(p))
    return verts


def bridge(bm, a, b, mat=0, closed=True, flip=False):
    """Quads between two vertex rings of the same size."""
    n = len(a)
    last = n if closed else n - 1
    for i in range(last):
        j = (i + 1) % n
        quad = (a[i], a[j], b[j], b[i]) if not flip else (a[i], b[i], b[j], a[j])
        f = bm.faces.new(quad)
        f.material_index = mat


def cap(bm, loop, mat=0, flip=False):
    """Close a vertex loop with a single n-gon face."""
    f = bm.faces.new(list(reversed(loop)) if flip else loop)
    f.material_index = mat
    return f


def tube(bm, a, b, ra, rb, n, mat=0, caps=True):
    """Tapered closed cylinder from `a` to `b`."""
    axis = (b - a).normalized()
    helper = Vector((0, 0, 1)) if abs(axis.z) < 0.9 else Vector((1, 0, 0))
    u = axis.cross(helper).normalized()
    v = axis.cross(u).normalized()
    ring_a = [
        bm.verts.new(
            a
            + (u * math.cos(2 * math.pi * i / n) + v * math.sin(2 * math.pi * i / n))
            * ra
        )
        for i in range(n)
    ]
    ring_b = [
        bm.verts.new(
            b
            + (u * math.cos(2 * math.pi * i / n) + v * math.sin(2 * math.pi * i / n))
            * rb
        )
        for i in range(n)
    ]
    bridge(bm, ring_a, ring_b, mat)
    if caps:
        cap(bm, ring_a, mat, flip=False)
        cap(bm, ring_b, mat, flip=True)
    return ring_a, ring_b


def box(bm, center, size, axes=None, mat=0):
    """Box with half-size `size` along `axes` (default world axes)."""
    ax = axes or (Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1)))
    corners = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                corners.append(
                    bm.verts.new(
                        center
                        + ax[0] * size[0] * sx
                        + ax[1] * size[1] * sy
                        + ax[2] * size[2] * sz
                    )
                )
    c = corners
    faces = [
        (0, 1, 3, 2),
        (4, 6, 7, 5),
        (0, 4, 5, 1),
        (2, 3, 7, 6),
        (0, 2, 6, 4),
        (1, 5, 7, 3),
    ]
    for f in faces:
        face = bm.faces.new([c[i] for i in f])
        face.material_index = mat
    return c


def finish(bm):
    """Recalculate face normals of a bmesh so they point outward."""
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)


# --- Grip frames ------------------------------------------------------------------------


def grip(ctx, side="R"):
    """Centre of the fist and the grip axes at rest (T-pose, palm down).

    Returns (centre, along: the weapon's forward direction through the fist, up, out).
    """
    wrist = ctx.head(f"Wrist.{side}")
    # The hand extends the forearm at rest (bone axes of the imported rig are not reliable).
    out = (wrist - ctx.head(f"LowerArm.{side}")).normalized()
    centre = wrist + out * 0.075 + Vector((0, 0, -0.01))
    along = Vector((0, -1, 0))
    up = out.cross(along).normalized()
    return centre, along, up, out


def nock_rest(ctx):
    """Rest position of the bow string's middle (the bow is in the left fist, T-pose)."""
    c, _along, _up, out = grip(ctx, "L")
    return c - out * 0.16


# --- Helmets ----------------------------------------------------------------------------


def _head_box(ctx):
    mn, mx = ctx.part_bbox("Head")
    centre = (mn + mx) / 2
    half = (mx - mn) / 2
    return centre, half, mn, mx


def bassinet(ctx, aventail=True, visor=False):
    """Pointed bassinet (open face) with a mail aventail down to the shoulders."""
    centre, half, mn, mx = _head_box(ctx)
    bm = bmesh.new()
    n = ctx.seg(16, 8, 6)
    rows = ctx.seg(8, 4, 3)
    rx, ry, _rz = half.x * 1.13, half.y * 1.12, half.z * 0.62
    base_z = mn.z + (mx.z - mn.z) * 0.42
    top = Vector((centre.x, centre.y + half.y * 0.25, mx.z + 0.07))
    rings = []
    for r in range(rows):
        t = r / rows
        ang = t * math.pi / 2
        rr = math.cos(ang)
        z = base_z + math.sin(ang) * (mx.z - base_z + 0.01)

        # Face opening: the front of the rim rises to the brow.
        def z_of(p, a, z=z, t=t):
            front = max(0.0, -math.sin(a))  # -Y is the face
            lift = (1 - t) * 0.09 * smoothstep(0.35, 0.8, front)
            return z + lift

        ring_verts = ring(
            bm,
            Vector((centre.x, centre.y + half.y * 0.08 * t, z)),
            rx * rr + 0.004,
            ry * rr + 0.004,
            n,
            z_of,
        )
        rings.append(ring_verts)
    apex = bm.verts.new(top)
    for a, b in zip(rings, rings[1:], strict=False):
        bridge(bm, a, b, 0)
    for i in range(n):
        f = bm.faces.new((rings[-1][i], rings[-1][(i + 1) % n], apex))
        f.material_index = 0
    finish(bm)
    obj = to_object("bassinet", bm, [ctx.material(C_PLATE, STEEL)])
    bind_rigid(obj, "Head")
    out = [obj]
    if aventail:
        out.append(_aventail(ctx, centre, half, mn, base_z, n))
    return out


def _aventail(ctx, centre, half, mn, base_z, n):
    """Mail cape from the helmet rim to the shoulders, open at the face."""
    bm = bmesh.new()
    chest = ctx.head("Chest")
    neck = ctx.head("Neck")
    shoulder_z = chest.z + 0.06
    levels = ctx.seg(4, 2, 2)
    rings = []
    for k in range(levels + 1):
        t = k / levels
        z = base_z + 0.01 + (shoulder_z - base_z - 0.01) * t
        rx = half.x * 1.1 * (1 - t) + 0.19 * t
        ry = half.y * 1.08 * (1 - t) + 0.13 * t
        cy = centre.y * (1 - t) + neck.y * t

        def z_of(p, a, z=z, t=t):
            front = max(0.0, -math.sin(a))
            return z - (0.05 * (1 - t) + 0.02) * front + (1 - t) * 0.0

        rings.append(ring(bm, Vector((centre.x, cy, z)), rx, ry, n, z_of))
    for a, b in zip(rings, rings[1:], strict=False):
        bridge(bm, a, b, 0)
    finish(bm)
    obj = to_object("aventail", bm, [ctx.material(C_MAIL, MAIL)])
    head_z = base_z
    low_z = shoulder_z

    def weigh(p):
        t = smoothstep(head_z - 0.02, low_z, p.z)
        return {"Head": 1 - t, "Neck": t * 0.4, "Chest": t * 0.6}

    bind_by(obj, weigh)
    return obj


def kettle_hat(ctx):
    """Chapel de fer: round skull and a wide sloping brim."""
    centre, half, mn, mx = _head_box(ctx)
    bm = bmesh.new()
    n = ctx.seg(16, 8, 6)
    rows = ctx.seg(5, 3, 2)
    base_z = mn.z + (mx.z - mn.z) * 0.62
    rx, ry = half.x * 1.1, half.y * 1.08
    rings = []
    for r in range(rows):
        ang = r / rows * math.pi / 2
        z = base_z + math.sin(ang) * (mx.z - base_z + 0.03)
        rings.append(
            ring(
                bm,
                Vector((centre.x, centre.y, z)),
                rx * math.cos(ang),
                ry * math.cos(ang),
                n,
            )
        )
    apex = bm.verts.new(Vector((centre.x, centre.y, mx.z + 0.03)))
    for a, b in zip(rings, rings[1:], strict=False):
        bridge(bm, a, b, 0)
    for i in range(n):
        bm.faces.new((rings[-1][i], rings[-1][(i + 1) % n], apex))
    brim = ring(
        bm, Vector((centre.x, centre.y, base_z - 0.05)), rx + 0.085, ry + 0.085, n
    )
    bridge(bm, brim, rings[0], 0)
    finish(bm)
    obj = to_object("kettle_hat", bm, [ctx.material(C_PLATE, (0.40, 0.40, 0.41))])
    bind_rigid(obj, "Head")
    return [obj]


def great_helm(ctx):
    """Flat-topped great helm with an eye slit (knights)."""
    centre, half, mn, mx = _head_box(ctx)
    bm = bmesh.new()
    n = ctx.seg(14, 8, 6)
    bottom = mn.z - 0.02
    top = mx.z + 0.03
    rx, ry = half.x * 1.15, half.y * 1.1
    levels = [
        bottom,
        bottom + (top - bottom) * 0.55,
        bottom + (top - bottom) * 0.62,
        top,
    ]
    rings = [
        ring(
            bm,
            Vector((centre.x, centre.y, z)),
            rx * (0.92 if z == top else 1.0),
            ry * (0.92 if z == top else 1.0),
            n,
        )
        for z in levels
    ]
    for k, (a, b) in enumerate(zip(rings, rings[1:], strict=False)):
        bridge(bm, a, b, 1 if k == 1 else 0)
    cap(bm, rings[-1], 0, flip=False)
    finish(bm)
    obj = to_object(
        "great_helm",
        bm,
        [ctx.material(C_PLATE, STEEL), ctx.material(C_EXACT, (0.01, 0.01, 0.01))],
    )
    bind_rigid(obj, "Head")
    return [obj]


def cloth_cap(ctx, colour=(0.25, 0.10, 0.05)):
    """Soft felt hat with a short brim (archers, peasants)."""
    centre, half, mn, mx = _head_box(ctx)
    bm = bmesh.new()
    n = ctx.seg(12, 7, 5)
    base_z = mn.z + (mx.z - mn.z) * 0.66
    rx, ry = half.x * 1.08, half.y * 1.06
    r0 = ring(bm, Vector((centre.x, centre.y, base_z)), rx, ry, n)
    r1 = ring(
        bm, Vector((centre.x, centre.y + 0.01, mx.z + 0.02)), rx * 0.85, ry * 0.8, n
    )
    tip = bm.verts.new(Vector((centre.x, centre.y + 0.04, mx.z + 0.08)))
    brim = ring(
        bm, Vector((centre.x, centre.y, base_z - 0.01)), rx + 0.05, ry + 0.05, n
    )
    bridge(bm, r0, r1, 0)
    bridge(bm, brim, r0, 0)
    for i in range(n):
        bm.faces.new((r1[i], r1[(i + 1) % n], tip))
    finish(bm)
    obj = to_object("cloth_cap", bm, [ctx.material(C_CLOTH, colour)])
    bind_rigid(obj, "Head")
    return [obj]


# --- Weapons ----------------------------------------------------------------------------


def sword(ctx, length=0.95):
    """Arming sword in the right fist."""
    c, along, up, out = grip(ctx, "R")
    bm = bmesh.new()
    n = ctx.seg(6, 4, 3)
    tube(bm, c - along * 0.07, c + along * 0.07, 0.016, 0.016, n, 1)  # grip
    tube(bm, c - along * 0.09, c - along * 0.07, 0.025, 0.025, n, 0)  # pommel
    blade_base = c + along * 0.1
    tip = c + along * length
    w = 0.025
    th = 0.005
    # Blade: flat diamond section, tapering to the tip.
    vb = [
        bm.verts.new(blade_base + up * w),
        bm.verts.new(blade_base + out * th),
        bm.verts.new(blade_base - up * w),
        bm.verts.new(blade_base - out * th),
    ]
    vt = bm.verts.new(tip)
    for i in range(4):
        bm.faces.new((vb[i], vb[(i + 1) % 4], vt))
    bm.faces.new(list(reversed(vb)))
    box(
        bm, c + along * 0.085, (0.012, 0.012, 0.012), (along, up * 7.0, out), 0
    )  # cross-guard
    finish(bm)
    obj = to_object(
        "sword",
        bm,
        [ctx.material(C_PLATE, (0.62, 0.63, 0.65)), ctx.material(C_LEATHER, LEATHER)],
    )
    bind_rigid(obj, "Wrist.R")
    return [obj]
