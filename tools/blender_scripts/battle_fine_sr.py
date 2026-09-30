"""Lot SR3b: realism fixes of the fine figures (gaps with the SR3 reference sheet).

Gaps and choices: ``docs/research/sr3b-ecarts.md``. Everything here is fine-only: the
Quaternius recipes of ``battle_skinned_figures`` (``--coarse-figures``) are not touched.

- ``fine_recipe``: copy of a recipe with the SR3b overrides (hose colours per recipe, mail
  aventail under the kettle hats of the sergeants and crossbowmen).
- ``gloves``: leather gloves of the sergeants with staff weapons.
- ``extras``: belt kit (purse, dagger) and a buckler hung at the belt, rigid on ``Hips``.

The gauntlet cuffs of the harnesses are in ``battle_fine_gear.limb_harness`` (shells of the
body, same weights) and the aventail under the kettle hat in ``battle_fine_gear.kettle_hat``.
Every piece is bound to existing bones: the bone textures and clips are reused.
"""

import copy
import math

import battle_fine_equipment as fe
import battle_fine_gear as gear
import battle_skinned_equipment as eq
import bmesh
from mathutils import Vector

# --- Recipe overrides -----------------------------------------------------------------------

# Hose (linear colours): green, blue, black, russet... instead of the same brown everywhere
# (reference sheet: English archer green, Genoese blue, sergeant black). The shader's
# per-soldier dyes still vary them slightly.
GREEN = (0.07, 0.13, 0.05)
BLUE = (0.07, 0.09, 0.16)
BLACK = (0.035, 0.03, 0.028)
RUSSET = (0.20, 0.075, 0.04)
GREY_GREEN = (0.11, 0.115, 0.075)
DRAB = (0.15, 0.12, 0.08)
HOSE = {
    "archer_0": GREEN,
    "cavalry_2": GREEN,
    "archer_1": BLUE,
    "archer_2": BLUE,
    "archer_3": RUSSET,
    "archer_4": GREY_GREEN,
    "archer_5": BLACK,
    "infantry_1": DRAB,
    "infantry_2": GREY_GREEN,
    "infantry_5": GREEN,
    "infantry_6": BLACK,
    "musician_0": RUSSET,
    "musician_1": BLUE,
    "cavalry_1": BLACK,
    "cavalry_6": GREY_GREEN,
}
# Material key of the hose per Quaternius outfit (see battle_fine_figures.dress).
HOSE_KEYS = ("Adventurer_Legs:Brown2", "Farmer_Pants:LightBlue", "Medieval_Legs:Black")

# Mail aventail (a coif's collar) under the kettle hat: sergeants, pikemen, crossbowmen.
KETTLE_AVENTAIL = {
    "infantry_1",
    "infantry_4",
    "infantry_5",
    "archer_1",
    "archer_2",
    "archer_4",
    "cavalry_1",
}

# Leather gloves (bare hands otherwise, except the harnesses' gauntlets).
GLOVE_LEATHER = (0.12, 0.075, 0.04)
GLOVED = {
    "infantry_1",
    "infantry_4",
    "infantry_5",
    "infantry_6",
    "cavalry_1",
    "cavalry_4",
    "cavalry_6",
}

# Belt kit of the figures on foot: purse and dagger, or the dagger alone (harness: rondel
# at the right hip); buckler at the back of the belt (archers, militia).
BELT_KIT = {
    **{
        f: ("purse", "dagger")
        for f in (
            "infantry_1",
            "infantry_2",
            "infantry_3",
            "infantry_4",
            "infantry_5",
            "infantry_6",
            "infantry_8",
            "archer_0",
            "archer_1",
            "archer_2",
            "archer_3",
            "archer_4",
            "archer_5",
            "musician_0",
            "musician_1",
        )
    },
    "infantry_0": ("rondel",),
    "infantry_7": ("rondel",),
    "standard_0": ("rondel",),
}
BUCKLER = {"archer_0", "archer_3", "infantry_2"}


def fine_recipe(fig_name, recipe):
    """Copy of `recipe` with the SR3b overrides of `fig_name`."""
    out = copy.deepcopy(recipe)
    hose = HOSE.get(fig_name)
    if hose is not None:
        colors = out.setdefault("colors", {})
        for key in HOSE_KEYS:
            code = colors.get(key, (eq.C_CLOTH,))[0]
            if code == eq.C_CLOTH:
                colors[key] = (eq.C_CLOTH, hose)
    if fig_name in KETTLE_AVENTAIL:
        items = []
        for item in out.get("equipment", []):
            if item[0] == "kettle_hat":
                kwargs = dict(item[2]) if len(item) > 2 else {}
                kwargs["aventail"] = True
                item = (item[0], item[1], kwargs)
            items.append(item)
        out["equipment"] = items
    return out


def gloves(fig_name, current):
    """Glove (code, rgb) of `fig_name`: leather for the sergeants, else `current`."""
    if current is None and fig_name in GLOVED:
        return (eq.C_LEATHER, GLOVE_LEATHER)
    return current


# --- Belt pieces ----------------------------------------------------------------------------

LEATHER_DARK = (0.09, 0.055, 0.03)
WOOD = (0.30, 0.20, 0.11)


def _belt_frame(g, worn):
    """(centre, z) of the belt of `g` (as ``battle_fine_equipment.belt``) and ray trees."""
    lm = g.lm
    z = lm.waist_z - 0.07
    centre = (lm.bone["UpperLeg.L"] + lm.bone["UpperLeg.R"]) / 2
    trees = list(g.bvhs) + [gear.world_tree(o) for o in worn]
    return Vector((centre.x, centre.y, z)), trees


def _radius(trees, centre, d, z, fallback=0.17):
    """Outer radius of the worn garments from `centre` towards `d` at height `z`."""
    c = Vector((centre.x, centre.y, z))
    r = max((fe._ray_radius(t, c, d, 0.0) for t in trees), default=0.0)
    return r if r > 0.0 else fallback


def _dir(x, y):
    """Horizontal unit direction."""
    return Vector((x, y, 0.0)).normalized()


def purse(g, centre, trees):
    """Leather purse with a flap hanging at the front left of the belt."""
    d = _dir(0.45, -0.89)  # the figure's left is +X, its front -Y
    top = centre.z - 0.01
    mid = top - 0.055
    r = max(_radius(trees, centre, d, top), _radius(trees, centre, d, mid - 0.04))
    t = Vector((-d.y, d.x, 0.0))
    up = Vector((0, 0, 1))
    out = d * (r + 0.02)
    base = Vector((centre.x, centre.y, 0.0)) + out
    bm = bmesh.new()
    eq.box(bm, base + up * mid, (0.045, 0.016, 0.05), (t, d, up), 0)
    eq.box(
        bm, base + up * (top - 0.028) + d * 0.004, (0.048, 0.017, 0.024), (t, d, up), 1
    )
    if g.level == 0:
        eq.box(
            bm,
            base + up * (top - 0.05) + d * 0.022,
            (0.006, 0.003, 0.006),
            (t, d, up),
            2,
        )
        # Strap loop over the belt.
        eq.box(
            bm,
            base + up * (top + 0.012) - d * 0.004,
            (0.012, 0.006, 0.02),
            (t, d, up),
            1,
        )
    eq.finish(bm)
    obj = eq.to_object(
        "purse",
        bm,
        [
            g.mat(eq.C_LEATHER, (0.16, 0.10, 0.05)),
            g.mat(eq.C_LEATHER, LEATHER_DARK),
            g.mat(eq.C_TRIM, gear.BRASS),
        ],
    )
    eq.bind_rigid(obj, "Hips")
    return obj


def dagger(g, centre, trees, rondel=False):
    """Dagger in its sheath at the belt.

    Ballock dagger at the back right, or a rondel at the right hip (harness).
    """
    d = _dir(-1.0, 0.25) if rondel else _dir(-0.55, 0.83)
    top = centre.z + 0.01
    r = max(_radius(trees, centre, d, top), _radius(trees, centre, d, top - 0.15))
    base = Vector((centre.x, centre.y, 0.0)) + d * (r + 0.025)
    # Hanging almost upright, point slightly forward and out.
    axis = (Vector((0, 0, -1)) + d * 0.12 + Vector((0, -0.1, 0))).normalized()
    hilt = base + Vector((0, 0, top))
    n = g.seg(6, 4, 4)
    bm = bmesh.new()
    sheath_end = hilt + axis * 0.27
    eq.tube(bm, hilt, sheath_end, 0.017, 0.007, n, 0)  # sheath
    if g.level == 0:
        eq.tube(
            bm, sheath_end - axis * 0.04, sheath_end + axis * 0.005, 0.009, 0.004, n, 2
        )
    # Hilt: guard (rondel disc or ballocks), grip, pommel.
    grip_end = hilt - axis * 0.1
    if rondel:
        eq.tube(
            bm,
            hilt - axis * 0.006,
            hilt + axis * 0.004,
            0.024,
            0.024,
            g.seg(8, 6, 4),
            2,
        )
        eq.tube(
            bm,
            grip_end - axis * 0.004,
            grip_end - axis * 0.012,
            0.022,
            0.022,
            g.seg(8, 6, 4),
            2,
        )
    else:
        side = axis.cross(d).normalized()
        for s in (-1, 1):
            eq.box(
                bm,
                hilt + side * s * 0.014 - axis * 0.008,
                (0.012, 0.012, 0.012),
                None,
                1,
            )
        eq.box(bm, grip_end - axis * 0.006, (0.01, 0.01, 0.01), None, 1)
    eq.tube(bm, hilt, grip_end, 0.011, 0.01, n, 1)
    eq.finish(bm)
    obj = eq.to_object(
        "dagger",
        bm,
        [
            g.mat(eq.C_LEATHER, LEATHER_DARK),
            g.mat(eq.C_WOOD, WOOD),
            g.mat(eq.C_TRIM, gear.IRON if not rondel else gear.STEEL),
        ],
    )
    eq.bind_rigid(obj, "Hips")
    return obj


def buckler(g, centre, trees, radius=0.15):
    """Buckler hung at the back left of the belt: dished wooden board, iron rim and boss."""
    d = _dir(0.5, 0.87)
    zc = centre.z - 0.13
    r = max(
        _radius(trees, centre, d, z) for z in (zc - radius * 0.6, zc, zc + radius * 0.6)
    )
    c = Vector((centre.x, centre.y, zc)) + d * (r + 0.035)
    t = Vector((-d.y, d.x, 0.0))
    up = Vector((0, 0, 1))
    n = g.seg(16, 8, 6)
    detail = g.level == 0

    def ring(k, bulge, back=0.0):
        pts = []
        for i in range(n):
            a = 2 * math.pi * i / n
            p = (
                c
                + (t * math.cos(a) + up * math.sin(a)) * radius * k
                + d * (bulge - back)
            )
            pts.append(bm.verts.new(p))
        return pts

    bm = bmesh.new()
    # Front (outwards along d): boss dome, dished board, rolled rim; flat back.
    if detail:
        rings = [
            (ring(0.3, 0.05), 2),
            (ring(0.34, 0.03), 2),
            (ring(0.62, 0.02), 0),
            (ring(0.94, 0.008), 0),
            (ring(1.0, 0.004), 1),
            (ring(1.0, -0.006), 1),
        ]
        apex = bm.verts.new(c + d * 0.07)
    else:
        rings = [(ring(0.3, 0.045), 2), (ring(1.0, 0.004), 0), (ring(1.0, -0.006), 1)]
        apex = bm.verts.new(c + d * 0.06)
    for i in range(n):
        f = bm.faces.new((apex, rings[0][0][(i + 1) % n], rings[0][0][i]))
        f.material_index = 2
    for (ra, _ma), (rb, mb) in zip(rings, rings[1:], strict=False):
        eq.bridge(bm, ra, rb, mb)
    eq.cap(bm, rings[-1][0], 0)
    eq.finish(bm)
    obj = eq.to_object(
        "buckler",
        bm,
        [
            g.mat(eq.C_WOOD, (0.26, 0.17, 0.09)),
            g.mat(eq.C_TRIM, gear.IRON),
            g.mat(eq.C_PLATE, gear.STEEL_DARK),
        ],
    )
    eq.bind_by(obj, lambda p: {"Hips": 0.85, "UpperLeg.L": 0.15})
    return obj


# Equipment worn over the tunic whose surface carries the belt pieces (ray targets).
WORN = ("jack", "brigandine", "tabard", "surcoat", "cuirass")


def extras(fig_name, g, equipment, level, mounted):
    """SR3b pieces of `fig_name` at `level` (world space, bound to ``Hips``)."""
    if mounted or level == 2:
        return []
    kit = BELT_KIT.get(fig_name, ())
    has_buckler = fig_name in BUCKLER
    if not kit and not has_buckler:
        return []
    worn = [o for o, _m, _k in equipment if o.name.startswith(WORN)]
    worn += [o for o, _r in g.garments if o.name.startswith(WORN)]
    centre, trees = _belt_frame(g, worn)
    out = []
    if level == 0:
        if "purse" in kit:
            out.append(purse(g, centre, trees))
        if "dagger" in kit:
            out.append(dagger(g, centre, trees))
        if "rondel" in kit:
            out.append(dagger(g, centre, trees, rondel=True))
    if has_buckler:
        out.append(buckler(g, centre, trees))
    return out
