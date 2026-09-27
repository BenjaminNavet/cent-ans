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
        if level > 0 and o.name in budget:
            decimate_rest(o, budget[o.name])
        if o.name in CARDS:
            double_sided(o)
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


def caparison(body, mount, level, hem=0.48):
    """Two-piece trapper in livery, parted at the saddle (FG0 shape), arms on the flanks."""
    stations = (26, 10, 5)[level]
    around = (24, 10, 6)[level]
    table = fh.weight_table(body)
    pts = [
        (v.co.copy(), w)
        for v, w in zip(body.data.vertices, table, strict=True)
        if _dominant(w) in TORSO
    ]
    ys = [p.y for p, _w in pts]
    y0, y1 = min(ys) + 0.12, max(ys) - 0.1
    bm = bmesh.new()
    rings = []
    for k in range(stations + 1):
        u = k / stations
        y = y0 + (y1 - y0) * u
        near = [p for p, _w in pts if abs(p.y - y) < 0.08]
        half = max((abs(p.x) for p in near), default=0.3) + 0.03
        top = max((p.z for p in near), default=1.4) + 0.035
        ring = []
        for j in range(around + 1):
            a = math.pi * j / around
            side = math.cos(a)
            lift = math.sin(a)
            if lift > 0.5:
                z = top - (1 - lift) * 0.33
                x = half * side * 1.04
            else:
                f = lift / 0.5
                z = hem + (top - 0.33 - hem) * f
                sx = 1.0 if side > 0 else -1.0
                fold = (
                    (0.022 if level == 0 else 0.0)
                    * (1 - f)
                    * math.sin(u * math.pi * 11 + 0.7)
                )
                x = sx * (half * (1.02 + 0.07 * (1 - f)) + fold)
            ring.append(bm.verts.new(Vector((x, y, z))))
        rings.append(ring)
    gap = (mount.seat.y - 0.24, mount.seat.y + 0.2)
    for ra, rb in zip(rings, rings[1:], strict=False):
        if gap[0] < (ra[0].co.y + rb[0].co.y) / 2 < gap[1]:
            continue
        for j in range(around):
            bm.faces.new((ra[j], ra[j + 1], rb[j + 1], rb[j]))
    bmesh.ops.delete(
        bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
    )
    for f in bm.faces:
        c = f.calc_center_median()
        f.normal_update()  # FG3: new faces carry no normal until updated
        if f.normal.dot(c - Vector((0, c.y, 1.0))) < 0:
            f.normal_flip()
    uv = bm.loops.layers.uv.new("UVMap")
    ymid = (y0 + y1) / 2
    zmid = hem + 0.45
    for f in bm.faces:
        for loop in f.loops:
            p = loop.vert.co
            uu = (p.y - ymid) / 0.75 + 0.5
            loop[uv].uv = (uu if p.x > 0 else 1 - uu, 0.5 - (p.z - zmid) / 0.75)
    obj = eq.to_object("caparison", bm, [bs.material(eq.C_ARMS, (1, 1, 1))])
    tree = KDTree(len(pts))
    for i, (p, _w) in enumerate(pts):
        tree.insert(Vector((p.x * 0.6, p.y, p.z)), i)
    tree.balance()
    weights = []
    for v in obj.data.vertices:
        p = v.co
        _co, idx, _d = tree.find(Vector((p.x * 0.6, p.y, max(p.z, 1.0))))
        weights.append(dict(pts[idx][1]))
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
    for name, mask in recipe.get("horse_equipment", []):
        if name == "caparison":
            out.append((caparison(body, mount, level), mask))
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
