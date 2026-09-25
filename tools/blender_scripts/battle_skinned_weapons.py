"""Weapons, shields and clothing pieces of the skinned battle figures (lot V2).

Same conventions as `battle_skinned_equipment` (Blender world space at rest, builders return
bound mesh objects). Props held by both hands are bound to the virtual bone `Prop` and built
in its rest frame (`prop_frame`: origin in the right fist, Y towards the tip, Z up).
"""

import math

import bmesh
from battle_skinned_equipment import (
    C_ARMS,
    C_CLOTH,
    C_EXACT,
    C_LEATHER,
    C_LIVERY,
    C_PLATE,
    C_QUILT,
    C_TRIM,
    C_WOOD,
    IRON,
    LEATHER,
    STRING,
    WHITE,
    WOOD,
    WOOD_LIGHT,
    bind_by,
    bind_rigid,
    box,
    bridge,
    finish,
    grip,
    nock_rest,
    ring,
    smoothstep,
    to_object,
    tube,
)
from mathutils import Vector


def prop_frame(ctx):
    """Rest frame of the right-hand prop (virtual bone `Prop`): origin, x, y (tip), z."""
    c, along, up, _out = grip(ctx, "R")
    y = along.normalized()
    x = y.cross(up).normalized()
    z = x.cross(y).normalized()
    return c, x, y, z


def _at(frame, local):
    o, x, y, z = frame
    return o + x * local[0] + y * local[1] + z * local[2]


def pike(ctx, length=4.6, below=1.25, bone="Prop"):
    """Pike: ash shaft and a narrow iron head, held by the `Prop` bone (both hands)."""
    fr = prop_frame(ctx)
    bm = bmesh.new()
    n = ctx.seg(5, 4, 3)
    top = _at(fr, (0, length - below, 0))
    tube(bm, _at(fr, (0, -below, 0)), top, 0.02, 0.016, n, 0)
    tube(
        bm,
        top,
        _at(fr, (0, length - below + 0.32, 0)),
        0.022,
        0.002,
        max(n - 1, 3),
        1,
        caps=False,
    )
    finish(bm)
    obj = to_object(
        "pike",
        bm,
        [ctx.material(C_WOOD, WOOD_LIGHT), ctx.material(C_PLATE, (0.35, 0.35, 0.36))],
    )
    bind_rigid(obj, bone)
    return [obj]


def spear(ctx, length=2.3):
    """Spear held in both hands like a short pike (militia, lot BV2)."""
    return pike(ctx, length=length, below=0.9, bone="Prop")


def bill(ctx):
    """Bill (hooked hedging blade on a staff) in both hands: militia and peasants (BV2)."""
    fr = prop_frame(ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    tube(
        bm,
        _at(fr, (0, -0.8, 0)),
        _at(fr, (0, 1.2, 0)),
        0.018,
        0.016,
        ctx.seg(5, 4, 3),
        0,
    )
    box(bm, _at(fr, (0, 1.38, 0.03)), (0.005, 0.2, 0.045), (x, y, z), 1)
    box(bm, _at(fr, (0, 1.5, 0.1)), (0.005, 0.07, 0.03), (x, y, z), 1)
    finish(bm)
    obj = to_object(
        "bill", bm, [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, IRON)]
    )
    bind_rigid(obj, "Prop")
    return [obj]


def pitchfork(ctx):
    """Two-tined fork (peasants)."""
    fr = prop_frame(ctx)
    bm = bmesh.new()
    tube(
        bm,
        _at(fr, (0, -0.8, 0)),
        _at(fr, (0, 1.1, 0)),
        0.017,
        0.015,
        ctx.seg(5, 4, 3),
        0,
    )
    for sx in (-0.05, 0.05):
        tube(
            bm,
            _at(fr, (0, 1.1, 0)),
            _at(fr, (sx, 1.42, 0)),
            0.008,
            0.004,
            3,
            1,
            caps=False,
        )
    finish(bm)
    obj = to_object(
        "pitchfork", bm, [ctx.material(C_WOOD, WOOD_LIGHT), ctx.material(C_PLATE, IRON)]
    )
    bind_rigid(obj, "Prop")
    return [obj]


def crossbow(ctx):
    """Crossbow on the `Prop` bone: tiller, composite prod, spanned string, stirrup."""
    fr = prop_frame(ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    box(bm, _at(fr, (0, 0.13, -0.01)), (0.025, 0.39, 0.03), (x, y, z), 0)
    segs = ctx.seg(6, 3, 2)
    pts = [
        _at(
            fr, ((-1 + 2 * i / segs) * 0.36, 0.5 - 0.07 * (-1 + 2 * i / segs) ** 2, 0.0)
        )
        for i in range(segs + 1)
    ]
    for a, b in zip(pts, pts[1:], strict=False):
        tube(bm, a, b, 0.012, 0.012, ctx.seg(4, 3, 3), 1, caps=False)
    if ctx.level < 2:
        nut = _at(fr, (0, 0.18, 0.02))
        tube(bm, pts[0], nut, 0.003, 0.003, 3, 2, caps=False)
        tube(bm, pts[-1], nut, 0.003, 0.003, 3, 2, caps=False)
        box(bm, _at(fr, (0, 0.6, 0)), (0.07, 0.01, 0.01), (x, y, z), 1)
    finish(bm)
    obj = to_object(
        "crossbow",
        bm,
        [
            ctx.material(C_WOOD, WOOD),
            ctx.material(C_PLATE, IRON),
            ctx.material(C_EXACT, STRING),
        ],
    )
    bind_rigid(obj, "Prop")
    return [obj]


def longbow(ctx, height=1.8):
    """Yew longbow in the left fist, with its string and nocked arrow.

    The string's ends sit on the bow, its middle on `Nock`; the nocked arrow (`Arrow`) is
    shown while drawing.
    """
    # Rest pose: arms hanging. The arrow lies along the forearm (it points at the target
    # once the arm is raised), the limbs across it, front to back.
    c, along, _up, out = grip(ctx, "L")
    vert = along.normalized()
    half = height / 2
    brace = 0.16
    segs = ctx.seg(8, 4, 2)
    bm = bmesh.new()
    pts = []
    for i in range(segs + 1):
        u = -1 + 2 * i / segs
        # Limbs bend back towards the archer (string side) at the tips.
        pts.append(c + vert * (u * half) - out * (brace * u * u))
    for k, (a, b) in enumerate(zip(pts, pts[1:], strict=False)):
        u = abs(-1 + 2 * (k + 0.5) / segs)
        r = 0.018 * (1 - 0.55 * u)
        tube(bm, a, b, r, r * 0.9, ctx.seg(5, 3, 3), 0, caps=False)
    finish(bm)
    bow = to_object("longbow", bm, [ctx.material(C_WOOD, (0.36, 0.2, 0.08))])
    bind_rigid(bow, "Wrist.L")
    objs = [bow]
    if ctx.level < 2:
        nock = nock_rest(ctx)
        bm = bmesh.new()
        tube(bm, pts[0], nock, 0.003, 0.003, 3, 0, caps=False)
        tube(bm, nock, pts[-1], 0.003, 0.003, 3, 0, caps=False)
        finish(bm)
        string = to_object("bowstring", bm, [ctx.material(C_EXACT, STRING)])

        def weigh(p):
            w = smoothstep(0.0, 1.0, min((p - nock).length / half, 1.0))
            return {"Nock": 1 - w, "Wrist.L": w}

        bind_by(string, weigh)
        objs.append(string)
        bm = bmesh.new()
        tip = nock + out * 0.8
        tube(bm, nock, tip, 0.005, 0.005, 3, 0, caps=False)
        tube(bm, tip, tip + out * 0.06, 0.009, 0.001, 3, 1, caps=False)
        if ctx.level == 0:
            for s in (1, -1):
                quad = [
                    bm.verts.new(nock + out * 0.03),
                    bm.verts.new(nock + out * 0.17),
                    bm.verts.new(nock + out * 0.15 + vert * s * 0.018),
                    bm.verts.new(nock + out * 0.05 + vert * s * 0.018),
                ]
                bm.faces.new(quad).material_index = 2
        finish(bm)
        arrow = to_object(
            "arrow",
            bm,
            [
                ctx.material(C_WOOD, (0.5, 0.4, 0.25)),
                ctx.material(C_PLATE, IRON),
                ctx.material(C_EXACT, WHITE),
            ],
        )
        bind_rigid(arrow, "Arrow")
        objs.append(arrow)
    return objs


def quiver(ctx, arrows=True):
    """Arrow bag or bolt case hanging at the right hip."""
    hips = ctx.head("Hips")
    bm = bmesh.new()
    base = hips + Vector((-0.2, 0.07, -0.25))
    top = hips + Vector((-0.22, 0.12, 0.08))
    tube(bm, base, top, 0.05, 0.06, ctx.seg(6, 4, 3), 0)
    if arrows and ctx.level < 2:
        for k in range(ctx.seg(5, 2, 0)):
            a = k / 5.0 * math.tau
            p = top + Vector((math.cos(a) * 0.03, math.sin(a) * 0.03, 0))
            tube(bm, p, p + Vector((0, 0.01, 0.12)), 0.008, 0.012, 3, 1, caps=False)
    finish(bm)
    obj = to_object(
        "quiver", bm, [ctx.material(C_LEATHER, LEATHER), ctx.material(C_EXACT, WHITE)]
    )
    bind_by(obj, lambda p: {"Hips": 0.6, "UpperLeg.R": 0.4})
    return [obj]


def _uv_panel(bm, centre, u_axis, v_axis, width, height, v0=0.5):
    uv = bm.loops.layers.uv.new("UVMap")
    for face in bm.faces:
        for loop in face.loops:
            rel = loop.vert.co - centre
            loop[uv].uv = (0.5 + rel.dot(u_axis) / width, v0 + rel.dot(v_axis) / height)


def heater_shield(ctx, width=0.5, height=0.62):
    """Heater shield on the left forearm, painted with the side's arms (UV = heraldry)."""
    lower = ctx.head("LowerArm.L")
    wrist = ctx.head("Wrist.L")
    fore = (wrist - lower).normalized()
    centre = lower.lerp(wrist, 0.45) + Vector((0, 0, 0.08))
    normal = Vector((0, 0, 1))
    side = normal.cross(fore).normalized()
    bm = bmesh.new()
    steps = ctx.seg(6, 3, 2)
    outline = []
    for i in range(steps + 1):
        u = i / steps
        outline.append(
            (width / 2 * math.cos(u * math.pi / 2) ** 0.8, -height * 0.35 + height * u)
        )
    pts = [(-w, v) for w, v in outline] + [(w, v) for w, v in reversed(outline)]
    pts = [
        p
        for i, p in enumerate(pts)
        if i == 0 or abs(p[0] - pts[i - 1][0]) + abs(p[1] - pts[i - 1][1]) > 1e-5
    ]
    front, back = [], []
    for sx, v in pts:
        p = centre + side * sx + fore * v
        bulge = 0.03 * (1 - (2 * sx / width) ** 2)
        front.append(bm.verts.new(p + normal * (0.012 + bulge)))
        back.append(bm.verts.new(p + normal * bulge))
    bm.faces.new(front).material_index = 0
    bm.faces.new(list(reversed(back))).material_index = 1
    n = len(front)
    for i in range(n):
        bm.faces.new(
            (front[i], back[i], back[(i + 1) % n], front[(i + 1) % n])
        ).material_index = 1
    bmesh.ops.triangulate(bm, faces=bm.faces)
    _uv_panel(bm, centre, side, -fore, width, height, 0.35)
    finish(bm)
    obj = to_object(
        "shield", bm, [ctx.material(C_ARMS, (1, 1, 1)), ctx.material(C_WOOD, WOOD)]
    )
    bind_rigid(obj, "LowerArm.L")
    return [obj]


def pavise(ctx):
    """Genoese pavise slung on the back, painted with the side's arms."""
    chest = ctx.head("Chest")
    bm = bmesh.new()
    w, h = 0.34, 0.6
    c = chest + Vector((0, 0.22, -0.15))
    corners = [
        c + Vector((-w, 0, -h)),
        c + Vector((w, 0, -h)),
        c + Vector((w * 0.9, 0, h)),
        c + Vector((-w * 0.9, 0, h)),
    ]
    front = [bm.verts.new(p + Vector((0, 0.03, 0))) for p in corners]
    back = [bm.verts.new(p) for p in corners]
    bm.faces.new(front).material_index = 0
    bm.faces.new(list(reversed(back))).material_index = 1
    for i in range(4):
        bm.faces.new(
            (front[i], back[i], back[(i + 1) % 4], front[(i + 1) % 4])
        ).material_index = 1
    _uv_panel(bm, c, Vector((-1, 0, 0)), Vector((0, 0, -1)), 2 * w, 2 * h)
    finish(bm)
    obj = to_object(
        "pavise", bm, [ctx.material(C_ARMS, (1, 1, 1)), ctx.material(C_WOOD, WOOD)]
    )
    bind_by(obj, lambda p: {"Chest": 0.7, "Torso": 0.3})
    return [obj]


def tabard(ctx, length=0.34, colour=(1.0, 1.0, 1.0)):
    """Livery tabard: front and back panels from the shoulders to mid-thigh."""
    chest = ctx.head("Chest")
    hips = ctx.head("Hips")
    mn, mx = ctx.part_bbox("Chest")
    bm = bmesh.new()
    top_z = chest.z + 0.12
    bottom_z = hips.z - length
    rows = ctx.seg(4, 2, 1)
    front_y = mn.y - 0.012
    back_y = mx.y + 0.012
    for side, y0 in ((-1, front_y), (1, back_y)):
        grid = []
        for r in range(rows + 1):
            t = r / rows
            z = top_z + (bottom_z - top_z) * t
            half = 0.16 + 0.05 * t
            yy = y0 + side * 0.03 * t
            grid.append([bm.verts.new(Vector((x, yy, z))) for x in (-half, 0.0, half)])
        for a, b in zip(grid, grid[1:], strict=False):
            for k in range(2):
                quad = (
                    (a[k], a[k + 1], b[k + 1], b[k])
                    if side < 0
                    else (a[k + 1], a[k], b[k], b[k + 1])
                )
                bm.faces.new(quad)
    finish(bm)
    obj = to_object("tabard", bm, [ctx.material(C_LIVERY, colour)])
    thigh_z = hips.z - 0.05

    def weigh(p):
        low = smoothstep(thigh_z, bottom_z, p.z)
        leg = "UpperLeg.L" if p.x > 0 else "UpperLeg.R"
        up = smoothstep(hips.z, chest.z, p.z)
        return {
            "Chest": up,
            "Hips": (1 - up) * (1 - low * 0.6),
            leg: (1 - up) * low * 0.6,
        }

    bind_by(obj, weigh)
    return [obj]


# --- Lot UR1: 15th-century kit, regional weapons ------------------------------------------


TORSO_GROUPS = {
    "Hips", "Abdomen", "Torso", "Chest", "Body", "Neck",
    "Shoulder.L", "Shoulder.R", "UpperLeg.L", "UpperLeg.R",
}


def _body_points(ctx, groups):
    """World positions of the body-part vertices dominated by one of `groups` (rest)."""
    out = []
    for obj in ctx.arm.children:
        if obj.type != "MESH":
            continue
        names = {g.index: g.name for g in obj.vertex_groups}
        mw = obj.matrix_world
        for v in obj.data.vertices:
            best = max(v.groups, key=lambda g: g.weight, default=None)
            if best is not None and best.weight > 0.5 and names.get(best.group) in groups:
                out.append(mw @ v.co)
    return out


def _torso_shell(ctx, name, material, skirt, flare, margin=0.018):
    """Closed garment around the torso, from the shoulders to `skirt` below the hips.

    Rings follow the torso's rest bounding boxes; the skirt flares by `flare` and is
    weighted to the thighs like the tabard.
    """
    chest = ctx.head("Chest")
    hips = ctx.head("Hips")
    body = _body_points(ctx, TORSO_GROUPS)
    top_z = chest.z + 0.1
    bottom_z = hips.z - skirt
    n = ctx.seg(14, 8, 6)
    rows = ctx.seg(6, 3, 2)
    bm = bmesh.new()
    rings = []
    mid_z = (chest.z + hips.z) / 2
    mid = [p.x for p in body if abs(p.z - mid_z) < 0.05] or [p.x for p in body]
    # No flaring collar: shoulders (wide on some bodies) are capped to the waist's width.
    max_rx = (max(mid) - min(mid)) / 2 * 1.15 + margin
    for r in range(rows + 1):
        t = r / rows
        z = top_z + (bottom_z - top_z) * t
        near = [p for p in body if abs(p.z - z) < 0.05] or body
        xs = [p.x for p in near]
        ys = [p.y for p in near]
        below = smoothstep(hips.z, bottom_z, z)
        cx = (min(xs) + max(xs)) / 2
        cy = (min(ys) + max(ys)) / 2
        rx = min((max(xs) - min(xs)) / 2 + margin, max_rx) + flare * below
        ry = (max(ys) - min(ys)) / 2 + margin + flare * 0.6 * below
        if r == 0:  # narrower at the neck opening
            rx *= 0.86
            ry *= 1.0
        rings.append(ring(bm, Vector((cx, cy, z)), rx, ry, n))
    for a, b in zip(rings, rings[1:], strict=False):
        bridge(bm, a, b, 0)
    finish(bm)
    ring_points = [[v.co.copy() for v in r] for r in rings]
    obj = to_object(name, bm, [material])
    thigh_z = hips.z - 0.05

    def weigh(p):
        low = smoothstep(thigh_z, bottom_z, p.z)
        leg = "UpperLeg.L" if p.x > 0 else "UpperLeg.R"
        up = smoothstep(hips.z, chest.z, p.z)
        return {
            "Chest": up * 0.7,
            "Torso": up * 0.3,
            "Hips": (1 - up) * (1 - low * 0.6),
            leg: (1 - up) * low * 0.6,
        }

    bind_by(obj, weigh)
    return obj, ring_points


def jack(ctx, colour=(0.55, 0.47, 0.32), skirt=0.3, livery=False):
    """Jaque: quilted jacket to mid-thigh (commoners of the 15th century).

    `livery=True` dyes it in the side's colours (francs-archers' hoquetons).
    """
    code = C_LIVERY if livery else C_QUILT
    obj, _rings = _torso_shell(ctx, "jack", ctx.material(code, colour), skirt, 0.06, margin=0.03)
    return [obj]


def brigandine(ctx, colour=(0.35, 0.05, 0.04), studs=True):
    """Brigandine: cloth-covered plates to the hips, rows of gilt rivets on the front."""
    obj, rings = _torso_shell(
        ctx, "brigandine", ctx.material(C_CLOTH, colour), 0.12, 0.03, margin=0.024
    )
    out = [obj]
    if studs and ctx.level == 0:
        bm = bmesh.new()
        centre_y = sum(p.y for p in rings[1]) / len(rings[1])
        for ring_pts in rings[1:-1]:
            for p in ring_pts:
                if p.y > centre_y - 0.03:
                    continue  # front only (-Y)
                box(bm, p + Vector((0, -0.008, 0)), (0.011, 0.006, 0.011))
        finish(bm)
        rivets = to_object("rivets", bm, [ctx.material(C_TRIM, (0.55, 0.42, 0.14))])
        chest = ctx.head("Chest")
        hips = ctx.head("Hips")
        bind_by(
            rivets,
            lambda p: {
                "Chest": smoothstep(hips.z, chest.z, p.z),
                "Hips": 1 - smoothstep(hips.z, chest.z, p.z),
            },
        )
        out.append(rivets)
    return out


def round_shield(ctx, radius=0.2, back=False, arms=True, boss=True):
    """Round shield: buckler or rondache on the left forearm, or targe slung on the back."""
    if back:
        chest = ctx.head("Chest")
        centre = chest + Vector((0.02, 0.15, -0.1))
        normal = Vector((0, 1, 0))
        side = Vector((-1, 0, 0))
        up = Vector((0, 0, 1))
    else:
        lower = ctx.head("LowerArm.L")
        wrist = ctx.head("Wrist.L")
        up = (wrist - lower).normalized()
        centre = lower.lerp(wrist, 0.5) + Vector((0, 0, 0.07))
        normal = Vector((0, 0, 1))
        side = normal.cross(up).normalized()
    n = ctx.seg(14, 8, 6)
    bm = bmesh.new()
    front, rear = [], []
    for i in range(n):
        a = 2 * math.pi * i / n
        p = centre + side * math.cos(a) * radius + up * math.sin(a) * radius
        front.append(bm.verts.new(p + normal * 0.012))
        rear.append(bm.verts.new(p))
    mid = bm.verts.new(centre + normal * (0.045 if boss else 0.02))
    for i in range(n):
        bm.faces.new((front[i], front[(i + 1) % n], mid)).material_index = 0
    bm.faces.new(list(reversed(rear))).material_index = 1
    for i in range(n):
        bm.faces.new(
            (front[i], rear[i], rear[(i + 1) % n], front[(i + 1) % n])
        ).material_index = 1
    bmesh.ops.triangulate(bm, faces=bm.faces)
    _uv_panel(bm, centre, side, -up, 2 * radius, 2 * radius)
    finish(bm)
    face = (
        ctx.material(C_ARMS, (1, 1, 1)) if arms else ctx.material(C_WOOD, WOOD_LIGHT)
    )
    obj = to_object("round_shield", bm, [face, ctx.material(C_LEATHER, LEATHER)])
    if back:
        bind_by(obj, lambda p: {"Chest": 0.7, "Torso": 0.3})
    else:
        bind_rigid(obj, "LowerArm.L")
    return [obj]


def goedendag(ctx):
    """Goedendag: 1.5 m club swelling to an iron-banded head with a spike, both hands."""
    fr = prop_frame(ctx)
    bm = bmesh.new()
    n = ctx.seg(7, 5, 3)
    tube(bm, _at(fr, (0, -0.55, 0)), _at(fr, (0, 0.55, 0)), 0.02, 0.03, n, 0)
    tube(bm, _at(fr, (0, 0.55, 0)), _at(fr, (0, 0.72, 0)), 0.03, 0.048, n, 0)
    tube(bm, _at(fr, (0, 0.72, 0)), _at(fr, (0, 0.9, 0)), 0.048, 0.042, n, 0)
    if ctx.level < 2:
        for y in (0.74, 0.87):
            tube(bm, _at(fr, (0, y, 0)), _at(fr, (0, y + 0.025, 0)), 0.053, 0.053, n, 1)
    tube(
        bm,
        _at(fr, (0, 0.9, 0)),
        _at(fr, (0, 1.08, 0)),
        0.014,
        0.001,
        max(n - 2, 3),
        1,
        caps=False,
    )
    finish(bm)
    obj = to_object(
        "goedendag",
        bm,
        [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, (0.30, 0.30, 0.31))],
    )
    bind_rigid(obj, "Prop")
    return [obj]


def _flat_blade(bm, base, tip, width, thickness, x, mat):
    """Flat diamond blade from `base` to `tip`, `width` along `x`."""
    along = (tip - base).normalized()
    z = along.cross(x).normalized()
    ring_verts = [
        bm.verts.new(base + x * width),
        bm.verts.new(base + z * thickness),
        bm.verts.new(base - x * width),
        bm.verts.new(base - z * thickness),
    ]
    t = bm.verts.new(tip)
    for i in range(4):
        bm.faces.new((ring_verts[i], ring_verts[(i + 1) % 4], t)).material_index = mat
    bm.faces.new(list(reversed(ring_verts))).material_index = mat


def coustille(ctx):
    """Coustille: long narrow knife-blade on a short haft (coutiliers), both hands."""
    fr = prop_frame(ctx)
    _o, _x, _y, z = fr
    bm = bmesh.new()
    tube(
        bm,
        _at(fr, (0, -0.6, 0)),
        _at(fr, (0, 0.95, 0)),
        0.018,
        0.016,
        ctx.seg(5, 4, 3),
        0,
    )
    tube(bm, _at(fr, (0, 0.93, 0)), _at(fr, (0, 1.0, 0)), 0.024, 0.022, 4, 1)
    _flat_blade(bm, _at(fr, (0, 1.0, 0)), _at(fr, (0, 1.55, 0)), 0.035, 0.008, z, 1)
    finish(bm)
    obj = to_object(
        "coustille",
        bm,
        [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, (0.62, 0.63, 0.65))],
    )
    bind_rigid(obj, "Prop")
    return [obj]


def pollaxe(ctx):
    """Pollaxe (hache d'armes): axe blade, hammer back and top spike on a 1.7 m haft."""
    fr = prop_frame(ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    tube(
        bm,
        _at(fr, (0, -0.55, 0)),
        _at(fr, (0, 1.12, 0)),
        0.02,
        0.018,
        ctx.seg(5, 4, 3),
        0,
    )
    box(bm, _at(fr, (0, 1.05, 0)), (0.022, 0.1, 0.022), (x, y, z), 1)  # langets
    box(bm, _at(fr, (0, 1.1, 0.09)), (0.006, 0.07, 0.08), (x, y, z), 1)  # blade
    box(bm, _at(fr, (0, 1.1, -0.05)), (0.02, 0.02, 0.04), (x, y, z), 1)  # hammer
    tube(
        bm, _at(fr, (0, 1.13, 0)), _at(fr, (0, 1.36, 0)), 0.014, 0.001, 4, 1, caps=False
    )
    finish(bm)
    obj = to_object(
        "pollaxe",
        bm,
        [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, (0.55, 0.56, 0.58))],
    )
    bind_rigid(obj, "Prop")
    return [obj]


def hand_culverin(ctx):
    """Hand culverin: iron tube on a wooden stock, held like a crossbow (both hands)."""
    fr = prop_frame(ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    n = ctx.seg(8, 5, 4)
    box(bm, _at(fr, (0, -0.1, -0.015)), (0.022, 0.28, 0.03), (x, y, z), 0)
    tube(bm, _at(fr, (0, 0.12, 0.01)), _at(fr, (0, 0.72, 0.01)), 0.03, 0.026, n, 1)
    if ctx.level < 2:
        tube(bm, _at(fr, (0, 0.68, 0.01)), _at(fr, (0, 0.73, 0.01)), 0.036, 0.036, n, 1)
        tube(bm, _at(fr, (0, 0.14, 0.01)), _at(fr, (0, 0.2, 0.01)), 0.036, 0.036, n, 1)
    finish(bm)
    obj = to_object(
        "hand_culverin",
        bm,
        [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, (0.20, 0.20, 0.21))],
    )
    bind_rigid(obj, "Prop")
    return [obj]


def powder_flask(ctx):
    """Powder horn and shot bag at the left hip (culveriners)."""
    hips = ctx.head("Hips")
    bm = bmesh.new()
    a = hips + Vector((0.2, -0.02, -0.08))
    b = hips + Vector((0.22, -0.06, -0.3))
    tube(bm, a, b, 0.04, 0.012, ctx.seg(6, 4, 3), 0)
    box(bm, hips + Vector((0.2, 0.08, -0.2)), (0.02, 0.05, 0.06), mat=1)
    finish(bm)
    obj = to_object(
        "powder_flask",
        bm,
        [ctx.material(C_EXACT, (0.55, 0.47, 0.33)), ctx.material(C_LEATHER, LEATHER)],
    )
    bind_by(obj, lambda p: {"Hips": 0.6, "UpperLeg.L": 0.4})
    return [obj]


def javelin(ctx, length=1.7):
    """Javelin (azagaya) held in the right hand like a short lance (jinetes)."""
    fr = prop_frame(ctx)
    bm = bmesh.new()
    n = ctx.seg(5, 4, 3)
    tube(bm, _at(fr, (0, -0.5, 0)), _at(fr, (0, length - 0.5, 0)), 0.014, 0.012, n, 0)
    tube(
        bm,
        _at(fr, (0, length - 0.5, 0)),
        _at(fr, (0, length - 0.25, 0)),
        0.022,
        0.001,
        max(n - 1, 3),
        1,
        caps=False,
    )
    finish(bm)
    obj = to_object(
        "javelin",
        bm,
        [ctx.material(C_WOOD, WOOD_LIGHT), ctx.material(C_PLATE, (0.35, 0.35, 0.36))],
    )
    bind_rigid(obj, "Prop")
    return [obj]


def adarga(ctx, width=0.5, height=0.62):
    """Adarga: heart-shaped leather shield (two lobes up) on the left forearm (jinetes)."""
    lower = ctx.head("LowerArm.L")
    wrist = ctx.head("Wrist.L")
    fore = (wrist - lower).normalized()
    centre = lower.lerp(wrist, 0.45) + Vector((0, 0, 0.08))
    normal = Vector((0, 0, 1))
    side = normal.cross(fore).normalized()
    steps = ctx.seg(20, 12, 8)
    pts = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        hx = math.sin(t) ** 3
        hy = (
            13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t)
        ) / 17
        pts.append((hx * width / 2, hy * height / 2))
    bm = bmesh.new()
    front, back = [], []
    for sx, v in pts:
        p = centre + side * sx + fore * v
        bulge = 0.025 * (1 - (2 * sx / width) ** 2)
        front.append(bm.verts.new(p + normal * (0.01 + bulge)))
        back.append(bm.verts.new(p + normal * bulge))
    mid = bm.verts.new(centre + normal * 0.05)
    n = len(front)
    for i in range(n):
        bm.faces.new((front[i], front[(i + 1) % n], mid)).material_index = 0
    bm.faces.new(list(reversed(back))).material_index = 1
    for i in range(n):
        bm.faces.new(
            (front[i], back[i], back[(i + 1) % n], front[(i + 1) % n])
        ).material_index = 1
    bmesh.ops.triangulate(bm, faces=bm.faces)
    _uv_panel(bm, centre, side, -fore, width, height)
    finish(bm)
    obj = to_object(
        "adarga",
        bm,
        [ctx.material(C_ARMS, (1, 1, 1)), ctx.material(C_LEATHER, (0.42, 0.30, 0.16))],
    )
    bind_rigid(obj, "LowerArm.L")
    return [obj]
