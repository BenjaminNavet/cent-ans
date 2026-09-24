"""Weapons, shields and clothing pieces of the skinned battle figures (lot V2).

Same conventions as `battle_skinned_equipment` (Blender world space at rest, builders return
bound mesh objects). Props held by both hands are bound to the virtual bone `Prop` and built
in its rest frame (`prop_frame`: origin in the right fist, Y towards the tip, Z up).
"""

import math

import bmesh
from battle_skinned_equipment import (
    C_ARMS,
    C_EXACT,
    C_LEATHER,
    C_LIVERY,
    C_PLATE,
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
    finish,
    grip,
    nock_rest,
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
    tube(bm, top, _at(fr, (0, length - below + 0.32, 0)), 0.022, 0.002, max(n - 1, 3), 1, caps=False)
    finish(bm)
    obj = to_object("pike", bm, [ctx.material(C_WOOD, WOOD_LIGHT), ctx.material(C_PLATE, (0.35, 0.35, 0.36))])
    bind_rigid(obj, bone)
    return [obj]


def spear(ctx, length=2.3):
    """Spear in the right fist (militia)."""
    return pike(ctx, length=length, below=0.9, bone="Wrist.R")


def bill(ctx):
    """Bill (hooked hedging blade on a staff) in the right fist: militia and peasants."""
    fr = prop_frame(ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    tube(bm, _at(fr, (0, -0.8, 0)), _at(fr, (0, 1.2, 0)), 0.018, 0.016, ctx.seg(5, 4, 3), 0)
    box(bm, _at(fr, (0, 1.38, 0.03)), (0.005, 0.2, 0.045), (x, y, z), 1)
    box(bm, _at(fr, (0, 1.5, 0.1)), (0.005, 0.07, 0.03), (x, y, z), 1)
    finish(bm)
    obj = to_object("bill", bm, [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, IRON)])
    bind_rigid(obj, "Wrist.R")
    return [obj]


def pitchfork(ctx):
    """Two-tined fork (peasants)."""
    fr = prop_frame(ctx)
    bm = bmesh.new()
    tube(bm, _at(fr, (0, -0.8, 0)), _at(fr, (0, 1.1, 0)), 0.017, 0.015, ctx.seg(5, 4, 3), 0)
    for sx in (-0.05, 0.05):
        tube(bm, _at(fr, (0, 1.1, 0)), _at(fr, (sx, 1.42, 0)), 0.008, 0.004, 3, 1, caps=False)
    finish(bm)
    obj = to_object("pitchfork", bm, [ctx.material(C_WOOD, WOOD_LIGHT), ctx.material(C_PLATE, IRON)])
    bind_rigid(obj, "Wrist.R")
    return [obj]


def crossbow(ctx):
    """Crossbow on the `Prop` bone: tiller, composite prod, spanned string, stirrup."""
    fr = prop_frame(ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    box(bm, _at(fr, (0, 0.13, -0.01)), (0.025, 0.39, 0.03), (x, y, z), 0)
    segs = ctx.seg(6, 3, 2)
    pts = [_at(fr, ((-1 + 2 * i / segs) * 0.36, 0.5 - 0.07 * (-1 + 2 * i / segs) ** 2, 0.0)) for i in range(segs + 1)]
    for a, b in zip(pts, pts[1:]):
        tube(bm, a, b, 0.012, 0.012, ctx.seg(4, 3, 3), 1, caps=False)
    if ctx.level < 2:
        nut = _at(fr, (0, 0.18, 0.02))
        tube(bm, pts[0], nut, 0.003, 0.003, 3, 2, caps=False)
        tube(bm, pts[-1], nut, 0.003, 0.003, 3, 2, caps=False)
        box(bm, _at(fr, (0, 0.6, 0)), (0.07, 0.01, 0.01), (x, y, z), 1)
    finish(bm)
    obj = to_object("crossbow", bm, [ctx.material(C_WOOD, WOOD), ctx.material(C_PLATE, IRON), ctx.material(C_EXACT, STRING)])
    bind_rigid(obj, "Prop")
    return [obj]


def longbow(ctx, height=1.8):
    """Yew longbow in the left fist, its string (ends on the bow, middle on `Nock`) and the
    nocked arrow (`Arrow`, shown while drawing)."""
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
    for k, (a, b) in enumerate(zip(pts, pts[1:])):
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
        arrow = to_object("arrow", bm, [ctx.material(C_WOOD, (0.5, 0.4, 0.25)), ctx.material(C_PLATE, IRON), ctx.material(C_EXACT, WHITE)])
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
    obj = to_object("quiver", bm, [ctx.material(C_LEATHER, LEATHER), ctx.material(C_EXACT, WHITE)])
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
        outline.append((width / 2 * math.cos(u * math.pi / 2) ** 0.8, -height * 0.35 + height * u))
    pts = [(-w, v) for w, v in outline] + [(w, v) for w, v in reversed(outline)]
    pts = [p for i, p in enumerate(pts) if i == 0 or abs(p[0] - pts[i - 1][0]) + abs(p[1] - pts[i - 1][1]) > 1e-5]
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
        bm.faces.new((front[i], back[i], back[(i + 1) % n], front[(i + 1) % n])).material_index = 1
    bmesh.ops.triangulate(bm, faces=bm.faces)
    _uv_panel(bm, centre, side, -fore, width, height, 0.35)
    finish(bm)
    obj = to_object("shield", bm, [ctx.material(C_ARMS, (1, 1, 1)), ctx.material(C_WOOD, WOOD)])
    bind_rigid(obj, "LowerArm.L")
    return [obj]


def pavise(ctx):
    """Genoese pavise slung on the back, painted with the side's arms."""
    chest = ctx.head("Chest")
    bm = bmesh.new()
    w, h = 0.34, 0.6
    c = chest + Vector((0, 0.22, -0.15))
    corners = [c + Vector((-w, 0, -h)), c + Vector((w, 0, -h)), c + Vector((w * 0.9, 0, h)), c + Vector((-w * 0.9, 0, h))]
    front = [bm.verts.new(p + Vector((0, 0.03, 0))) for p in corners]
    back = [bm.verts.new(p) for p in corners]
    bm.faces.new(front).material_index = 0
    bm.faces.new(list(reversed(back))).material_index = 1
    for i in range(4):
        bm.faces.new((front[i], back[i], back[(i + 1) % 4], front[(i + 1) % 4])).material_index = 1
    _uv_panel(bm, c, Vector((-1, 0, 0)), Vector((0, 0, -1)), 2 * w, 2 * h)
    finish(bm)
    obj = to_object("pavise", bm, [ctx.material(C_ARMS, (1, 1, 1)), ctx.material(C_WOOD, WOOD)])
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
        for a, b in zip(grid, grid[1:]):
            for k in range(2):
                quad = (a[k], a[k + 1], b[k + 1], b[k]) if side < 0 else (a[k + 1], a[k], b[k], b[k + 1])
                bm.faces.new(quad)
    finish(bm)
    obj = to_object("tabard", bm, [ctx.material(C_LIVERY, colour)])
    thigh_z = hips.z - 0.05

    def weigh(p):
        low = smoothstep(thigh_z, bottom_z, p.z)
        leg = "UpperLeg.L" if p.x > 0 else "UpperLeg.R"
        up = smoothstep(hips.z, chest.z, p.z)
        return {"Chest": up, "Hips": (1 - up) * (1 - low * 0.6), leg: (1 - up) * low * 0.6}

    bind_by(obj, weigh)
    return [obj]
