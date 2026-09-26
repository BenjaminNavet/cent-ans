"""Lot FG2: fine weapons and shields of the battle figures (registered in ``GEAR``).

Same placement as the V2 builders they replace (``battle_skinned_weapons``,
``battle_skinned_equipment.sword``, ``battle_skinned_cavalry.lance``), so that the baked
poses (hands on the shaft, bow at the cheek, crossbow at the shoulder) still hold: the
two-handed weapons are built in the ``Prop`` frame (origin in the right fist, x across, y
towards the tip, z up), the shields on the left forearm or the back.

Shapes follow surviving pieces of 1340-1450: type XVI arming sword with wheel pommel,
langeted pike and pollaxe heads, French vouge, Flemish goedendag, crossbow with a steel prod
and stirrup, D-section yew longbow with horn nocks, heater, rondache, buckler, targe, adarga
and pavise with their dished boards and rims.
"""

import math

import battle_fine_equipment as fe
import battle_fine_proto as fp
import battle_skinned_equipment as eq
import battle_skinned_weapons as weapons
import bmesh
from battle_fine_gear import (
    BRASS,
    IRON,
    STEEL,
    finish_object,
    gear,
    rivet,
    sweep,
)
from mathutils import Vector

WOOD = eq.WOOD
WOOD_LIGHT = eq.WOOD_LIGHT
ASH = (0.40, 0.29, 0.17)
YEW = (0.42, 0.20, 0.07)
HORN = (0.55, 0.47, 0.33)
STRING = eq.STRING
FLETCH = (0.62, 0.60, 0.55)
BLADE = (0.62, 0.63, 0.65)
GRIP = (0.10, 0.06, 0.03)
HIDE = (0.42, 0.30, 0.16)

# Visible arrow: shaft length from the nock to the head's base. Real livery arrows measure
# ~0.76 m; the fine rig draws ~0.59 m (FG2: torso -75 degrees, bow shoulder pushed 20), so
# the nocked arrow is shortened to end a hand's breadth past the bow (``battle_fine_rig``).
ARROW_LENGTH = 0.62


def _at(fr, x, y, z=0.0):
    """Point of the prop frame `fr` at local (x, y, z)."""
    return weapons._at(fr, (x, y, z))


def shaft(g, bm, fr, y0, y1, r0, r1, mat=0):
    """Tapered wooden haft along the prop axis from y0 to y1 (octagonal at LOD0)."""
    eq.tube(bm, _at(fr, 0, y0), _at(fr, 0, y1), r0, r1, g.seg(8, 5, 3), mat)


def blade(
    bm,
    base,
    along,
    side,
    length,
    width,
    thick,
    steps,
    mat,
    taper=0.55,
    fuller=0.0,
    point=0.03,
    single=False,
    width_of=None,
):
    """Lofted blade from `base` along `along`, edges along `side` (flats face along x).

    Lens section with an optional fuller down the first two thirds; `single` gives a
    single-edged section (back along -side). `width_of(t)` overrides the edge profile.
    """
    out = along.cross(side).normalized()
    rings = []
    for s in range(steps + 1):
        t = s / steps
        p = base + along * length * t
        w = width_of(t) if width_of else width * (1 - t) ** taper + 0.001
        th = thick * (1 - 0.8 * t) + 0.0008
        fu = fuller if t < 0.66 else 0.0
        if single:
            pts = [
                p + side * w,
                p + side * w * 0.3 + out * th,
                p - side * w * 0.9 + out * th,
                p - side * w,
                p - side * w * 0.9 - out * th,
                p + side * w * 0.3 - out * th,
            ]
        else:
            pts = [
                p + side * w,
                p + side * w * 0.4 + out * th,
                p + out * (th - fu),
                p - side * w * 0.4 + out * th,
                p - side * w,
                p - side * w * 0.4 - out * th,
                p - out * (th - fu),
                p + side * w * 0.4 - out * th,
            ]
        rings.append([bm.verts.new(q) for q in pts])
    for a_, b_ in zip(rings, rings[1:], strict=False):
        eq.bridge(bm, a_, b_, mat)
    eq.cap(bm, rings[0], mat)
    tip = bm.verts.new(base + along * (length + point) + (side * 0.0))
    k = len(rings[-1])
    for i in range(k):
        bm.faces.new((rings[-1][i], rings[-1][(i + 1) % k], tip)).material_index = mat
    return rings


def flat_plate(bm, outline, centre_off, normal, thick, mat, rim_mat=None):
    """Flat piece (axe blade, langet) from a closed 3D `outline`, `thick` along `normal`."""
    n = normal.normalized()
    front = [bm.verts.new(p + n * thick) for p in outline]
    back = [bm.verts.new(p - n * thick) for p in outline]
    bm.faces.new(front).material_index = mat
    bm.faces.new(list(reversed(back))).material_index = mat
    m = len(outline)
    for i in range(m):
        j = (i + 1) % m
        f = bm.faces.new((front[j], front[i], back[i], back[j]))
        f.material_index = rim_mat if rim_mat is not None else mat


def langets(g, bm, fr, y0, y1, r, mat, rivets=True):
    """Two steel strips nailed down the haft below a head (x = +-r), with rivets."""
    _o, x, y, z = fr
    for sx in (-1, 1):
        c = _at(fr, sx * r, (y0 + y1) / 2)
        eq.box(bm, c, (0.002, (y1 - y0) / 2, 0.009), (x, y, z), mat)
        if rivets and g.level == 0:
            for yy in (y0 + 0.04, (y0 + y1) / 2, y1 - 0.04):
                rivet(bm, _at(fr, sx * (r + 0.002), yy), x * sx, 0.004, mat)


def _obj(name, bm, mats, bone="Prop"):
    eq.finish(bm)
    return finish_object(name, bm, mats, bone=bone)


# --- Swords -------------------------------------------------------------------------------


@gear("sword")
def sword(g, length=0.92):
    """Arming sword (type XVI): fullered blade, wheel pommel, cord-bound grip, long quillons."""
    ctx = g.ctx
    c, along, up, _out = eq.grip(ctx, "R")
    fist = fp.FIST.get("R")
    if fist is not None:
        c, along = fist
        fore = (ctx.head("Wrist.R") - ctx.head("LowerArm.R")).normalized()
        up = (fore - along * fore.dot(along)).normalized()
    bm = bmesh.new()
    sides = g.seg(10, 6, 4)
    # Grip: leather over cord, swelling in the middle.
    eq.tube(bm, c - along * 0.075, c, 0.0135, 0.0155, sides, 1)
    eq.tube(bm, c, c + along * 0.07, 0.0155, 0.0135, sides, 1)
    # Wheel pommel with a raised centre.
    eq.tube(bm, c - along * 0.105, c - along * 0.078, 0.028, 0.028, g.seg(16, 8, 4), 2)
    if g.level == 0:
        eq.tube(bm, c - along * 0.111, c - along * 0.105, 0.014, 0.014, 8, 2)
        eq.tube(bm, c - along * 0.078, c - along * 0.074, 0.012, 0.016, 8, 2)
    # Cross-guard: quillons tapering outwards, drooping slightly towards the blade.
    gd = c + along * 0.082
    path = [
        gd - up * 0.11 + along * 0.012,
        gd - up * 0.05,
        gd,
        gd + up * 0.05,
        gd + up * 0.11 + along * 0.012,
    ]
    sweep(bm, path, [0.006, 0.008, 0.011, 0.008, 0.006], g.seg(6, 4, 3), 0)
    # Blade.
    base = c + along * 0.095
    if g.level < 2:
        blade(
            bm,
            base,
            along,
            up,
            length - 0.095,
            0.026,
            0.0055,
            g.seg(8, 3, 2),
            0,
            fuller=0.0025 if g.level == 0 else 0.0,
        )
    else:
        blade(bm, base, along, up, length - 0.095, 0.022, 0.004, 1, 0, taper=0.3)
    return [
        _obj(
            "sword",
            bm,
            [
                g.mat(eq.C_PLATE, BLADE),
                g.mat(eq.C_LEATHER, GRIP),
                g.mat(eq.C_PLATE, STEEL),
            ],
            bone="Wrist.R",
        )
    ]


# --- Staff weapons ------------------------------------------------------------------------


def _staff_mats(g, wood=WOOD):
    return [g.mat(eq.C_WOOD, wood), g.mat(eq.C_PLATE, STEEL), g.mat(eq.C_PLATE, IRON)]


@gear("pike")
def pike(g, length=4.6, below=1.25):
    """Pike: tapering ash shaft, long square-sectioned head on langets, iron butt."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    top = length - below
    bm = bmesh.new()
    shaft(g, bm, fr, -below, top, 0.02, 0.015, 0)
    if g.level < 2:
        langets(g, bm, fr, top - 0.32, top, 0.016, 2, rivets=True)
        eq.tube(
            bm,
            _at(fr, 0, -below - 0.04),
            _at(fr, 0, -below + 0.05),
            0.019,
            0.022,
            g.seg(8, 5, 3),
            2,
        )
    # Head: socket, then a narrow bodkin of square section.
    eq.tube(
        bm,
        _at(fr, 0, top - 0.02),
        _at(fr, 0, top + 0.06),
        0.018,
        0.013,
        g.seg(8, 4, 3),
        1,
    )
    blade(
        bm,
        _at(fr, 0, top + 0.06),
        y,
        z,
        0.22,
        0.014,
        0.009,
        g.seg(4, 2, 1),
        1,
        taper=0.9,
        point=0.03,
    )
    return [_obj("pike", bm, _staff_mats(g, ASH))]


@gear("spear")
def spear(g, length=2.3):
    """Spear: ash shaft, leaf-shaped head with a midrib, socket with a collar."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    below = 0.9
    top = length - below
    bm = bmesh.new()
    shaft(g, bm, fr, -below, top, 0.019, 0.016, 0)
    eq.tube(
        bm,
        _at(fr, 0, top - 0.06),
        _at(fr, 0, top + 0.08),
        0.019,
        0.012,
        g.seg(8, 4, 3),
        1,
    )
    if g.level == 0:
        eq.tube(bm, _at(fr, 0, top - 0.06), _at(fr, 0, top - 0.045), 0.021, 0.021, 8, 2)
    blade(
        bm,
        _at(fr, 0, top + 0.08),
        y,
        z,
        0.24,
        0.03,
        0.006,
        g.seg(6, 3, 1),
        1,
        point=0.02,
        width_of=lambda t: 0.034 * math.sin(math.pi * (0.12 + 0.88 * t) ** 0.7) + 0.001,
    )
    return [_obj("spear", bm, _staff_mats(g, ASH))]


@gear("javelin")
def javelin(g, length=1.7):
    """Azagaya: light shaft, small leaf head, iron ferrule."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    top = length - 0.5
    bm = bmesh.new()
    shaft(g, bm, fr, -0.5, top, 0.014, 0.012, 0)
    eq.tube(
        bm,
        _at(fr, 0, top - 0.04),
        _at(fr, 0, top + 0.03),
        0.014,
        0.009,
        g.seg(6, 4, 3),
        1,
    )
    blade(
        bm,
        _at(fr, 0, top + 0.03),
        y,
        z,
        0.17,
        0.022,
        0.005,
        g.seg(5, 2, 1),
        1,
        width_of=lambda t: 0.024 * math.sin(math.pi * (0.1 + 0.9 * t) ** 0.8) + 0.001,
        point=0.015,
    )
    if g.level < 2:
        eq.tube(
            bm, _at(fr, 0, -0.55), _at(fr, 0, -0.48), 0.008, 0.014, g.seg(6, 4, 3), 2
        )
    return [_obj("javelin", bm, _staff_mats(g, WOOD_LIGHT))]


@gear("bill")
def vouge(g):
    """Vouge: broad cleaver blade with a point and a back hook, socketed on a 2 m staff."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    shaft(g, bm, fr, -0.8, 1.2, 0.018, 0.016, 0)
    # Socket (two rings).
    eq.tube(bm, _at(fr, 0, 1.1), _at(fr, 0, 1.24), 0.019, 0.017, g.seg(8, 4, 3), 2)
    if g.level == 0:
        for yy in (1.1, 1.22):
            eq.tube(bm, _at(fr, 0, yy), _at(fr, 0, yy + 0.015), 0.021, 0.021, 8, 2)
    # Blade outline in the (y, z) plane: edge forward (+z), point up, hook on the back.
    pts = [
        (1.2, 0.0),
        (1.24, 0.07),
        (1.35, 0.085),
        (1.5, 0.08),
        (1.62, 0.055),
        (1.7, 0.012),  # point
        (1.6, -0.005),
        (1.52, -0.012),
        (1.49, -0.055),  # back hook
        (1.46, -0.02),
        (1.3, -0.012),
    ]
    if g.level == 2:
        pts = [pts[0], pts[2], pts[5], pts[8], pts[10]]
    outline = [_at(fr, 0, yy, zz) for yy, zz in pts]
    flat_plate(bm, outline, None, x, 0.003, 1)
    return [_obj("bill", bm, _staff_mats(g))]


@gear("pitchfork")
def pitchfork(g):
    """Two-tined iron fork on an ash staff, with a ferrule."""
    fr = weapons.prop_frame(g.ctx)
    bm = bmesh.new()
    shaft(g, bm, fr, -0.8, 1.1, 0.017, 0.015, 0)
    eq.tube(bm, _at(fr, 0, 1.04), _at(fr, 0, 1.12), 0.018, 0.02, g.seg(6, 4, 3), 2)
    for sx in (-1, 1):
        path = [
            _at(fr, 0, 1.12),
            _at(fr, sx * 0.04, 1.16),
            _at(fr, sx * 0.055, 1.25),
            _at(fr, sx * 0.055, 1.44, 0.02),
        ]
        sweep(bm, path, [0.008, 0.007, 0.006, 0.002], g.seg(5, 3, 3), 2)
    return [_obj("pitchfork", bm, _staff_mats(g, WOOD_LIGHT))]


@gear("goedendag")
def goedendag(g):
    """Goedendag: club swelling to a banded head, square spike on an iron collar."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    n = g.seg(10, 6, 4)
    bm = bmesh.new()
    prof = [
        (-0.55, 0.02),
        (0.0, 0.022),
        (0.5, 0.03),
        (0.62, 0.043),
        (0.72, 0.05),
        (0.88, 0.046),
        (0.92, 0.036),
    ]
    for (y0, r0), (y1, r1) in zip(prof, prof[1:], strict=False):
        eq.tube(bm, _at(fr, 0, y0), _at(fr, 0, y1), r0, r1, n, 0)
    if g.level < 2:
        for yy in (0.66, 0.84):
            eq.tube(bm, _at(fr, 0, yy), _at(fr, 0, yy + 0.028), 0.054, 0.054, n, 2)
            if g.level == 0:
                for k in range(6):
                    a = 2 * math.pi * k / 6
                    d = x * math.cos(a) + z * math.sin(a)
                    rivet(bm, _at(fr, 0, yy + 0.014) + d * 0.054, d, 0.005, 1)
    eq.tube(bm, _at(fr, 0, 0.9), _at(fr, 0, 0.94), 0.022, 0.018, g.seg(6, 4, 3), 2)
    blade(
        bm,
        _at(fr, 0, 0.94),
        y,
        x,
        0.14,
        0.011,
        0.011,
        g.seg(3, 1, 1),
        1,
        taper=1.0,
        point=0.02,
    )
    return [_obj("goedendag", bm, _staff_mats(g))]


@gear("coustille")
def coustille(g):
    """Coustille: long single-edged knife blade on a short haft, rondel and ferrule."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    shaft(g, bm, fr, -0.6, 0.95, 0.018, 0.016, 0)
    eq.tube(bm, _at(fr, 0, 0.9), _at(fr, 0, 1.0), 0.02, 0.019, g.seg(8, 4, 3), 2)
    if g.level < 2:
        eq.tube(
            bm, _at(fr, 0, 0.955), _at(fr, 0, 0.965), 0.045, 0.045, g.seg(12, 6, 4), 1
        )
    blade(
        bm,
        _at(fr, 0, 1.0),
        y,
        z,
        0.52,
        0.03,
        0.006,
        g.seg(6, 2, 1),
        1,
        taper=0.35,
        single=True,
        point=0.04,
    )
    return [_obj("coustille", bm, _staff_mats(g))]


@gear("pollaxe")
def pollaxe(g):
    """Pollaxe: crescent axe blade, crowned hammer, top and butt spikes, langets, rondel."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    shaft(g, bm, fr, -0.55, 1.12, 0.02, 0.018, 0)
    if g.level < 2:
        langets(g, bm, fr, 0.72, 1.1, 0.018, 1)
        eq.tube(
            bm, _at(fr, 0, 0.62), _at(fr, 0, 0.63), 0.05, 0.05, g.seg(12, 6, 4), 1
        )  # rondel
        eq.tube(
            bm, _at(fr, 0, -0.62), _at(fr, 0, -0.55), 0.004, 0.02, g.seg(6, 4, 3), 1
        )
    # Head block.
    eq.box(bm, _at(fr, 0, 1.1), (0.022, 0.04, 0.022), (x, y, z), 1)
    # Axe blade (+z), crescent edge.
    edge = []
    for k in range(g.seg(7, 3, 2) + 1):
        t = k / g.seg(7, 3, 2)
        a = -0.9 + 1.8 * t
        edge.append((1.1 + 0.1 * math.sin(a), 0.105 + 0.02 * math.cos(a)))
    outline = [(1.08, 0.02)] + edge + [(1.13, 0.02)]
    flat_plate(bm, [_at(fr, 0, yy, zz) for yy, zz in outline], None, x, 0.003, 1)
    # Hammer (-z): short neck and a crowned face.
    eq.tube(
        bm,
        _at(fr, 0, 1.1, -0.02),
        _at(fr, 0, 1.1, -0.06),
        0.014,
        0.018,
        g.seg(6, 4, 3),
        1,
    )
    eq.box(bm, _at(fr, 0, 1.1, -0.068), (0.022, 0.022, 0.009), (x, y, z), 1)
    if g.level == 0:
        for sx in (-1, 1):
            for sy in (-1, 1):
                rivet(bm, _at(fr, sx * 0.012, 1.1 + sy * 0.012, -0.077), -z, 0.007, 1)
    # Top spike.
    blade(
        bm,
        _at(fr, 0, 1.14),
        y,
        x,
        0.2,
        0.013,
        0.01,
        g.seg(3, 1, 1),
        1,
        taper=1.0,
        point=0.02,
    )
    return [_obj("pollaxe", bm, _staff_mats(g))]


@gear("lance")
def lance(g, pennon=True):
    """War lance: fluted shaft swelling at the grip, steel vamplate, socketed head, pennon."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    n = g.seg(10, 6, 3)
    bm = bmesh.new()
    prof = [
        (-1.1, 0.024),
        (-0.25, 0.034),
        (-0.08, 0.027),
        (0.08, 0.027),
        (0.3, 0.034),
        (2.4, 0.017),
    ]
    for (y0, r0), (y1, r1) in zip(prof, prof[1:], strict=False):
        eq.tube(bm, _at(fr, 0, y0), _at(fr, 0, y1), r0, r1, n, 0)
    if g.level < 2:
        # Vamplate: a steel cone before the hand.
        eq.tube(bm, _at(fr, 0, 0.12), _at(fr, 0, 0.26), 0.1, 0.03, g.seg(14, 8, 4), 1)
    eq.tube(bm, _at(fr, 0, 2.36), _at(fr, 0, 2.46), 0.019, 0.014, g.seg(8, 4, 3), 1)
    blade(
        bm,
        _at(fr, 0, 2.46),
        y,
        z,
        0.16,
        0.022,
        0.009,
        g.seg(4, 2, 1),
        1,
        taper=0.8,
        point=0.02,
    )
    mats = _staff_mats(g, WOOD_LIGHT) + [g.mat(eq.C_LIVERY, (1, 1, 1))]
    if pennon and g.level < 2:
        # Pennon: forked streamer nailed below the head, gently waving (both sides).
        cols = g.seg(6, 3, 2)
        rows = []
        for k in range(cols + 1):
            t = k / cols
            wave = 0.03 * math.sin(t * math.pi * 1.5) * t
            y_top = 2.3 - 0.02 * t
            y_bot = 1.95 + 0.12 * t
            if t > 0.75:  # swallowtail
                mid = (y_top + y_bot) / 2
                y_top, y_bot = y_top - (t - 0.75) * 0.3, y_bot + (t - 0.75) * 0.3
                del mid
            rows.append((_at(fr, t * 0.5, y_top, wave), _at(fr, t * 0.5, y_bot, wave)))
        for flip in (False, True):
            vs = [
                (
                    bm.verts.new(a + z * (0.001 if flip else 0)),
                    bm.verts.new(b + z * (0.001 if flip else 0)),
                )
                for a, b in rows
            ]
            for k in range(cols):
                quad = (vs[k][0], vs[k][1], vs[k + 1][1], vs[k + 1][0])
                f = bm.faces.new(tuple(reversed(quad)) if flip else quad)
                f.material_index = 3
    return [_obj("lance", bm, mats)]


@gear("hand_culverin")
def hand_culverin(g):
    """Hand culverin: octagonal barrel banded at breech and muzzle on a long wooden tiller."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    n = g.seg(8, 6, 4)
    # Tiller: tapering stave from the butt (under the arm) to the barrel's seat.
    eq.tube(
        bm,
        _at(fr, 0, -0.42, -0.02),
        _at(fr, 0, 0.2, -0.01),
        0.018,
        0.028,
        g.seg(6, 4, 4),
        0,
    )
    eq.tube(bm, _at(fr, 0, 0.1, 0.01), _at(fr, 0, 0.72, 0.01), 0.032, 0.025, n, 1)
    if g.level < 2:
        for yy, r in ((0.1, 0.038), (0.36, 0.031), (0.68, 0.031)):
            eq.tube(bm, _at(fr, 0, yy, 0.01), _at(fr, 0, yy + 0.04, 0.01), r, r, n, 1)
        if g.level == 0:
            # Touch hole pan and the muzzle bore (dark).
            eq.box(bm, _at(fr, 0, 0.17, 0.045), (0.008, 0.01, 0.004), (x, y, z), 1)
            eq.tube(
                bm, _at(fr, 0, 0.721, 0.01), _at(fr, 0, 0.722, 0.01), 0.013, 0.013, 8, 3
            )
    mats = [
        g.mat(eq.C_WOOD, WOOD),
        g.mat(eq.C_PLATE, (0.24, 0.24, 0.25)),
        g.mat(eq.C_PLATE, IRON),
        g.mat(eq.C_EXACT, (0.01, 0.01, 0.01)),
    ]
    return [_obj("hand_culverin", bm, mats)]


# --- Bows ---------------------------------------------------------------------------------


@gear("crossbow")
def crossbow(g):
    """Crossbow: shaped tiller, steel prod bound in its bridle, stirrup, nut, trigger, string."""
    fr = weapons.prop_frame(g.ctx)
    _o, x, y, z = fr
    bm = bmesh.new()
    # Tiller: lofted section, deeper at the butt, narrow at the prod.
    prof = [
        (-0.26, 0.022, 0.035, -0.02),
        (-0.1, 0.02, 0.03, -0.012),
        (0.15, 0.018, 0.024, -0.008),
        (0.5, 0.016, 0.02, -0.006),
    ]
    steps = g.seg(8, 4, 4)
    rings = []
    for yy, hw, hh, dz in prof:
        c = _at(fr, 0, yy, dz)
        ring = []
        for k in range(steps):
            a = 2 * math.pi * k / steps + math.pi / steps
            ring.append(
                bm.verts.new(c + x * hw * math.cos(a) * 1.15 + z * hh * math.sin(a))
            )
        rings.append(ring)
    for a_, b_ in zip(rings, rings[1:], strict=False):
        eq.bridge(bm, a_, b_, 0)
    eq.cap(bm, rings[0], 0)
    eq.cap(bm, rings[-1], 0, flip=True)
    # Steel prod: flattened section, recurved slightly towards the archer at the tips.
    segs = g.seg(10, 5, 2)
    path = []
    for i in range(segs + 1):
        u = -1 + 2 * i / segs
        path.append(_at(fr, u * 0.36, 0.5 - 0.075 * u * u, 0.0))
    radii = [0.011 - 0.004 * abs(-1 + 2 * i / segs) for i in range(segs + 1)]
    sweep(bm, path, radii, g.seg(6, 4, 3), 1)
    if g.level < 2:
        # Stirrup: iron loop in front of the prod.
        loop = [
            _at(fr, 0.055 * math.cos(a), 0.52 + 0.09 * math.sin(a) + 0.02, 0.0)
            for a in [math.pi * k / 6 for k in range(7)]
        ]
        sweep(bm, loop, 0.005, g.seg(5, 3, 3), 2)
        # Bridle binding the prod to the tiller.
        eq.box(bm, _at(fr, 0, 0.495, -0.004), (0.024, 0.018, 0.026), (x, y, z), 3)
        # Nut and string (spanned).
        nut = _at(fr, 0, 0.18, 0.02)
        eq.tube(bm, nut - x * 0.013, nut + x * 0.013, 0.012, 0.012, g.seg(8, 4, 3), 1)
        sweep(bm, [path[0], nut + z * 0.012], 0.002, 3, 4, caps=False)
        sweep(bm, [path[-1], nut + z * 0.012], 0.002, 3, 4, caps=False)
        if g.level == 0:
            # Trigger lever under the tiller.
            sweep(
                bm,
                [
                    _at(fr, 0, 0.17, -0.03),
                    _at(fr, 0, 0.0, -0.06),
                    _at(fr, 0, -0.15, -0.07),
                ],
                0.005,
                4,
                2,
            )
    mats = [
        g.mat(eq.C_WOOD, WOOD),
        g.mat(eq.C_PLATE, STEEL),
        g.mat(eq.C_PLATE, IRON),
        g.mat(eq.C_LEATHER, GRIP),
        g.mat(eq.C_EXACT, STRING),
    ]
    return [_obj("crossbow", bm, mats)]


@gear("longbow")
def longbow(g, height=1.8):
    """Yew longbow: D-section limbs tapering to horn nocks, leather grip, string, arrow.

    Same rest placement as V2 (``battle_skinned_weapons.longbow``): the limbs along the left
    fist's forward axis, bent back at the tips; string on ``Nock``, arrow on ``Arrow``.
    """
    ctx = g.ctx
    c, along, _up, out = eq.grip(ctx, "L")
    vert = along.normalized()
    half = height / 2
    brace = 0.16
    segs = g.seg(16, 6, 2)
    bm = bmesh.new()
    pts = [
        c + vert * (u * half) - out * (brace * u * u)
        for u in (-1 + 2 * i / segs for i in range(segs + 1))
    ]
    radii = [
        0.019 * (1 - 0.6 * abs(-1 + 2 * i / segs)) + 0.002 for i in range(segs + 1)
    ]
    sweep(bm, pts, radii, g.seg(6, 4, 3), 0)
    mats = [
        g.mat(eq.C_WOOD, YEW),
        g.mat(eq.C_LEATHER, GRIP),
        g.mat(eq.C_EXACT, HORN),
        g.mat(eq.C_EXACT, STRING),
    ]
    if g.level < 2:
        eq.tube(
            bm, c - vert * 0.055, c + vert * 0.055, 0.0205, 0.0205, g.seg(8, 5, 3), 1
        )
        for end, prev in ((pts[0], pts[1]), (pts[-1], pts[-2])):
            d = (end - prev).normalized()
            eq.tube(
                bm, end - d * 0.02, end + d * 0.035, 0.0065, 0.002, g.seg(6, 4, 3), 2
            )
    bow = _obj("longbow", bm, mats, bone="Wrist.L")
    objs = [bow]
    if g.level < 2:
        nock = eq.nock_rest(ctx)
        bm = bmesh.new()
        eq.tube(bm, pts[0], nock, 0.002, 0.002, 3, 0, caps=False)
        eq.tube(bm, nock, pts[-1], 0.002, 0.002, 3, 0, caps=False)
        eq.finish(bm)
        string = eq.to_object("bowstring", bm, [g.mat(eq.C_EXACT, STRING)])

        def weigh(p):
            w = eq.smoothstep(0.0, 1.0, min((p - nock).length / half, 1.0))
            return {"Nock": 1 - w, "Wrist.L": w}

        eq.bind_by(string, weigh)
        objs.append(string)
        objs.append(arrow(g, nock, out, vert))
    return objs


def arrow(g, nock, out, vert, length=None):
    """Nocked livery arrow: ash shaft, bodkin head, three goose-feather fletchings."""
    length = length or ARROW_LENGTH
    bm = bmesh.new()
    tip = nock + out * length
    eq.tube(bm, nock, tip, 0.0045, 0.005, g.seg(5, 3, 3), 0, caps=False)
    eq.tube(bm, tip, tip + out * 0.07, 0.006, 0.0008, g.seg(4, 3, 3), 1, caps=False)
    side = vert.cross(out).normalized()
    if g.level == 0:
        for k in range(3):
            a = 2 * math.pi * k / 3
            d = vert * math.cos(a) + side * math.sin(a)
            quad = [
                nock + out * 0.03 + d * 0.004,
                nock + out * 0.2 + d * 0.004,
                nock + out * 0.17 + d * 0.019,
                nock + out * 0.05 + d * 0.016,
            ]
            for flip in (False, True):
                vs = [bm.verts.new(p) for p in quad]
                bm.faces.new(vs[::-1] if flip else vs).material_index = 2
    elif g.level == 1:
        quad = [
            nock + out * 0.03,
            nock + out * 0.2,
            nock + out * 0.17 + vert * 0.016,
            nock + out * 0.05 + vert * 0.016,
        ]
        bm.faces.new([bm.verts.new(p) for p in quad]).material_index = 2
    eq.finish(bm)
    obj = eq.to_object(
        "arrow",
        bm,
        [
            g.mat(eq.C_WOOD, (0.5, 0.4, 0.25)),
            g.mat(eq.C_PLATE, IRON),
            g.mat(eq.C_EXACT, FLETCH),
        ],
    )
    eq.bind_rigid(obj, "Arrow")
    return obj


@gear("quiver")
def quiver(g, arrows=True):
    """Arrow bag (linen, laced leather rim) or bolt case at the right hip, shafts showing."""
    hips = g.ctx.head("Hips")
    base = hips + Vector((-0.2, 0.07, -0.25))
    top = hips + Vector((-0.22, 0.12, 0.08))
    axis = (top - base).normalized()
    bm = bmesh.new()
    n = g.seg(10, 6, 4)
    prof = [(0.0, 0.035), (0.05, 0.05), (0.6, 0.055), (0.95, 0.06), (1.0, 0.062)]
    for (t0, r0), (t1, r1) in zip(prof, prof[1:], strict=False):
        eq.tube(
            bm,
            base.lerp(top, t0),
            base.lerp(top, t1),
            r0,
            r1,
            n,
            0 if t1 < 0.95 else 1,
            caps=t0 == 0.0,
        )
    if arrows and g.level < 2:
        count = g.seg(9, 4, 0)
        for k in range(count):
            a = 2 * math.pi * k / count
            r = 0.028 if k % 2 else 0.012
            p = top + Vector((math.cos(a) * r, math.sin(a) * r, 0))
            end = p + axis * 0.1 + Vector((math.cos(a), math.sin(a), 0)) * 0.012
            eq.tube(bm, p - axis * 0.02, end, 0.004, 0.004, 3, 2, caps=False)
            if g.level == 0:
                eq.tube(bm, end - axis * 0.07, end, 0.009, 0.006, 3, 3, caps=False)
    eq.finish(bm)
    obj = eq.to_object(
        "quiver",
        bm,
        [
            g.mat(eq.C_CLOTH, (0.45, 0.40, 0.30))
            if arrows
            else g.mat(eq.C_LEATHER, eq.LEATHER),
            g.mat(eq.C_LEATHER, eq.LEATHER),
            g.mat(eq.C_WOOD, (0.5, 0.4, 0.25)),
            g.mat(eq.C_EXACT, FLETCH),
        ],
    )
    eq.bind_by(obj, lambda p: {"Hips": 0.6, "UpperLeg.R": 0.4})
    return [obj]


# --- Shields ------------------------------------------------------------------------------


@gear("heater_shield")
def heater_shield(g):
    """Heater shield (FG0): curved board painted with the arms, raw-hide rim."""
    cols, rows = g.seg((10, 12), (4, 5), (2, 3))
    objs = fe.heater_shield(g.ctx, cols=cols, rows=rows)
    for obj in objs:
        obj.data.materials[0] = g.mat(eq.C_ARMS, (1, 1, 1))
        obj.data.materials[1] = g.mat(eq.C_WOOD, WOOD)
        obj.data.materials[2] = g.mat(eq.C_LEATHER, HIDE)
    return objs


def _dished(
    g,
    bm,
    centre,
    side,
    up,
    normal,
    radius_of,
    n,
    rings,
    depth,
    thick,
    face_mat,
    back_mat,
    rim_mat,
):
    """Dished board: painted front, plain back, rim; outline `radius_of(a)` (polar)."""
    front_rows, back_rows = [], []
    for r in range(1, rings + 1):
        t = r / rings
        fr_, bk = [], []
        for i in range(n):
            a = 2 * math.pi * i / n
            rr = radius_of(a) * t
            p = centre + side * math.cos(a) * rr + up * math.sin(a) * rr
            bulge = depth * (1 - t * t)
            fr_.append(bm.verts.new(p + normal * (thick + bulge)))
            bk.append(bm.verts.new(p + normal * bulge))
        front_rows.append(fr_)
        back_rows.append(bk)
    cf = bm.verts.new(centre + normal * (thick + depth))
    cb = bm.verts.new(centre + normal * depth)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((cf, front_rows[0][i], front_rows[0][j])).material_index = face_mat
        bm.faces.new((cb, back_rows[0][j], back_rows[0][i])).material_index = back_mat
        for r in range(rings - 1):
            bm.faces.new(
                (
                    front_rows[r][i],
                    front_rows[r + 1][i],
                    front_rows[r + 1][j],
                    front_rows[r][j],
                )
            ).material_index = face_mat
            bm.faces.new(
                (
                    back_rows[r][j],
                    back_rows[r + 1][j],
                    back_rows[r + 1][i],
                    back_rows[r][i],
                )
            ).material_index = back_mat
        bm.faces.new(
            (front_rows[-1][i], back_rows[-1][i], back_rows[-1][j], front_rows[-1][j])
        ).material_index = rim_mat
    return front_rows


def _panel_uv(obj, centre, u_axis, v_axis, width, height, v0=0.5):
    layer = obj.data.uv_layers.new(name="UVMap")
    mw_inv = obj.matrix_world.inverted()
    for loop in obj.data.loops:
        co = mw_inv @ obj.data.vertices[loop.vertex_index].co
        rel = co - centre
        layer.data[loop.index].uv = (
            0.5 + rel.dot(u_axis) / width,
            v0 + rel.dot(v_axis) / height,
        )


@gear("round_shield")
def round_shield(g, radius=0.2, back=False, arms=True, boss=True):
    """Round shields: rondache (painted), buckler (small, steel boss), targe (studded hide).

    A painted rondache has an iron rim; the Welsh buckler (unpainted, small) is a dished
    wooden board with a steel boss; the Scots targe (on the back) is hide over boards with
    brass nails in rings round a central boss.
    """
    ctx = g.ctx
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
    n = g.seg(24, 10, 6)
    rings = g.seg(4, 2, 1)
    targe = back and not arms
    buckler = not arms and not back
    bm = bmesh.new()
    face = 0
    front = _dished(
        g,
        bm,
        centre,
        side,
        up,
        normal,
        lambda a: radius,
        n,
        rings,
        0.03 if not targe else 0.015,
        0.012,
        face,
        1,
        2,
    )
    if g.level < 2:
        # Rim band.
        path = [
            centre
            + (
                side * math.cos(2 * math.pi * i / n)
                + up * math.sin(2 * math.pi * i / n)
            )
            * radius
            + normal * 0.012
            for i in range(n)
        ]
        sweep(bm, path, 0.006, g.seg(4, 3, 3), 2, closed=True)
    if boss and (buckler or targe or g.level < 2):
        br = 0.065 if buckler else 0.05
        c0 = centre + normal * (0.012 + 0.03)
        for k in range(g.seg(3, 2, 1)):
            t0 = k / g.seg(3, 2, 1)
            t1 = (k + 1) / g.seg(3, 2, 1)
            a0, a1 = t0 * math.pi / 2, t1 * math.pi / 2
            eq.tube(
                bm,
                c0 + normal * math.sin(a0) * br * 0.7,
                c0 + normal * math.sin(a1) * br * 0.7,
                br * math.cos(a0) + 0.001,
                br * math.cos(a1) + 0.001,
                g.seg(12, 6, 4),
                3,
                caps=k == 0,
            )
    if targe and g.level == 0:
        for ring_r, count in ((0.09, 8), (0.16, 14)):
            for k in range(count):
                a = 2 * math.pi * k / count
                p = centre + (side * math.cos(a) + up * math.sin(a)) * ring_r
                t = ring_r / radius
                rivet(bm, p + normal * (0.012 + 0.015 * (1 - t * t)), normal, 0.006, 4)
    elif arms and g.level == 0:
        for k in range(12):
            a = 2 * math.pi * k / 12
            p = centre + (side * math.cos(a) + up * math.sin(a)) * (radius - 0.015)
            rivet(bm, p + normal * 0.014, normal, 0.005, 4)
    del front
    eq.finish(bm)
    if arms:
        face_mat = g.mat(eq.C_ARMS, (1, 1, 1))
    elif targe:
        face_mat = g.mat(eq.C_LEATHER, HIDE)
    else:
        face_mat = g.mat(eq.C_WOOD, WOOD_LIGHT)
    boss_mat = g.mat(eq.C_PLATE, STEEL) if buckler else g.mat(eq.C_PLATE, IRON)
    obj = eq.to_object(
        "round_shield",
        bm,
        [
            face_mat,
            g.mat(eq.C_WOOD, WOOD),
            g.mat(eq.C_PLATE, IRON),
            boss_mat,
            g.mat(eq.C_TRIM, BRASS),
        ],
    )
    _panel_uv(obj, centre, side, -up, 2 * radius, 2 * radius)
    if back:
        eq.bind_by(obj, lambda p: {"Chest": 0.7, "Torso": 0.3})
    else:
        eq.bind_rigid(obj, "LowerArm.L")
    return [obj]


@gear("adarga")
def adarga(g, width=0.5, height=0.62):
    """Adarga: heart-shaped hide shield (two lobes up), dished, with a stitched rim."""
    ctx = g.ctx
    lower = ctx.head("LowerArm.L")
    wrist = ctx.head("Wrist.L")
    fore = (wrist - lower).normalized()
    centre = lower.lerp(wrist, 0.45) + Vector((0, 0, 0.08))
    normal = Vector((0, 0, 1))
    side = normal.cross(fore).normalized()
    n = g.seg(32, 14, 8)

    # Polar radius of the heart (V2 curve, lobes towards +fore), sampled densely and
    # interpolated by angle.
    samples = []
    for k in range(512):
        t = 2 * math.pi * k / 512
        hx = math.sin(t) ** 3 * width / 2
        hy = (
            (
                13 * math.cos(t)
                - 5 * math.cos(2 * t)
                - 2 * math.cos(3 * t)
                - math.cos(4 * t)
            )
            / 17
            * height
            / 2
        )
        samples.append((math.atan2(hy, hx) % (2 * math.pi), math.hypot(hx, hy)))
    samples.sort()

    def outline(a):
        a %= 2 * math.pi
        for (a0, r0), (a1, r1) in zip(
            samples,
            samples[1:] + [(samples[0][0] + 2 * math.pi, samples[0][1])],
            strict=False,
        ):
            if a0 <= a <= a1 or a0 <= a + 2 * math.pi <= a1:
                u = ((a if a >= a0 else a + 2 * math.pi) - a0) / max(a1 - a0, 1e-9)
                return r0 + (r1 - r0) * u
        return width / 2

    radii = [outline(2 * math.pi * i / n) for i in range(n)]
    bm = bmesh.new()
    _dished(
        g,
        bm,
        centre,
        side,
        fore,
        normal,
        lambda a: radii[int(round(a / (2 * math.pi) * n)) % n],
        n,
        g.seg(4, 2, 1),
        0.035,
        0.01,
        0,
        1,
        1,
    )
    eq.finish(bm)
    obj = eq.to_object(
        "adarga", bm, [g.mat(eq.C_ARMS, (1, 1, 1)), g.mat(eq.C_LEATHER, HIDE)]
    )
    _panel_uv(obj, centre, side, -fore, width, height)
    eq.bind_rigid(obj, "LowerArm.L")
    return [obj]


@gear("pavise")
def pavise(g):
    """Pavise slung on the back: tall board with a central ridge (canal), rim, painted."""
    chest = g.ctx.head("Chest")
    c = chest + Vector((0, 0.22, -0.15))
    w, h = 0.34, 0.6
    cols = g.seg(8, 4, 2)
    rows = g.seg(6, 3, 1)
    bm = bmesh.new()
    front, back_ = [], []
    for r in range(rows + 1):
        v = -1 + 2 * r / rows
        fr_, bk = [], []
        for i in range(cols + 1):
            u = -1 + 2 * i / cols
            half = w * (1 - 0.1 * (v + 1) / 2)
            x = u * half
            # Curved board, ridge down the middle.
            y = 0.04 * (1 - u * u) + 0.03 * max(0.0, 1 - abs(u) * 4)
            p = c + Vector((-x, y, v * h))
            fr_.append(bm.verts.new(p + Vector((0, 0.025, 0))))
            bk.append(bm.verts.new(p))
        front.append(fr_)
        back_.append(bk)
    for r in range(rows):
        for i in range(cols):
            bm.faces.new(
                (front[r][i], front[r][i + 1], front[r + 1][i + 1], front[r + 1][i])
            ).material_index = 0
            bm.faces.new(
                (back_[r][i], back_[r + 1][i], back_[r + 1][i + 1], back_[r][i + 1])
            ).material_index = 1
    border_f = (
        [front[r][0] for r in range(rows + 1)]
        + front[rows][1:]
        + [front[r][cols] for r in range(rows - 1, -1, -1)]
        + [front[0][i] for i in range(cols - 1, 0, -1)]
    )
    border_b = (
        [back_[r][0] for r in range(rows + 1)]
        + back_[rows][1:]
        + [back_[r][cols] for r in range(rows - 1, -1, -1)]
        + [back_[0][i] for i in range(cols - 1, 0, -1)]
    )
    m = len(border_f)
    for i in range(m):
        k = (i + 1) % m
        bm.faces.new(
            (border_f[i], border_b[i], border_b[k], border_f[k])
        ).material_index = 2
    eq.finish(bm)
    obj = eq.to_object(
        "pavise",
        bm,
        [
            g.mat(eq.C_ARMS, (1, 1, 1)),
            g.mat(eq.C_WOOD, WOOD),
            g.mat(eq.C_LEATHER, HIDE),
        ],
    )
    _panel_uv(obj, c, Vector((-1, 0, 0)), Vector((0, 0, -1)), 2 * w, 2 * h)
    eq.bind_by(obj, lambda p: {"Chest": 0.7, "Torso": 0.3})
    return [obj]
