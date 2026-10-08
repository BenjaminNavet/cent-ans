"""Lots FG0/FG4: finer horse on the ``cavalry`` rig (Quaternius horse bones kept).

Source: "Rigged Horse" by Lyndon Daniels (OpenGameArt, CC0), a textured draught horse
(7 400 triangles of body, 3 900 of mane, 1 750 of tail, 2k colour / normal / AO maps). Its own
19-bone rig is dropped.

Fit (FG4, replaces the FG0 RBF warp):
1. Every piece is baked to world space (natural standing shape), scaled uniformly so the
   back at the saddle matches the Quaternius horse, centred on its legs and narrowed to its
   barrel (``similarity``).
2. The anatomical joints of the new horse (``OGA_JOINTS``) are mapped joint to joint onto
   the Quaternius bones, one affine map per bone (``fit_transforms``).
3. Weights are computed in the natural stance: body by bone heat on an armature whose bones
   run between the new horse's own joints, mane from the nearest body point, tail along
   the tail chain, eyes rigid on ``Head``; the blended bone maps then move every vertex to
   the rest pose (``apply_fit``): the bends sit on the clips' pivots.
"""

import math
import os

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree
from mathutils.geometry import barycentric_transform

OGA_URL = "https://opengameart.org/sites/default/files/riggedHorse.blend"
OGA_BLEND = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..",
    "..",
    "game",
    "assets",
    "third_party",
    "animals",
    "oga_rigged_horse",
    "riggedHorse.blend",
)
PIECES = {
    "Plane": "horse_body",
    "BezierCurve": "horse_mane",
    "BezierCurve.005": "horse_tail",
    "Sphere": "horse_eye_l",
    "Sphere.002": "horse_eye_r",
}


def fetch_oga():
    """Download the CC0 source (20 MB, kept out of git) if it is missing."""
    if os.path.exists(OGA_BLEND):
        return
    import urllib.request

    os.makedirs(os.path.dirname(OGA_BLEND), exist_ok=True)
    urllib.request.urlretrieve(OGA_URL, OGA_BLEND)


def load_oga():
    """Append the CC0 horse pieces, baked to world space, unparented; returns {name: obj}."""
    fetch_oga()
    with bpy.data.libraries.load(OGA_BLEND, link=False) as (src, dst):
        dst.objects = [n for n in src.objects if n in PIECES or n == "Armature"]
        # Legacy (2.6) materials lose their texture slots: append the packed images.
        dst.images = list(src.images)
    out = {}
    for o in dst.objects:
        bpy.context.scene.collection.objects.link(o)
    bpy.context.view_layer.update()
    for o in dst.objects:
        if o.type != "MESH":
            continue
        mw = o.matrix_world.copy()
        o.modifiers.clear()
        o.parent = None
        o.data.transform(mw)
        o.matrix_world.identity()
        o.vertex_groups.clear()
        o.name = o.data.name = PIECES.get(o.name, o.name)
        out[o.name] = o
    for o in dst.objects:
        if o.type == "ARMATURE":
            bpy.data.objects.remove(o)
    return out


def _verts(obj):
    return [obj.matrix_world @ v.co for v in obj.data.vertices]


def _back_top(pts, y, half=0.1):
    zs = [p.z for p in pts if abs(p.y - y) < half and abs(p.x) < 0.12]
    return max(zs) if zs else 0.0


def _slice_centroid(pts, z, around, radius=0.16, dz=0.02):
    sel = [
        p
        for p in pts
        if abs(p.z - z) < dz and math.hypot(p.x - around.x, p.y - around.y) < radius
    ]
    if not sel:
        return None
    return sum(sel, Vector()) / len(sel)


def similarity(pieces, mount):
    """Uniform scale + translation of the new horse onto the Quaternius one."""
    body = pieces["horse_body"]
    q_pts = [p for m in mount.hmeshes for p in _verts(m)]
    pts = _verts(body)
    # 1. Hooves on the ground, overall height of the Quaternius horse (rough metres).
    ground = min(p.z for p in pts)
    s0 = _height(q_pts) / _height(pts)
    T = Matrix.Scale(s0, 4) @ Matrix.Translation((0, 0, -ground))
    for o in pieces.values():
        o.data.transform(T)
    pts = _verts(body)
    # 2. Legs: clusters of the vertices at 30 cm; centre them on the Quaternius legs.
    low = [p for p in pts if abs(p.z - 0.3) < 0.04]
    y_mid = sum(p.y for p in low) / len(low)
    legs = []
    for sx, front in ((1, True), (-1, True), (1, False), (-1, False)):
        sel = [p for p in low if (p.x > 0) == (sx > 0) and (p.y < y_mid) == front]
        legs.append(sum(sel, Vector()) / len(sel))
    src_mid_y = sum(v.y for v in legs) / 4
    q_mid_y = _q_leg_mid(mount).y
    # 3. Back height at the saddle equal to the Quaternius horse's (the rider's seat).
    q_back = _back_top(q_pts, mount.seat.y)
    back = _back_top(pts, src_mid_y + (mount.seat.y - q_mid_y), half=0.15)
    s1 = q_back / back
    T = (
        Matrix.Translation((0, q_mid_y, 0))
        @ Matrix.Scale(s1, 4)
        @ Matrix.Translation((0, -src_mid_y, 0))
    )
    for o in pieces.values():
        o.data.transform(T)
    # 4. Barrel width: the rider's stirrups (fixed by ``Mount``) are set for the slimmer
    # Quaternius horse; a draught barrel would swallow his legs. Narrow the whole horse.
    pts = _verts(body)

    def half_width(ps):
        band = [p for p in ps if abs(p.y - mount.seat.y) < 0.15 and 1.0 < p.z < 1.35]
        return max(abs(p.x) for p in band)

    sx = min(1.0, half_width(q_pts) * 1.08 / half_width(pts))
    for o in pieces.values():
        o.data.transform(Matrix.Diagonal((sx, 1.0, 1.0, 1.0)))
    print(f"HORSE scale={s0 * s1:.4f} back={q_back:.3f} width={sx:.3f}")
    return s0 * s1


def _height(pts):
    return max(p.z for p in pts) - min(p.z for p in pts)


def _q_leg_mid(mount):
    arm = mount.harm
    ps = [
        arm.matrix_world @ arm.data.bones[f"{j}.{s}"].head_local
        for j in ("FrontLowerLeg", "BackLowerLeg")
        for s in "LR"
    ]
    return sum(ps, Vector()) / 4


# --- Joint-to-joint fit (lot FG4) ---------------------------------------------------------
#
# The Quaternius horse stands "camped out": fore legs straight under the point of the
# shoulder, hocks far behind the croup, neck upright. The CC0 horse stands naturally. Rather
# than warp the mesh (FG0: a Gaussian RBF, which put the bends away from the pivots and
# twisted hocks and pasterns at the gallop), the new horse is *posed* onto the Quaternius
# bones: its own anatomical joints (read on the mesh, table below) are mapped joint to joint
# onto the Quaternius ones by one affine transform per bone (rotation about the joint +
# stretch along the segment), blended with the skin weights computed in the natural stance.
# Joints then sit exactly on the pivots of the ``cavalry`` clips, and the head, carried by a
# single rigid transform, keeps its shape.

# Anatomical joints of the CC0 horse after ``similarity`` (left side; metres, y forwards is
# negative): read on orthographic side views with a 5 cm grid (docs/archive/chantiers.md).
# Leg joints outside the body take their x from the mesh slice at their height.
OGA_JOINTS = {
    "FrontUpperLeg": (None, -0.60, 1.02),  # point of the shoulder
    "FrontLowerLeg": ("slice", -0.32, 0.51),  # knee (carpus)
    "IKFrontLeg": ("slice", -0.26, 0.18),  # fetlock
    "FF": ("slice", -0.27, 0.063),  # hoof
    "BackUpperLeg": (None, 0.52, 0.95),  # stifle
    "BackLowerLeg": ("slice", 0.69, 0.57),  # hock
    "IKBackLeg": ("slice", 0.61, 0.19),  # hind fetlock
    "FFB": ("slice", 0.50, 0.063),  # hind hoof
    "Neck2": (0.0, -0.711, 1.360),
    "Neck3": (0.0, -0.921, 1.487),
    "Head": (0.0, -1.14, 1.62),  # poll
}

# Bone -> the joint its segment runs to (the bone's region of the skin).
SEGMENT_TO = {
    "Back": "Torso",
    "Torso": "Torso2",
    "Torso2": "Torso3",
    "Torso3": "Neck1",
    "Neck1": "Neck2",
    "Neck2": "Neck3",
    "Neck3": "Head",
}
for _s in "LR":
    SEGMENT_TO.update(
        {
            f"FrontShoulder.{_s}": f"FrontUpperLeg.{_s}",
            f"FrontUpperLeg.{_s}": f"FrontLowerLeg.{_s}",
            f"FrontLowerLeg.{_s}": f"IKFrontLeg.{_s}",
            f"IKFrontLeg.{_s}": f"FF.{_s}",
            f"BackShoulder.{_s}": f"BackLeg.{_s}",
            f"BackLeg.{_s}": f"BackUpperLeg.{_s}",
            f"BackUpperLeg.{_s}": f"BackLowerLeg.{_s}",
            f"BackLowerLeg.{_s}": f"IKBackLeg.{_s}",
            f"IKBackLeg.{_s}": f"FFB.{_s}",
        }
    )
for _k in range(1, 7):
    SEGMENT_TO[f"Tail{_k}"] = f"Tail{_k + 1}"

# Bones whose transform is the identity (trunk, girdles, tail: already aligned by
# ``similarity``); the others are fitted joint to joint.
RIGID_BONES = {"Back", "Torso", "Torso2", "Torso3", "Neck1"} | {
    f"{b}.{s}" for b in ("FrontShoulder", "BackShoulder", "BackLeg") for s in "LR"
}
# Skin of the body: every horse bone but the ears (rigid on the head) and the tail.
TAIL_BONES = [f"Tail{k}" for k in range(1, 8)]
BODY_BONES = (set(SEGMENT_TO) - set(TAIL_BONES)) | {
    "Head",
    "FF.L",
    "FF.R",
    "FFB.L",
    "FFB.R",
}


def _slice_x(pts, y, z, dz=0.025, dy=0.14):
    sel = [p.x for p in pts if p.x > 0 and abs(p.z - z) < dz and abs(p.y - y) < dy]
    return sum(sel) / len(sel) if sel else None


def joints(pieces, mount):
    """(natural-stance joints, Quaternius rest joints) of every horse bone, world space."""
    arm = mount.harm
    q = {b.name: arm.matrix_world @ b.head_local for b in arm.data.bones}
    pts = _verts(pieces["horse_body"])
    oga = dict(q)
    for base, (xs, y, z) in OGA_JOINTS.items():
        names = (
            [base] if base.startswith(("Neck", "Head")) else [f"{base}.L", f"{base}.R"]
        )
        for name in names:
            sx = -1.0 if name.endswith(".R") else 1.0
            if xs == "slice":
                x = _slice_x(pts, y, max(z, 0.1)) or abs(q[name].x)
            elif xs is None:
                x = abs(q[name].x)
            else:
                x = xs
            oga[name] = Vector((x * sx, y, z))
    # Hoof tips (segment ends of the leaf bones): 8 cm ahead, 6 cm down.
    for s in "LR":
        for hoof in (f"FF.{s}", f"FFB.{s}"):
            oga[hoof + ">"] = oga[hoof] + Vector((0, -0.08, -0.05))
            q[hoof + ">"] = q[hoof] + Vector((0, -0.08, -0.05))
    # Head: from the poll to the muzzle (same length on both, rotated with the neck).
    return oga, q


def _rotation_between(a, b):
    """Rotation (3x3) taking direction `a` onto direction `b` (shortest arc)."""
    return a.normalized().rotation_difference(b.normalized()).to_matrix()


def fit_transforms(oga, q):
    """Affine map (4x4) per bone from the natural stance onto the Quaternius rest pose."""
    out = {}
    rots = {}
    for name in oga:
        if name.endswith(">") or name not in q:
            continue
        child = SEGMENT_TO.get(name)
        if name in RIGID_BONES or name.startswith(("Tail", "Ear", "Pole", "Body")):
            out[name] = Matrix.Identity(4)
            continue
        if child is None:
            continue
        d_o = oga[child] - oga[name]
        d_q = q[child] - q[name]
        rot = _rotation_between(d_o, d_q)
        rots[name] = rot
        axis = d_o.normalized()
        s = max(0.6, min(1.6, d_q.length / max(d_o.length, 1e-6)))
        stretch = Matrix.Identity(3) + (s - 1.0) * Matrix(
            [[axis[i] * axis[j] for j in range(3)] for i in range(3)]
        )
        lin = (rot @ stretch).to_4x4()
        out[name] = Matrix.Translation(q[name]) @ lin @ Matrix.Translation(-oga[name])
    # Leaves: the head turns with the last neck segment; hooves stay level (translation).
    rot = rots["Neck3"].to_4x4()
    out["Head"] = Matrix.Translation(q["Head"]) @ rot @ Matrix.Translation(-oga["Head"])
    for s in "LR":
        for hoof in (f"FF.{s}", f"FFB.{s}"):
            out[hoof] = Matrix.Translation(q[hoof] - oga[hoof])
    return out


def heat_armature(oga, bones):
    """Armature with a bone per skin region, at the natural-stance joints (for bone heat)."""
    data = bpy.data.armatures.new("fg_heat")
    arm = bpy.data.objects.new("fg_heat", data)
    bpy.context.scene.collection.objects.link(arm)
    with bpy.context.temp_override(
        active_object=arm, object=arm, selected_objects=[arm]
    ):
        bpy.context.view_layer.objects.active = arm
        bpy.ops.object.mode_set(mode="EDIT")
        for name in bones:
            eb = data.edit_bones.new(name)
            eb.head = oga[name]
            child = SEGMENT_TO.get(name)
            if child is not None:
                eb.tail = oga[child]
            elif name == "Head":
                eb.tail = oga[name] + Vector((0, -0.42, -0.38))
            else:
                eb.tail = oga[name + ">"]
            if (eb.tail - eb.head).length < 0.02:
                eb.tail = eb.head + Vector((0, 0, 0.05))
        bpy.ops.object.mode_set(mode="OBJECT")
    return arm


def heat_weights(obj, arm):
    """Bone-heat weights of `obj` from `arm` (Blender's automatic weights)."""
    obj.vertex_groups.clear()
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    obj.select_set(True)
    arm.select_set(True)
    bpy.context.view_layer.objects.active = arm
    with bpy.context.temp_override(
        active_object=arm,
        object=arm,
        selected_objects=[obj, arm],
        selected_editable_objects=[obj, arm],
    ):
        bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    mw = obj.matrix_world.copy()
    obj.parent = None
    obj.matrix_world = mw
    obj.modifiers.clear()


def weight_table(obj):
    """[{bone: weight}] per vertex."""
    names = {g.index: g.name for g in obj.vertex_groups}
    return [
        {names[g.group]: g.weight for g in v.groups if g.weight > 1e-4}
        for v in obj.data.vertices
    ]


def set_weights(obj, table, influences=4):
    """Replace the vertex groups of `obj` by `table` (top `influences`, normalised)."""
    obj.vertex_groups.clear()
    groups = {}
    for i, w in enumerate(table):
        top = sorted(w.items(), key=lambda kv: -kv[1])[:influences]
        total = sum(x for _k, x in top) or 1.0
        for k, x in top:
            if k not in groups:
                groups[k] = obj.vertex_groups.new(name=k)
            groups[k].add([i], x / total, "REPLACE")


def smooth_weights(obj, table, passes=2, only=None):
    """Laplacian relaxation of the weights along the mesh edges.

    `only`: vertex indices to relax (others are kept, but still feed their neighbours).
    """
    adj = [[] for _ in obj.data.vertices]
    for e in obj.data.edges:
        i, j = e.vertices
        adj[i].append(j)
        adj[j].append(i)
    for _ in range(passes):
        new = []
        for i, w in enumerate(table):
            if not adj[i] or (only is not None and i not in only):
                new.append(w)
                continue
            acc = {k: v * 0.5 for k, v in w.items()}
            share = 0.5 / len(adj[i])
            for j in adj[i]:
                for k, v in table[j].items():
                    acc[k] = acc.get(k, 0.0) + v * share
            new.append(acc)
        table = new
    return table


def nearest_weights(obj, source, source_table):
    """Weights of the nearest point of `source` (barycentric over its triangle)."""
    me = source.data
    me.calc_loop_triangles()
    verts = [source.matrix_world @ v.co for v in me.vertices]
    tris = [tuple(t.vertices) for t in me.loop_triangles]
    bvh = BVHTree.FromPolygons(verts, tris)
    out = []
    for v in obj.data.vertices:
        loc, _n, ti, _d = bvh.find_nearest(obj.matrix_world @ v.co)
        a, b, c = tris[ti]
        bary = barycentric_transform(
            loc,
            verts[a],
            verts[b],
            verts[c],
            Vector((1, 0, 0)),
            Vector((0, 1, 0)),
            Vector((0, 0, 1)),
        )
        acc = {}
        for idx, f in zip((a, b, c), bary, strict=True):
            for k, w in source_table[idx].items():
                acc[k] = acc.get(k, 0.0) + w * max(f, 0.0)
        out.append(acc)
    return out


def chain_weights(obj, chain, blend=0.3):
    """Weights along a bone chain [(bone, head, tail)]: nearest segment, blended at joints."""
    out = []
    for v in obj.data.vertices:
        p = obj.matrix_world @ v.co
        best = None
        for k, (_name, a, b) in enumerate(chain):
            ab = b - a
            t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.length_squared, 1e-9)))
            d = (a + ab * t - p).length
            if best is None or d < best[0]:
                best = (d, k, t)
        _d, k, t = best
        w = {chain[k][0]: 1.0}
        if t > 1 - blend and k + 1 < len(chain):
            f = 0.5 * (t - (1 - blend)) / blend
            w = {chain[k][0]: 1 - f, chain[k + 1][0]: f}
        elif t < blend and k > 0:
            f = 0.5 * (blend - t) / blend
            w = {chain[k][0]: 1 - f, chain[k - 1][0]: f}
        out.append(w)
    return out


def apply_fit(obj, table, fit):
    """Move every vertex from the natural stance to the rest pose (blended bone maps)."""
    n = len(obj.data.vertices)
    co = np.empty(n * 3)
    obj.data.vertices.foreach_get("co", co)
    co = co.reshape(n, 3)
    mats = {k: np.array(m) for k, m in fit.items()}
    out = np.zeros_like(co)
    hom = np.concatenate([co, np.ones((n, 1))], axis=1)
    for i, w in enumerate(table):
        total = 0.0
        acc = np.zeros(3)
        for k, x in w.items():
            m = mats.get(k)
            if m is None:
                continue
            acc += x * (m @ hom[i])[:3]
            total += x
        out[i] = acc / total if total > 0 else co[i]
    obj.data.vertices.foreach_set("co", out.ravel())
    obj.data.update()


def coat_shade(body, points_below=0.42):
    """Per-vertex shade of the coat, colour attribute ``fg_shade``.

    The CC0 AO and coat relief, darker lower legs and muzzle ("points"); the shader's
    robe tint multiplies it.

    The painted white socks and blaze of the source are not kept (every horse of a
    regiment would carry the same markings): the shade is clamped around the coat.
    """
    imgs = {i.name.split(".")[0]: i for i in bpy.data.images}
    col = imgs["HorseMain4k00"]
    ao = imgs.get("HorseMain4k00AO00")
    w, h = col.size
    px = np.array(col.pixels[:], dtype=np.float32).reshape(h, w, 4)
    lum = px[..., :3] @ np.array([0.3, 0.59, 0.11], dtype=np.float32)
    ao_px = None
    if ao is not None and ao.size[0]:
        aw, ah = ao.size
        ao_px = np.array(ao.pixels[:], dtype=np.float32).reshape(ah, aw, 4)[..., 0]
    me = body.data
    uv = me.uv_layers.get("UVTex") or me.uv_layers.active
    acc = np.zeros((len(me.vertices), 2))
    for loop in me.loops:
        acc[loop.vertex_index] += (*uv.data[loop.index].uv,)
    counts = np.zeros(len(me.vertices))
    for loop in me.loops:
        counts[loop.vertex_index] += 1
    uvs = acc / np.maximum(counts, 1)[:, None]
    xi = np.clip((uvs[:, 0] % 1.0) * (w - 1), 0, w - 1).astype(int)
    yi = np.clip((uvs[:, 1] % 1.0) * (h - 1), 0, h - 1).astype(int)
    lv = lum[yi, xi]
    rel = np.clip(lv / np.median(lv), 0.72, 1.12)
    if ao_px is not None:
        a = ao_px[
            np.clip(yi * ao_px.shape[0] // h, 0, ao_px.shape[0] - 1),
            np.clip(xi * ao_px.shape[1] // w, 0, ao_px.shape[1] - 1),
        ]
        a = np.clip(a / max(np.percentile(a, 90), 1e-3), 0.0, 1.0)
        rel *= 0.55 + 0.45 * a
    shade = []
    for i, v in enumerate(me.vertices):
        p = body.matrix_world @ v.co
        s = float(rel[i])
        # Points: lower legs darken from the knee/hock down; hooves are a separate material.
        f = min(max((points_below - p.z) / 0.25, 0.0), 1.0)
        s *= 1.0 - 0.5 * f
        shade.append(s)
    # Muzzle: the front 20 cm of the head.
    ys = [(body.matrix_world @ v.co).y for v in me.vertices]
    y_nose = min(ys)
    for i, v in enumerate(me.vertices):
        p = body.matrix_world @ v.co
        f = min(max((y_nose + 0.2 - p.y) / 0.12, 0.0), 1.0)
        shade[i] *= 1.0 - 0.45 * f
    attr = me.color_attributes.get("fg_shade") or me.color_attributes.new(
        "fg_shade", "FLOAT_COLOR", "POINT"
    )
    for i, s in enumerate(shade):
        attr.data[i].color = (s, s, s, 1.0)


def hoof_faces(body, top=0.1):
    """Face indices of the hooves (natural stance, below `top` metres)."""
    mw = body.matrix_world
    return [
        f.index
        for f in body.data.polygons
        if all((mw @ body.data.vertices[i].co).z < top for i in f.vertices)
    ]


HORSE_BUDGET = {
    "horse_body": 4200,
    "horse_mane": 1000,
    "horse_tail": 600,
    "horse_eye_l": 80,
    "horse_eye_r": 80,
}


def open_eyes(pieces, lift=0.009, radius=0.045, grow=1.12):
    """Open the drowsy lids of the source.

    Raise the upper lid, lower the lower one a little and enlarge the eyeballs (dark in
    game, they read as a slit otherwise).
    """
    body = pieces["horse_body"]
    for side in ("horse_eye_l", "horse_eye_r"):
        eye = pieces[side]
        pts = [v.co.copy() for v in eye.data.vertices]
        c = sum(pts, Vector()) / len(pts)
        for v in eye.data.vertices:
            v.co = c + (v.co - c) * grow
        eye.data.update()
        for v in body.data.vertices:
            d = v.co - c
            r = d.length
            if r > radius:
                continue
            f = 1 - r / radius
            f = f * f * (3 - 2 * f)
            v.co.z += (lift if d.z > -0.004 else -0.35 * lift) * f
        body.data.update()


def build_horse(mount, budget=None):
    """New horse fitted and skinned onto the Quaternius horse bones; returns objects.

    Every piece carries its weights (vertex groups named after the ``cavalry`` horse
    bones), an armature modifier on ``mount.harm`` and, for the body, the ``fg_shade``
    colour attribute and the ``fg_hoof`` face flag.
    """
    budget = budget or HORSE_BUDGET
    pieces = load_oga()
    similarity(pieces, mount)
    oga, q = joints(pieces, mount)
    fit = fit_transforms(oga, q)
    open_eyes(pieces)
    for name, o in pieces.items():
        decimate(o, budget.get(name, 0))
    body = pieces["horse_body"]
    heat = heat_armature(oga, sorted(BODY_BONES))
    heat_weights(body, heat)
    bpy.data.objects.remove(heat)
    table = smooth_weights(body, weight_table(body), passes=1)
    coat_shade(body)
    hooves = set(hoof_faces(body))
    flag = body.data.attributes.new("fg_hoof", "INT", "FACE")
    for i in range(len(body.data.polygons)):
        flag.data[i].value = 1 if i in hooves else 0
    tables = {"horse_body": table}
    tables["horse_mane"] = smooth_weights(
        pieces["horse_mane"],
        nearest_weights(pieces["horse_mane"], body, table),
        passes=2,
    )
    tail_chain = []
    for k, name in enumerate(TAIL_BONES):
        a = q[name]
        b = (
            q[TAIL_BONES[k + 1]]
            if k + 1 < len(TAIL_BONES)
            else a + Vector((0, 0.05, -0.7))
        )
        tail_chain.append((name, a, b))
    tables["horse_tail"] = chain_weights(pieces["horse_tail"], tail_chain)
    out = []
    for name, o in pieces.items():
        t = tables.get(name) or [{"Head": 1.0} for _v in o.data.vertices]
        apply_fit(o, t, fit)
        set_weights(o, t)
        mod = o.modifiers.new("arm", "ARMATURE")
        mod.object = mount.harm
        o.parent = mount.harm
        o.matrix_parent_inverse = mount.harm.matrix_world.inverted()
        o.data.shade_smooth()
        o.data.calc_loop_triangles()
        print(f"PIECE {name} tris={len(o.data.loop_triangles)}")
        out.append(o)
    for m in mount.hmeshes:
        m.hide_render = True
    return out


def rigid(obj, bone):
    """Bind every vertex to `bone`."""
    obj.vertex_groups.clear()
    g = obj.vertex_groups.new(name=bone)
    g.add(range(len(obj.data.vertices)), 1.0, "REPLACE")


def decimate(obj, target):
    """Collapse-decimate `obj` to about `target` triangles (applied)."""
    obj.data.calc_loop_triangles()
    tris = len(obj.data.loop_triangles)
    if target and tris > target:
        mod = obj.modifiers.new("dec", "DECIMATE")
        mod.ratio = target / tris
        mod.use_collapse_triangulate = True
        with bpy.context.temp_override(
            object=obj, active_object=obj, selected_objects=[obj]
        ):
            bpy.ops.object.modifier_apply(modifier=mod.name)
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def horse_materials(objs):
    """Eevee materials from the packed CC0 textures (colour, normal, AO; hair with alpha)."""
    imgs = {i.name.split(".")[0]: i for i in bpy.data.images}

    def textured(name, colour, normal, rough, alpha=False, tint=None):
        m = bpy.data.materials.new(name)
        m.use_nodes = True
        nt = m.node_tree
        bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
        uv = nt.nodes.new("ShaderNodeUVMap")
        uv.uv_map = "UVTex"
        tex = nt.nodes.new("ShaderNodeTexImage")
        tex.image = imgs.get(colour)
        nt.links.new(uv.outputs[0], tex.inputs[0])
        col = tex.outputs[0]
        if tint is not None:
            mix = nt.nodes.new("ShaderNodeMix")
            mix.data_type = "RGBA"
            mix.blend_type = "MULTIPLY"
            mix.inputs[0].default_value = 1.0
            nt.links.new(col, mix.inputs[6])
            mix.inputs[7].default_value = (*tint, 1)
            col = mix.outputs[2]
        nt.links.new(col, bsdf.inputs["Base Color"])
        if alpha:
            nt.links.new(tex.outputs[1], bsdf.inputs["Alpha"])
            m.blend_method = "HASHED" if hasattr(m, "blend_method") else None
        if normal and imgs.get(normal):
            ntex = nt.nodes.new("ShaderNodeTexImage")
            ntex.image = imgs[normal]
            ntex.image.colorspace_settings.name = "Non-Color"
            nt.links.new(uv.outputs[0], ntex.inputs[0])
            nmap = nt.nodes.new("ShaderNodeNormalMap")
            nmap.uv_map = "UVTex"
            nt.links.new(ntex.outputs[0], nmap.inputs[1])
            nt.links.new(nmap.outputs[0], bsdf.inputs["Normal"])
        bsdf.inputs["Roughness"].default_value = rough
        return m

    coat = textured("fg_horse_coat", "HorseMain4k00", "HorseMain4k00Norm00", 0.6)
    hair = textured(
        "fg_horse_hair",
        "Hair12Main2k",
        "Hair12Main2kNorm",
        0.7,
        alpha=True,
        tint=(0.35, 0.3, 0.25),
    )
    eye = bpy.data.materials.new("fg_horse_eye")
    eye.use_nodes = True
    b = next(n for n in eye.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    b.inputs["Base Color"].default_value = (0.02, 0.012, 0.008, 1)
    b.inputs["Roughness"].default_value = 0.1
    for o in objs:
        o.data.materials.clear()
        o.data.materials.append(
            coat if o.name == "horse_body" else eye if "eye" in o.name else hair
        )


TORSO = {
    "Back",
    "Torso",
    "Torso2",
    "Torso3",
    "Neck1",
    "FrontShoulder.L",
    "FrontShoulder.R",
    "BackShoulder.L",
    "BackShoulder.R",
    "BackLeg.L",
    "BackLeg.R",
}


def caparison(mount, body, hem=0.48, stations=26, around=24):
    """Trapper in livery from the withers to the croup, hanging in soft folds.

    Same principle as ``battle_skinned_cavalry.caparison`` (rings along the body, flat
    sides falling to `hem`), finer and fitted to the new horse; UV ``heraldry`` = arms on
    both flanks; weights of the nearest horse torso vertex (legs excluded).
    """
    import battle_fine_equipment as fe
    import battle_skinned_equipment as eq
    from mathutils.kdtree import KDTree

    names = {g.index: g.name for g in body.vertex_groups}
    mw = body.matrix_world
    pts = []
    for v in body.data.vertices:
        best = max(v.groups, key=lambda g: g.weight, default=None)
        if best is not None and names[best.group] in TORSO:
            pts.append(
                (
                    mw @ v.co,
                    [(names[g.group], g.weight) for g in v.groups if g.weight > 0],
                )
            )
    ys = [p.y for p, _w in pts]
    y0, y1 = min(ys) + 0.12, max(ys) - 0.1
    bm = bmesh.new()
    rings = []
    for k in range(stations + 1):
        u = k / stations
        y = y0 + (y1 - y0) * u
        near = [p for p, _w in pts if abs(p.y - y) < 0.08]
        half = max((abs(p.x) for p in near), default=0.3) + 0.025
        top = max((p.z for p in near), default=1.4) + 0.035
        ring = []
        for j in range(around + 1):
            a = math.pi * j / around  # 0 = left hem, pi = right hem
            side = math.cos(a)
            lift = math.sin(a)
            if lift > 0.5:
                z = top - (1 - lift) * 0.33
                x = half * side * 1.04
            else:
                f = lift / 0.5
                z = hem + (top - 0.33 - hem) * f
                sx = 1.0 if side > 0 else -1.0
                # Folds: stronger towards the hem, a few along the flank.
                fold = 0.022 * (1 - f) * math.sin(u * math.pi * 11 + 0.7)
                x = sx * (half * (1.02 + 0.07 * (1 - f)) + fold)
            ring.append(bm.verts.new(Vector((x, y, z))))
        rings.append(ring)
    # Two-piece trapper, parted at the saddle: the rider's legs hang between the halves.
    gap = (mount.seat.y - 0.24, mount.seat.y + 0.2)
    for ra, rb in zip(rings, rings[1:], strict=False):
        if gap[0] < (ra[0].co.y + rb[0].co.y) / 2 < gap[1]:
            continue
        for j in range(around):
            bm.faces.new((ra[j], ra[j + 1], rb[j + 1], rb[j]))
    bmesh.ops.delete(
        bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
    )
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for f in bm.faces:
        c = f.calc_center_median()
        if f.normal.dot(c - Vector((0, c.y, 1.0))) < 0:
            f.normal_flip()
    uv = bm.loops.layers.uv.new("heraldry")
    ymid = (y0 + y1) / 2
    zmid = hem + 0.45
    for f in bm.faces:
        for loop in f.loops:
            p = loop.vert.co
            uu = (p.y - ymid) / 0.75 + 0.5
            loop[uv].uv = (uu if p.x > 0 else 1 - uu, 0.5 - (p.z - zmid) / 0.75)
    obj = eq.to_object("caparison", bm, [fe.mat("arms")])
    tree = KDTree(len(pts))
    for i, (p, _w) in enumerate(pts):
        tree.insert(Vector((p.x * 0.6, p.y, p.z)), i)
    tree.balance()
    groups = {}
    for v in obj.data.vertices:
        p = obj.matrix_world @ v.co
        _co, idx, _d = tree.find(Vector((p.x * 0.6, p.y, max(p.z, 1.0))))
        for name, w in pts[idx][1]:
            if name not in groups:
                groups[name] = obj.vertex_groups.new(name=name)
            groups[name].add([v.index], w, "REPLACE")
    obj.data.shade_smooth()
    return [obj]
