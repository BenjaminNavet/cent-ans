r"""Lot FG4: fine horse, harness and bards of the mounted battle figures (``cavalry`` rig).

Used by the FG1 pipeline (``battle_fine.py figures``, ``battle_fine_figures.build_figure``):
for every mounted recipe (``cavalry_0``-``6``, ``standard_1``) and level of detail, the
CC0 horse fitted on the Quaternius horse bones (``battle_fine_horse.build_horse``) in one of
three builds (destrier, rouncey, jennet), with its harness and the bards of the recipe
(``horse_and_harness``), replaces the Quaternius horse under the fine rider.

Colour and material codes per vertex as in ``battle_skinned``: the coat is ``C_COAT`` with a
per-vertex shade (``fg_shade``) that the shader's robe tint multiplies (bay, chestnut, black,
grey, dun by soldier); mane and tail follow the robe, darker.

Check renders (Eevee, several frames per clip, horse builds and robes, LODs):

    blender -b --factory-startup --python tools/blender_scripts/battle_fine_cavalry.py \
        -- --render DIR [--only cavalry_0,cavalry_1]
    uv run --project tools python tools/blender_scripts/fg4_planche.py DIR
"""

import contextlib
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_fine_horse as fh  # noqa: E402
import battle_skinned as bs  # noqa: E402
import battle_skinned_cavalry as cav  # noqa: E402
import battle_skinned_equipment as eq  # noqa: E402
import battle_skinned_poses as poses  # noqa: E402
import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402
from mathutils.kdtree import KDTree  # noqa: E402

MOUNTED = [
    "cavalry_0",
    "cavalry_1",
    "cavalry_2",
    "cavalry_3",
    "cavalry_4",
    "cavalry_5",
    "cavalry_6",
    "standard_1",
]

# --- Horse builds -------------------------------------------------------------------------
#
# The bones are shared, so every build keeps the height and the joints of the destrier;
# lighter horses are slimmer around their bones (barrel, belly, neck, legs).
HORSE_TYPES = {
    # Knights, gendarmes, standard bearer: the heavy war horse, as fitted.
    "destrier": {"barrel": 1.0, "belly": 1.0, "neck": 1.0, "leg": 1.0},
    # Sergeants, mounted archers, écorcheurs, hobelars: riding horse (roncin).
    "rouncey": {"barrel": 0.9, "belly": 0.9, "neck": 0.9, "leg": 0.85},
    # Jinetes: small light Iberian horse (genet).
    "jennet": {"barrel": 0.83, "belly": 0.85, "neck": 0.84, "leg": 0.78},
}
HORSE_OF = {
    "cavalry_0": "destrier",
    "cavalry_3": "destrier",
    "standard_1": "destrier",
    "cavalry_1": "rouncey",
    "cavalry_2": "rouncey",
    "cavalry_4": "rouncey",
    "cavalry_6": "rouncey",
    "cavalry_5": "jennet",
}

# Triangles per horse piece and level (cards are doubled afterwards: no alpha, back faces
# culled). Horse alone: LOD0 ~5.7 k, LOD1 ~0.8 k, LOD2 ~200 (FG5: LOD1 and LOD2 lighter).
HORSE_LOD = [
    {"horse_body": 3700, "horse_mane": 600, "horse_tail": 350, "eyes": True},
    {"horse_body": 600, "horse_mane": 60, "horse_tail": 36, "eyes": False},
    {"horse_body": 170, "horse_mane": 12, "horse_tail": 8, "eyes": False},
]
CARDS = ("horse_mane", "horse_tail")

COAT_RGB = (0.8, 0.8, 0.8)  # times fg_shade, times the robe of the shader
HAIR_RGB = (0.33, 0.33, 0.33)  # mane and tail: the robe, darker
HOOF_RGB = (0.055, 0.047, 0.04)
EYE_RGB = (0.012, 0.009, 0.007)
LEATHER_DARK = (0.10, 0.05, 0.025)
LEATHER_SEAT = (0.17, 0.09, 0.04)
BARD_STEEL = (0.62, 0.63, 0.66)

LEG_BONES = {
    f"{b}.{s}"
    for b in (
        "FrontUpperLeg",
        "FrontLowerLeg",
        "IKFrontLeg",
        "FF",
        "BackLeg",
        "BackUpperLeg",
        "BackLowerLeg",
        "IKBackLeg",
        "FFB",
    )
    for s in "LR"
}
NECK_BONES = {"Neck1", "Neck2", "Neck3"}


def _q_joints(mount):
    arm = mount.harm
    return {b.name: arm.matrix_world @ b.head_local for b in arm.data.bones}


def slim(pieces, mount, factors):
    """Lighter build: every vertex drawn towards its bones' axes (rest pose)."""
    if all(abs(v - 1.0) < 1e-6 for v in factors.values()):
        return
    q = _q_joints(mount)
    segs = {}
    for b, child in fh.SEGMENT_TO.items():
        if b in q and child in q:
            segs[b] = (q[b], q[child])
    for o in pieces:
        if not o.name.startswith(("horse_body", "horse_mane")):
            continue
        table = fh.weight_table(o)
        for v, w in zip(o.data.vertices, table, strict=True):
            p = v.co.copy()
            acc = Vector()
            total = 0.0
            for b, x in w.items():
                seg = segs.get(b)
                if seg is None:
                    acc += x * p
                    total += x
                    continue
                a, e = seg
                ax = e - a
                t = max(0.0, min(1.0, (p - a).dot(ax) / max(ax.length_squared, 1e-9)))
                c = a + ax * t
                r = p - c
                along = ax.normalized() * r.dot(ax.normalized())
                perp = r - along
                if b in LEG_BONES:
                    perp *= factors["leg"]
                elif b in NECK_BONES:
                    perp.x *= factors["neck"]
                else:
                    perp.x *= factors["barrel"]
                    if perp.z < 0:
                        perp.z *= factors["belly"]
                acc += x * (c + along + perp)
                total += x
            v.co = acc / total if total > 0 else p
        o.data.update()


def decimate_rest(obj, target):
    """Collapse-decimate `obj` to about `target` triangles before its armature modifier."""
    obj.data.calc_loop_triangles()
    tris = len(obj.data.loop_triangles)
    if not target or tris <= target:
        return tris
    mod = obj.modifiers.new("dec", "DECIMATE")
    mod.ratio = target / tris
    mod.use_collapse_triangulate = True
    with bpy.context.temp_override(
        object=obj, active_object=obj, selected_objects=[obj]
    ):
        bpy.ops.object.modifier_move_to_index(modifier=mod.name, index=0)
        bpy.ops.object.modifier_apply(modifier=mod.name)
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def double_sided(obj):
    """Add reversed copies of the faces (hair cards: the shader culls back faces)."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    faces = list(bm.faces)
    dup = bmesh.ops.duplicate(bm, geom=faces)
    new_faces = [g for g in dup["geom"] if isinstance(g, bmesh.types.BMFace)]
    bmesh.ops.reverse_faces(bm, faces=new_faces)
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def copy_shade(obj, source):
    """``fg_shade`` of the nearest vertex of `source` (the LOD0 body) on every vertex."""
    src = source.data.color_attributes["fg_shade"]
    tree = KDTree(len(source.data.vertices))
    for v in source.data.vertices:
        tree.insert(v.co, v.index)
    tree.balance()
    me = obj.data
    attr = me.color_attributes.get("fg_shade") or me.color_attributes.new(
        "fg_shade", "FLOAT_COLOR", "POINT"
    )
    for v in me.vertices:
        _co, idx, _d = tree.find(v.co)
        attr.data[v.index].color = src.data[idx].color


def horse_materials(pieces):
    """Material codes of the horse pieces (coat and hooves, hair, eyes)."""
    for o in pieces:
        me = o.data
        me.materials.clear()
        if o.name.startswith("horse_body"):
            me.materials.append(bs.material(eq.C_COAT, COAT_RGB))
            me.materials.append(bs.material(eq.C_EXACT, HOOF_RGB))
            flag = me.attributes.get("fg_hoof")
            for poly in me.polygons:
                poly.material_index = 1 if flag and flag.data[poly.index].value else 0
        elif o.name.startswith(CARDS):
            me.materials.append(bs.material(eq.C_COAT, HAIR_RGB))
        else:
            me.materials.append(bs.material(eq.C_EXACT, EYE_RGB))


# CR4: mane and tail as strand locks (triangular prisms) instead of the CC0 hair cards,
# whose alpha the battle shader does not read (solid dark blocks). Per level: (strands,
# rings before the tip); LOD0 stays within the cards' doubled budget (mane 1200, tail 700).
STRANDS = {
    "horse_tail": ((22, 5), (6, 2), (2, 1)),
    "horse_mane": ((44, 4), (10, 2), (4, 1)),
}
STRAND_WIDTH = {"horse_tail": (0.021, 0.004), "horse_mane": (0.016, 0.003)}


def _golden(i, n):
    """Point `i` of `n` spread over the unit disk (sunflower pattern)."""
    r = math.sqrt((i + 0.5) / n)
    a = i * 2.39996
    return r * math.cos(a), r * math.sin(a)


def _polyline_at(pts, t):
    """Point at parameter `t` in [0, 1] along the polyline `pts` (uniform per segment)."""
    if t <= 0:
        return pts[0].copy()
    x = min(t, 1.0) * (len(pts) - 1)
    i = min(int(x), len(pts) - 2)
    return pts[i].lerp(pts[i + 1], x - i)


def _tail_paths(verts, card_table, n, rings):
    """Strand polylines of the tail: centroid line from the dock, cross-section spread."""
    import numpy as np

    root_pts = [
        v for v, w in zip(verts, card_table, strict=True) if _dominant(w) == "Tail1"
    ] or [max(verts, key=lambda v: v.z)]
    root = sum(root_pts, Vector()) / len(root_pts)
    d = [(v - root).length for v in verts]
    far = max(d)
    bins = 8
    centres, axes = [], []
    for m in range(bins):
        lo, hi = far * m / bins, far * (m + 1) / bins
        sel = [v for v, x in zip(verts, d, strict=True) if lo <= x <= hi] or [root]
        c = sum(sel, Vector()) / len(sel)
        centres.append(c)
        arr = (
            np.array([tuple(v - c) for v in sel]) if len(sel) > 2 else np.eye(3) * 0.01
        )
        cov = arr.T @ arr / max(len(sel), 1)
        axes.append(cov)
    centres[0] = root
    paths = []
    for i in range(n):
        a, b = _golden(i, n)
        length = 0.72 + 0.28 * ((i * 0.6180339) % 1.0)
        pts = []
        for j in range(rings + 1):
            t = length * j / rings
            c = _polyline_at(centres, t)
            m = min(int(t * (bins - 1) + 0.5), bins - 1)
            vals, vecs = np.linalg.eigh(axes[m])
            e1 = Vector(vecs[:, 2]) * math.sqrt(max(vals[2], 1e-6)) * 1.6
            e2 = Vector(vecs[:, 1]) * math.sqrt(max(vals[1], 1e-6)) * 1.6
            spread = 0.25 + 0.75 * min(1.0, t * 3.0)  # the locks gather at the dock
            pts.append(c + (e1 * a + e2 * b) * spread)
        paths.append(pts)
    return paths


def _mane_paths(verts, n, rings):
    """Strand polylines of the mane: from the crest down the card, along the neck."""
    import numpy as np

    mean = sum(verts, Vector()) / len(verts)
    arr = np.array([tuple(v - mean) for v in verts])
    _vals, vecs = np.linalg.eigh(arr.T @ arr)
    e = Vector(vecs[:, 2])
    s = [(v - mean).dot(e) for v in verts]
    lo, hi = min(s), max(s)
    bins = max(6, min(14, n // 3))
    lines = []
    for m in range(bins):
        a, b = lo + (hi - lo) * m / bins, lo + (hi - lo) * (m + 1) / bins
        sel = [v for v, x in zip(verts, s, strict=True) if a <= x <= b]
        if len(sel) < 3:
            lines.append(None)
            continue
        root = max(sel, key=lambda v: v.z)
        sel.sort(key=lambda v: (v - root).length)
        groups = [
            sel[len(sel) * k // rings : len(sel) * (k + 1) // rings]
            for k in range(rings)
        ]
        line = [root] + [sum(g, Vector()) / len(g) for g in groups if g]
        while len(line) < rings + 1:
            line.append(line[-1].copy())
        lines.append(line)
    good = [x for x in lines if x is not None]
    lines = [x if x is not None else good[0] for x in lines]
    paths = []
    for i in range(n):
        u = (i + 0.5) / n * (bins - 1)
        k = min(int(u), bins - 2) if bins > 1 else 0
        f = u - k
        base = [
            p.lerp(q, f)
            for p, q in zip(lines[k], lines[min(k + 1, bins - 1)], strict=True)
        ]
        jit = ((i * 0.7548776) % 1.0 - 0.5) * 0.012
        length = 0.75 + 0.25 * ((i * 0.5698403) % 1.0)
        pts = [
            _polyline_at(base, length * j / rings) + e * jit for j in range(rings + 1)
        ]
        paths.append(pts)
    return paths


def strands(card, level, mount):
    """Replace the hair card `card` by strand locks with its weights and material."""
    name = card.name
    n, rings = STRANDS[name][level]
    w0, w1 = STRAND_WIDTH[name]
    table = fh.weight_table(card)
    verts = [card.matrix_world @ v.co for v in card.data.vertices]
    paths = (
        _tail_paths(verts, table, n, rings)
        if name == "horse_tail"
        else _mane_paths(verts, n, rings)
    )
    bm = bmesh.new()
    uv_of = {}
    for pts in paths:
        ring_prev = None
        for j, p in enumerate(pts):
            t = j / rings
            if j == rings:
                tip = bm.verts.new(p)
                uv_of[tip] = (0.5, 1.0)
                for k in range(3):
                    bm.faces.new((ring_prev[k], ring_prev[(k + 1) % 3], tip))
                break
            tan = (pts[j + 1] - p).normalized()
            ref = Vector((1, 0, 0)) if abs(tan.x) < 0.9 else Vector((0, 1, 0))
            nrm = tan.cross(ref).normalized()
            bi = tan.cross(nrm)
            w = w0 + (w1 - w0) * t
            ring = []
            for k in range(3):
                ang = 2 * math.pi * k / 3
                v = bm.verts.new(p + (nrm * math.cos(ang) + bi * math.sin(ang)) * w)
                uv_of[v] = (k / 3, t)
                ring.append(v)
            if ring_prev is not None:
                for k in range(3):
                    bm.faces.new(
                        (
                            ring_prev[k],
                            ring_prev[(k + 1) % 3],
                            ring[(k + 1) % 3],
                            ring[k],
                        )
                    )
            ring_prev = ring
    for f in bm.faces:
        f.normal_update()
    uv = bm.loops.layers.uv.new("UVTex")
    for f in bm.faces:
        for loop in f.loops:
            loop[uv].uv = uv_of[loop.vert]
    me = bpy.data.meshes.new(name + "_strands")
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new(name + "_strands", me)
    bpy.context.scene.collection.objects.link(obj)
    for m in card.data.materials:
        me.materials.append(m)
    weights = fh.nearest_weights(obj, card, table)
    fh.set_weights(obj, weights)
    mod = obj.modifiers.new("arm", "ARMATURE")
    mod.object = mount.harm
    obj.parent = mount.harm
    obj.matrix_parent_inverse = mount.harm.matrix_world.inverted()
    me.shade_smooth()
    for k in list(card.keys()):
        obj[k] = card[k]
    bpy.data.objects.remove(card)
    obj.name = name
    me.name = name
    return obj


def fine_horse(mount, level, htype):
    """The fine horse at `level` (fitted, slimmed, decimated, coded); returns pieces."""
    lod0 = HORSE_LOD[0]
    pieces = fh.build_horse(
        mount,
        {
            "horse_body": lod0["horse_body"],
            "horse_mane": lod0["horse_mane"],
            "horse_tail": lod0["horse_tail"],
            "horse_eye_l": 60,
            "horse_eye_r": 60,
        },
    )
    for m in list(mount.hmeshes):
        bpy.data.objects.remove(m)
    mount.hmeshes = []
    slim(pieces, mount, HORSE_TYPES[htype])
    horse_materials(pieces)
    body = next(o for o in pieces if o.name == "horse_body")
    ref = None
    if level > 0:
        ref = body.copy()
        ref.data = body.data.copy()
    budget = HORSE_LOD[level]
    out = []
    for o in pieces:
        if o.name.startswith("horse_eye") and not budget["eyes"]:
            bpy.data.objects.remove(o)
            continue
        if o.name in CARDS:
            o = strands(o, level, mount)  # CR4 (no card decimation, no back faces)
        elif level > 0 and o.name in budget:
            decimate_rest(o, budget[o.name])
        out.append(o)
    if ref is not None:
        copy_shade(body, ref)
        bpy.data.objects.remove(ref)
    return out


# --- Harness ------------------------------------------------------------------------------


def _no_legs(table):
    """Weight table without the leg bones (renormalised; the saddle bone if empty)."""
    out = []
    for w in table:
        kept = {k: x for k, x in w.items() if k not in LEG_BONES}
        total = sum(kept.values())
        out.append(
            {k: x / total for k, x in kept.items()}
            if total > 1e-3
            else {cav.SADDLE_BONE: 1.0}
        )
    return out


def _skin_like(obj, body):
    """Weights of the nearest body point (rest pose), leg bones left out."""
    table = fh.nearest_weights(obj, body, fh.weight_table(body))
    fh.set_weights(obj, _no_legs(table))


def _faces_copy(body, keep, offset, name, material):
    """Copy of the body faces for which `keep(...)` holds, with the body's weights.

    `keep(centre, normal, weight table, vertex indices)`; the copy is pushed out by
    `offset` along the vertex normals.
    """
    me = body.data
    table = fh.weight_table(body)
    bm = bmesh.new()
    remap = {}
    weights = []
    for poly in me.polygons:
        if not keep(poly.center, poly.normal, table, poly.vertices):
            continue
        verts = []
        for i in poly.vertices:
            if i not in remap:
                v = me.vertices[i]
                remap[i] = bm.verts.new(v.co + v.normal * offset)
                weights.append(table[i])
            verts.append(remap[i])
        try:
            bm.faces.new(verts)
        except ValueError:
            continue
    obj = eq.to_object(name, bm, [material])
    fh.set_weights(obj, weights)
    return obj


def _dominant(w):
    return max(w.items(), key=lambda kv: kv[1])[0] if w else None


def head_frame(body, mount):
    """(poll, unit axis to the muzzle, unit 'down' towards the jaw, length) at rest."""
    q = _q_joints(mount)
    poll = q["Head"]
    table = fh.weight_table(body)
    pts = [
        v.co
        for v, w in zip(body.data.vertices, table, strict=True)
        if w.get("Head", 0.0) > 0.8
    ]
    muzzle = max(pts, key=lambda p: (p - poll).length)
    axis = (muzzle - poll).normalized()
    down = axis.cross(Vector((1, 0, 0))).normalized()
    if down.z > 0:
        down = -down
    return poll, axis, down, (muzzle - poll).length


def strap(body, name, plane_co, plane_no, width, filt, material, lift=0.006):
    """Leather strap along the section of the body by a plane (ribbon on the surface)."""
    bm = bmesh.new()
    bm.from_mesh(body.data)
    table = fh.weight_table(body)
    doomed = [f for f in bm.faces if not filt(f.calc_center_median(), table, f)]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    res = bmesh.ops.bisect_plane(
        bm,
        geom=list(bm.verts) + list(bm.edges) + list(bm.faces),
        plane_co=plane_co,
        plane_no=plane_no,
    )
    bm.normal_update()
    cut_edges = [g for g in res["geom_cut"] if isinstance(g, bmesh.types.BMEdge)]
    out = bmesh.new()
    n = plane_no.normalized()
    made = {}

    def ribbon(v):
        if v not in made:
            p = v.co + v.normal * lift
            made[v] = (
                out.verts.new(p - n * width / 2),
                out.verts.new(p + n * width / 2),
            )
        return made[v]

    for e in cut_edges:
        a, b = e.verts
        ra, rb = ribbon(a), ribbon(b)
        with contextlib.suppress(ValueError):
            out.faces.new((ra[0], rb[0], rb[1], ra[1]))
    bm.free()
    for f in out.faces:
        c = f.calc_center_median()
        # Outward: away from the nearest body point (FG3: normal computed first).
        f.normal_update()
        if f.normal.dot(c - _nearest(body, c)) < 0:
            f.normal_flip()
    obj = eq.to_object(name, out, [material])
    _skin_like(obj, body)
    return obj


_BVH = {}


def _nearest(body, p):
    key = body.name
    if key not in _BVH:
        me = body.data
        me.calc_loop_triangles()
        _BVH[key] = BVHTree.FromPolygons(
            [v.co for v in me.vertices], [tuple(t.vertices) for t in me.loop_triangles]
        )
    loc, _n, _i, _d = _BVH[key].find_nearest(p)
    return loc


def bridle(body, mount):
    """Headstall (crownpiece and cheekpieces, noseband, throatlatch), bit and reins."""
    leather = bs.material(eq.C_LEATHER, LEATHER_DARK)
    iron = bs.material(eq.C_PLATE, eq.IRON)
    poll, axis, down, length = head_frame(body, mount)

    def head_only(c, table, f):
        return all(
            table[v.index].get("Head", 0) + table[v.index].get("Neck3", 0) > 0.6
            for v in f.verts
        )

    mouth = poll + axis * 0.86 * length + down * 0.02
    top = poll + axis * 0.04 - down * 0.06
    side = Vector((1, 0, 0))
    cheek_no = side.cross(top - mouth).normalized()
    out = [
        strap(
            body, "headstall", (top + mouth) / 2, cheek_no, 0.025, head_only, leather
        ),
        strap(
            body, "noseband", poll + axis * 0.7 * length, axis, 0.03, head_only, leather
        ),
        strap(
            body,
            "throatlatch",
            poll + axis * 0.1 * length,
            (axis - down * 0.6).normalized(),
            0.018,
            head_only,
            leather,
        ),
    ]
    # Bit rings at the corners of the mouth: lateral extremes of the head at `mouth`.
    head_w = fh.weight_table(body)
    pts = [
        v.co
        for v in body.data.vertices
        if head_w[v.index].get("Head", 0.0) > 0.8
        and abs((v.co - mouth).dot(axis)) < 0.03
        and (v.co - mouth).dot(down) > -0.05
    ]
    rings = {}
    for s, sx in (("L", 1), ("R", -1)):
        cand = [p for p in pts if p.x * sx > 0]
        edge = max(cand, key=lambda p: p.x * sx) if cand else mouth
        rings[s] = Vector((edge.x + 0.012 * sx, edge.y, edge.z))
    bm = bmesh.new()
    eq.tube(bm, rings["R"], rings["L"], 0.006, 0.006, 5, 0)
    for s in "LR":
        c = rings[s]
        eq.tube(bm, c - axis * 0.025, c + axis * 0.025, 0.018, 0.018, 8, 0, caps=True)
    eq.finish(bm)
    bit = eq.to_object("bit", bm, [iron])
    eq.bind_rigid(bit, "Head")
    out.append(bit)
    out.append(reins(body, mount, rings, leather))
    return out


def reins(body, mount, rings, leather):
    """Reins from the bit rings to the rider's rein hand (left, above the pommel).

    The hand end rides on the saddle bone (the rider's hand is set in the saddle's frame
    by ``_reins``), the bit end on the head, the middle like the nearest point of the neck.
    """
    hand = mount.pommel + Vector((0.06, -0.12, 0.12))
    bm = bmesh.new()
    params = []
    steps = 10
    for s in "LR":
        a = rings[s]
        b = hand + Vector((0.015 if s == "L" else -0.015, 0.0, -0.01))
        mid = (a + b) / 2 + Vector((0, 0, -0.1))
        prev = None
        for k in range(steps + 1):
            t = k / steps
            p = a * (1 - t) ** 2 + mid * 2 * t * (1 - t) + b * t * t
            if prev is not None:
                before = len(bm.verts)
                eq.tube(bm, prev, p, 0.006, 0.006, 4, 0, caps=False)
                params += [t - 0.5 / steps] * (len(bm.verts) - before)
            prev = p
    eq.finish(bm)
    obj = eq.to_object("reins", bm, [leather])
    table = _no_legs(fh.nearest_weights(obj, body, fh.weight_table(body)))
    for i, t in enumerate(params):
        w = table[i]
        if t < 0.12:
            w = {"Head": 1.0}
        elif t > 0.7:
            f = min((t - 0.7) / 0.2, 1.0)
            w = {k: x * (1 - f) for k, x in w.items()}
            w[cav.SADDLE_BONE] = w.get(cav.SADDLE_BONE, 0.0) + f
        table[i] = w
    fh.set_weights(obj, table)
    return obj


def _arch(bm, y, half, base, height, radius, lean, n, mat):
    """Saddle bow (pommel or cantle): a rounded arch over the back, leaning along y."""
    prev = None
    for k in range(n + 1):
        a = -math.pi / 2 + math.pi * k / n
        z = base + height * abs(math.cos(a)) ** 0.7
        p = Vector((half * math.sin(a), y + lean * (z - base), z))
        if prev is not None:
            eq.tube(bm, prev, p, radius, radius, 6, mat, caps=False)
        prev = p


def saddle(body, mount, level):
    """War saddle and its straps.

    Panels shaped on the back, high cantle, pommel, girth, breast strap, stirrup leathers
    and irons.
    """
    seat = mount.seat
    leather = bs.material(eq.C_LEATHER, LEATHER_DARK)
    seat_mat = bs.material(eq.C_LEATHER, LEATHER_SEAT)
    iron = bs.material(eq.C_PLATE, eq.IRON)
    out = []

    def on_back(c, n, table, vs):
        return (
            abs(c.x) < 0.25
            and seat.y - 0.3 < c.y < seat.y + 0.27
            and n.z > 0.35
            and c.z > seat.z - 0.3
        )

    panels = _faces_copy(body, on_back, 0.03, "saddle_panels", seat_mat)
    out.append(panels)
    bm = bmesh.new()
    back = _nearest(body, Vector((0, seat.y, seat.z))).z + 0.03
    n = (10, 5, 3)[level]
    _arch(bm, seat.y + 0.22, 0.2, back - 0.03, 0.2, 0.03, 0.35, n, 0)  # cantle
    _arch(bm, seat.y - 0.27, 0.17, back - 0.03, 0.15, 0.035, -0.3, n, 0)  # pommel
    if level == 0:
        for s, sx in (("L", 1), ("R", -1)):
            stir = mount.stirrups[s]
            bar = Vector((0.2 * sx, seat.y - 0.1, back - 0.06))
            eye = stir + Vector((0.0, 0.0, 0.12))
            eq.tube(bm, bar, eye, 0.012, 0.012, 4, 1)  # leather
            tread = stir + Vector((0.0, 0.02, -0.07))
            eq.box(bm, tread, (0.03, 0.06, 0.006), mat=2)
            for dy in (-0.05, 0.05):
                eq.tube(
                    bm,
                    tread + Vector((0, dy, 0)),
                    eye + Vector((0, dy * 0.3, 0)),
                    0.006,
                    0.006,
                    4,
                    2,
                )
    eq.finish(bm)
    frame = eq.to_object("saddle", bm, [seat_mat, leather, iron])
    eq.bind_rigid(frame, cav.SADDLE_BONE)
    out.append(frame)
    if level == 0:

        def barrel(c, table, f):
            return abs(c.y - seat.y) < 0.4 and c.z > 0.55

        out.append(
            strap(
                body,
                "girth",
                Vector((0, seat.y - 0.12, 1.0)),
                Vector((0, 1, 0.2)).normalized(),
                0.07,
                barrel,
                leather,
            )
        )

        def chest(c, table, f):
            return c.y < seat.y - 0.3 and c.z > 0.8

        out.append(
            strap(
                body,
                "breast_strap",
                Vector((0, seat.y - 0.6, seat.z - 0.52)),
                Vector((0, -0.25, 1)).normalized(),
                0.05,
                chest,
                leather,
            )
        )
    return out


def chanfron(body, mount):
    """Chanfron: steel plate over the brow and the face, eye holes left open."""
    poll, axis, down, length = head_frame(body, mount)
    steel = bs.material(eq.C_PLATE, BARD_STEEL)

    def keep(c, n, table, vs):
        if any(table[i].get("Head", 0) < 0.7 for i in vs):
            return False
        s = (c - poll).dot(axis) / length
        return 0.08 < s < 0.82 and n.dot(-down) > 0.3

    obj = _faces_copy(body, keep, 0.012, "chanfron", steel)
    obj.vertex_groups.clear()
    eq.bind_rigid(obj, "Head")
    return obj


def flanchards(body, mount):
    """Flanchards: steel plates on both flanks behind the saddle."""
    seat = mount.seat
    steel = bs.material(eq.C_PLATE, BARD_STEEL)

    def keep(c, n, table, vs):
        legs = sum(table[i].get(b, 0) for i in vs for b in LEG_BONES) / len(vs)
        return (
            abs(n.x) > 0.45
            and seat.y + 0.12 < c.y < seat.y + 0.55
            and seat.z - 0.72 < c.z < seat.z - 0.22
            and legs < 0.3
        )

    return _faces_copy(body, keep, 0.02, "flanchards", steel)


TORSO = fh.TORSO


def _leg_anchor(table, pts_all, bone):
    """Mean rest position of the body vertices dominated by `bone` (None if absent)."""
    sel = [p for p, w in zip(pts_all, table, strict=True) if _dominant(w) == bone]
    if not sel:
        return None
    return sum(sel, Vector()) / len(sel)


def caparison(body, mount, level, hem=0.46, dagged=False):
    """Two-piece trapper in livery, parted at the saddle, armorial over the whole cloth.

    CR3: heavy cloth rather than a rigid ring: the flanks hang from the back in broad,
    irregular organ-pipe folds that deepen and flare towards the hem; the hem waves around
    the hocks (optionally dagged at LOD0); slits over the fore and hind legs, the panel in
    front of the fore slit and behind the hind slit partly carried by the upper leg so the
    cloth parts when the horse strides. The arms cover the whole cloth (charges ~22 cm),
    not a shield pasted on each flank. Vertices: the top arc over the back takes a third
    of the ring, each flank a third (the old ring spent two thirds on the back).
    """
    stations = (26, 10, 5)[level]
    around = (24, 10, 6)[level]
    table = fh.weight_table(body)
    body_pts = [v.co.copy() for v in body.data.vertices]
    pts = [
        (p, w) for p, w in zip(body_pts, table, strict=True) if _dominant(w) in TORSO
    ]
    ys = [p.y for p, _w in pts]
    y0, y1 = min(ys) + 0.12, max(ys) - 0.1
    flank_n = max(2, round(around / 3))
    top_n = around - 2 * flank_n
    # Slits over the legs (side, station, leg bone); the side of a leg from its mean x.
    slits = []
    for fore, bone in ((True, "FrontUpperLeg"), (False, "BackUpperLeg")):
        for s_ in "LR":
            c = _leg_anchor(table, body_pts, f"{bone}.{s_}")
            if c is None:
                continue
            k = round((c.y - y0) / (y1 - y0) * stations)
            if 1 <= k <= stations - 1:
                slits.append((1.0 if c.x > 0 else -1.0, k, fore, f"{bone}.{s_}"))

    def fold(u, sx):
        # Irregular broad folds (heavy wool), not mirrored between the flanks.
        ph = 0.0 if sx > 0 else 2.1
        return 0.7 * math.sin(u * math.pi * 12.5 + 0.7 + ph) + 0.3 * math.sin(
            u * math.pi * 29.0 + 1.9 + 1.7 * ph
        )

    def hem_z(u, sx):
        ph = 0.0 if sx > 0 else 1.3
        z = hem + 0.022 * math.sin(u * math.pi * 5.0 + 1.1 + ph)
        return z + 0.012 * math.sin(u * math.pi * 17.0 + ph)

    bm = bmesh.new()
    rings = []
    ring_uv = []
    for k in range(stations + 1):
        u = k / stations
        y = y0 + (y1 - y0) * u
        near = [p for p, _w in pts if abs(p.y - y) < 0.08]
        half = max((abs(p.x) for p in near), default=0.3) + 0.03
        top = max((p.z for p in near), default=1.4) + 0.035
        shoulder = top - 0.33
        ring = []
        uvs = []
        for j in range(around + 1):
            if flank_n <= j <= around - flank_n:
                # Top arc over the back (30° to 150° as before).
                a = math.radians(30 + 120 * (j - flank_n) / max(top_n, 1))
                side, lift = math.cos(a), math.sin(a)
                z = top - (1 - lift) * 0.33
                x = half * side * 1.04
                f = 1.0
                sx = 1.0 if side > 0 else -1.0
            else:
                sx = 1.0 if j < flank_n else -1.0
                f = (j if j < flank_n else around - j) / flank_n  # 1 at the back, 0 hem
                hz = hem_z(u, sx)
                if dagged and level == 0 and j in (0, around) and k % 2 == 1:
                    hz -= 0.055  # dags: pointed tongues at every other station
                g = (1 - f) ** 1.3
                z = hz + (shoulder - hz) * f
                depth = (0.05 if level == 0 else 0.03 if level == 1 else 0.0) * g
                x = sx * (half * (1.02 + 0.1 * (1 - f) ** 1.5) + depth * fold(u, sx))
            ring.append(bm.verts.new(Vector((x, y, z))))
            uvs.append((u, 0.0, f, sx))
        rings.append(ring)
        ring_uv.append(uvs)
    # Distance along each ring from the spine (UV v), before any vertex is deleted.
    spine = [[0.0] * (around + 1) for _k in range(stations + 1)]
    mid = around // 2
    for k in range(stations + 1):
        for step in (1, -1):
            acc = 0.0
            j = mid
            while 0 <= j + step <= around:
                acc += (rings[k][j + step].co - rings[k][j].co).length
                j += step
                spine[k][j] = acc
    gap = (mount.seat.y - 0.24, mount.seat.y + 0.2)
    # Slit copies: at a slit station, the lower flank rows of the faces behind (fore slit)
    # or in front (hind slit) use separate vertices, 1 cm outside (panels overlap).
    slit_rows = {}
    for sx, k, fore, _bone in slits:
        for j in range(around + 1):
            f = ring_uv[k][j][2]
            if (
                ring_uv[k][j][3] != sx
                or f > 0.55
                or not (j < flank_n or j > around - flank_n)
            ):
                continue
            co = rings[k][j].co
            slit_rows[(k, j, fore)] = bm.verts.new(co + Vector((0.01 * sx, 0, 0)))
    carried = {}  # vertex -> (leg bone, share)

    def vert(k, j, face_k):
        v = rings[k][j]
        for fore in (True, False):
            alt = slit_rows.get((k, j, fore))
            if alt is None:
                continue
            # Fore slit: faces behind it (face_k >= k) take the copy; hind: faces in front.
            if (fore and face_k >= k) or (not fore and face_k < k):
                return alt
        return v

    faces_uv = []
    for k in range(stations):
        ra_y = (rings[k][0].co.y + rings[k + 1][0].co.y) / 2
        if gap[0] < ra_y < gap[1]:
            continue
        for j in range(around):
            quad = (
                vert(k, j, k),
                vert(k, j + 1, k),
                vert(k + 1, j + 1, k),
                vert(k + 1, j, k),
            )
            f = bm.faces.new(quad)
            faces_uv.append((f, ((k, j), (k, j + 1), (k + 1, j + 1), (k + 1, j))))
    # Leg share: in front of the fore slit / behind the hind slit, lower flank only.
    for sx, k_s, fore, bone in slits:
        for k in range(stations + 1):
            if (fore and k > k_s) or (not fore and k < k_s):
                continue
            reach = abs(k - k_s) / max(stations * 0.25, 1)
            for j in range(around + 1):
                uvj = ring_uv[k][j]
                if (
                    uvj[3] != sx
                    or uvj[2] > 0.6
                    or not (j < flank_n or j > around - flank_n)
                ):
                    continue
                share = 0.45 * (1 - uvj[2] / 0.6) ** 1.2 * max(0.0, 1 - reach)
                if share <= 0.02:
                    continue
                v = rings[k][j]
                carried[v] = (bone, share)
    bmesh.ops.delete(
        bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
    )
    for f in bm.faces:
        c = f.calc_center_median()
        f.normal_update()  # FG3: new faces carry no normal until updated
        if f.normal.dot(c - Vector((0, c.y, 1.0))) < 0:
            f.normal_flip()
    uv = bm.loops.layers.uv.new("UVMap")
    # Whole-cloth arms, one scale both ways (`cloth_m` of cloth per unit, charges ~22 cm):
    # u along the body, mirrored on the right flank so the charges face the head on both
    # sides (seam at the spine: the side is the face's, not the vertex's); v = distance
    # along the ring from the spine, 0.04 at the spine.
    cloth_m = 1.5
    ymid = (y0 + y1) / 2
    for f, keys in faces_uv:
        if not f.is_valid:
            continue
        flip = f.calc_center_median().x < 0
        # (loops looked up by vertex: `normal_flip` reverses the loop order)
        at = {vert(k, j, keys[0][0]): (k, j) for k, j in keys}
        for loop in f.loops:
            k, j = at[loop.vert]
            du = (loop.vert.co.y - ymid) / cloth_m
            loop[uv].uv = (0.5 - du if flip else 0.5 + du, 0.04 + spine[k][j] / cloth_m)
    carried_idx = {}
    bm.verts.index_update()
    for v, val in carried.items():
        if v.is_valid:
            carried_idx[v.index] = val
    obj = eq.to_object("caparison", bm, [bs.material(eq.C_ARMS, (1, 1, 1))])
    tree = KDTree(len(pts))
    for i, (p, _w) in enumerate(pts):
        tree.insert(Vector((p.x * 0.6, p.y, p.z)), i)
    tree.balance()
    weights = []
    for v in obj.data.vertices:
        p = v.co
        _co, idx, _d = tree.find(Vector((p.x * 0.6, p.y, max(p.z, 1.0))))
        w = dict(pts[idx][1])
        leg = carried_idx.get(v.index)
        if leg is not None:
            bone, share = leg
            w = {b: x * (1 - share) for b, x in w.items()}
            w[bone] = w.get(bone, 0.0) + share
        weights.append(w)
    fh.set_weights(obj, weights)
    return obj


def hidden_faces(body, cover, max_dist=0.25):
    """Body faces hidden under `cover` (trunk faces whose outward ray meets it)."""
    me = cover.data
    me.calc_loop_triangles()
    bvh = BVHTree.FromPolygons(
        [v.co for v in me.vertices], [tuple(t.vertices) for t in me.loop_triangles]
    )
    table = fh.weight_table(body)
    out = []
    for poly in body.data.polygons:
        # Upper legs swing out from under the cloth: keep them, and the lower flanks.
        legs = sum(table[i].get(b, 0) for i in poly.vertices for b in LEG_BONES)
        if legs / len(poly.vertices) > 0.3 or poly.center.z < 0.85:
            continue
        n = poly.normal
        if n.z < -0.6:
            continue
        hit = bvh.ray_cast(poly.center + n * 0.004, n, max_dist)
        if hit[0] is not None:
            out.append(poly.index)
    return out


def set_mask(obj, mask, faces=None):
    """Variant mask on `faces` of `obj` (all when None)."""
    me = obj.data
    attr = me.attributes.get("vmask") or me.attributes.new("vmask", "INT", "FACE")
    for i in faces if faces is not None else range(len(me.polygons)):
        attr.data[i].value = mask


def harness(recipe, body, mount, level):
    """Harness and bards of `recipe` on the fine horse: [(object, variant mask)]."""
    out = []
    for obj in saddle(body, mount, level):
        out.append((obj, 0))
    if level == 0:
        for obj in bridle(body, mount):
            out.append((obj, 0))
    for item in recipe.get("horse_equipment", []):
        name, mask = item[0], item[1]
        kwargs = item[2] if len(item) > 2 else {}
        if name == "caparison":
            out.append((caparison(body, mount, level, **kwargs), mask))
        elif name == "chanfron":
            out.append((chanfron(body, mount), mask))
        elif name == "flanchards":
            out.append((flanchards(body, mount), mask))
    return out


def attach(objs, mount):
    """Armature modifier on the horse armature (previews, pose renders)."""
    for o in objs:
        if any(m.type == "ARMATURE" for m in o.modifiers):
            continue
        mod = o.modifiers.new("arm", "ARMATURE")
        mod.object = mount.harm
        o.parent = mount.harm
        o.matrix_parent_inverse = mount.harm.matrix_world.inverted()


def horse_and_harness(fig_name, recipe, level, mount):
    """Fine horse, harness and bards of a mounted figure at `level`, on `mount`.

    `mount` is the ``battle_skinned_cavalry`` mount just built (its Quaternius horse meshes
    are replaced). Returns the objects, variant masks set, tagged ``fg4``.
    """
    _BVH.clear()
    horse = fine_horse(mount, level, HORSE_OF[fig_name])
    body = next(o for o in horse if o.name == "horse_body")
    for o in horse:
        set_mask(o, 0)
    kit = harness(recipe, body, mount, level)
    variants = recipe.get("variants", 1)
    everyone = (1 << variants) - 1
    for obj, mask in kit:
        set_mask(obj, mask)
        if obj.name == "caparison":
            faces = hidden_faces(body, obj)
            print(
                f"HIDDEN {fig_name} LOD{level} {len(faces)} faces under the caparison"
            )
            if mask == 0:
                _delete_faces(body, faces)
            else:
                set_mask(body, everyone & ~mask, faces)
    out = horse + [o for o, _m in kit]
    for o in out:
        o["fg4"] = True
    print(
        f"HORSE {fig_name} LOD{level} {HORSE_OF[fig_name]} horse={_tris(horse)} "
        f"harness={_tris(out) - _tris(horse)}"
    )
    return out


def build_figure(fig_name, level):
    """Whole fine figure (FG1 rider, fine horse): (mount, rider, horse, harness)."""
    import battle_fine_figures as ff

    _arm, objs, _recipe = ff.build_figure(fig_name, level)
    mount = poses.RIDE["mount"]
    mine = [o for o in objs if o.get("fg4")]
    rider = [o for o in objs if not o.get("fg4")]
    horse = [o for o in mine if o.name.startswith("horse_")]
    extra = [o for o in mine if not o.name.startswith("horse_")]
    attach(mine, mount)
    return mount, rider, horse, extra


def _delete_faces(obj, faces):
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.faces.ensure_lookup_table()
    bmesh.ops.delete(bm, geom=[bm.faces[i] for i in faces], context="FACES_ONLY")
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def _tris(objs):
    total = 0
    for o in objs:
        o.data.calc_loop_triangles()
        total += len(o.data.loop_triangles)
    return total


def main():
    """Check renders (``--render DIR``) of the fine mounted figures."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    names = MOUNTED
    if "--only" in args:
        names = [n for n in args[args.index("--only") + 1].split(",") if n in MOUNTED]
    if "--render" in args:
        import fg4_render

        fg4_render.render_all(args[args.index("--render") + 1], names)
    print("OK")


if __name__ == "__main__":
    main()
