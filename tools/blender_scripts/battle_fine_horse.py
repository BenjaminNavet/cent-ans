"""Lot FG0: finer horse on the ``cavalry`` rig (Quaternius horse bones kept).

Source: "Rigged Horse" by Lyndon Daniels (OpenGameArt, CC0), a textured draught horse
(7 400 triangles of body, 3 900 of mane, 1 750 of tail, 2k colour / normal / AO maps). Its own
19-bone rig is dropped.

Fit:
1. Every piece is baked to world space (standing rest shape), scaled uniformly so the
   back at the saddle matches the Quaternius horse, and centred on its legs.
2. A Gaussian RBF space warp moves landmarks of the new horse onto the Quaternius ones:
   each leg sliced at the joint heights (elbow/stifle, knee/hock, fetlock, hoof), the head,
   the withers and the croup. Legs then pivot where the Quaternius bones do.
3. Weights: body by bone heat (Blender automatic weights) on a copy of the horse armature
   whose stub bones get real segments (head to child head); mane and tail by the nearest
   point on the Quaternius horse surface (barycentric), relaxed; eyes rigid on ``Head``.
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

FRONT_JOINTS = ("FrontUpperLeg", "FrontLowerLeg", "FF")
BACK_JOINTS = ("BackUpperLeg", "BackLowerLeg", "FFB")


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


def landmarks(pieces, mount):
    """(source, target) landmark pairs for the warp."""
    arm = mount.harm

    def q(name):
        return arm.matrix_world @ arm.data.bones[name].head_local

    pts = _verts(pieces["horse_body"])
    pairs = []
    for side, sx in (("L", 1), ("R", -1)):
        for joints, front in ((FRONT_JOINTS, True), (BACK_JOINTS, False)):
            targets = [q(f"{j}.{side}") for j in joints]
            # Track the leg upwards from the hoof, starting under the target hoof.
            around = targets[-1].copy()
            prev = None
            track = {}
            for tgt in reversed(targets):
                z = max(tgt.z, 0.06)
                guess = prev if prev is not None else around
                c = _slice_centroid(pts, z, guess, radius=0.2)
                if c is None or z > 0.75:
                    c = None
                track[tgt.z] = c
                if c is not None:
                    prev = c
            last_src, last_tgt = None, None
            for tgt in reversed(targets):
                c = track[tgt.z]
                if c is not None:
                    src = Vector((c.x, c.y, tgt.z))
                    last_src, last_tgt = src, tgt
                elif last_src is not None:
                    # Upper joint merged with the body: move it with the joint below.
                    src = tgt - (last_tgt - last_src)
                else:
                    continue
                pairs.append((src, tgt))
    # Head, withers and croup: mesh-centroid correspondences.
    q_pts = [p for m in mount.hmeshes for p in _verts(m)]
    # (No head landmark: the Quaternius head is a different shape; raise_neck aligns it.)
    for fn in (_withers, _croup):
        pairs.append((fn(pts), fn(q_pts)))
    return pairs


def _head_centroid(pts):
    y0 = min(p.y for p in pts)
    sel = [p for p in pts if p.y < y0 + 0.3]
    return sum(sel, Vector()) / len(sel)


def _withers(pts):
    y0 = min(p.y for p in pts)
    y1 = max(p.y for p in pts)
    band = [
        p
        for p in pts
        if y0 + 0.35 * (y1 - y0) < p.y < y0 + 0.45 * (y1 - y0) and abs(p.x) < 0.1
    ]
    return max(band, key=lambda p: p.z)


def _croup(pts):
    y0 = min(p.y for p in pts)
    y1 = max(p.y for p in pts)
    band = [
        p
        for p in pts
        if y0 + 0.78 * (y1 - y0) < p.y < y0 + 0.88 * (y1 - y0) and abs(p.x) < 0.1
    ]
    return max(band, key=lambda p: p.z)


def rbf_warp(pieces, pairs, sigma=0.3):
    """Gaussian RBF displacement field through the landmark pairs, applied to every piece."""
    src = np.array([tuple(s) for s, _t in pairs])
    dst = np.array([tuple(t) for _s, t in pairs])
    d2 = ((src[:, None, :] - src[None, :, :]) ** 2).sum(-1)
    k = np.exp(-d2 / (2 * sigma * sigma)) + 1e-6 * np.eye(len(src))
    w = np.linalg.solve(k, dst - src)
    for o in pieces.values():
        n = len(o.data.vertices)
        co = np.empty(n * 3)
        o.data.vertices.foreach_get("co", co)
        co = co.reshape(n, 3)
        d2 = ((co[:, None, :] - src[None, :, :]) ** 2).sum(-1)
        co = co + np.exp(-d2 / (2 * sigma * sigma)) @ w
        o.data.vertices.foreach_set("co", co.ravel())
        o.data.update()
    err = max((Vector(t) - Vector(s)).length for s, t in pairs)
    print(f"HORSE warp landmarks={len(pairs)} max_move={err:.3f}")


def transfer_weights(obj, q_mesh, relax=3):
    """Barycentric weights of the nearest Quaternius horse triangle, then relaxed."""
    q_me = q_mesh.data
    q_me.calc_loop_triangles()
    mw = q_mesh.matrix_world
    verts = [mw @ v.co for v in q_me.vertices]
    tris = [tuple(t.vertices) for t in q_me.loop_triangles]
    bvh = BVHTree.FromPolygons(verts, tris)
    names = {g.index: g.name for g in q_mesh.vertex_groups}
    vw = [
        {names[g.group]: g.weight for g in v.groups if g.weight > 0}
        for v in q_me.vertices
    ]
    weights = []
    for v in obj.data.vertices:
        p = obj.matrix_world @ v.co
        loc, _n, ti, _d = bvh.find_nearest(p)
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
            for k, w in vw[idx].items():
                acc[k] = acc.get(k, 0.0) + w * max(f, 0.0)
        weights.append(acc)
    adj = [[] for _ in obj.data.vertices]
    for e in obj.data.edges:
        i, j = e.vertices
        adj[i].append(j)
        adj[j].append(i)
    for _ in range(relax):
        new = []
        for i, w in enumerate(weights):
            if not adj[i]:
                new.append(w)
                continue
            acc = {k: v * 0.5 for k, v in w.items()}
            share = 0.5 / len(adj[i])
            for j in adj[i]:
                for k, v in weights[j].items():
                    acc[k] = acc.get(k, 0.0) + v * share
            new.append(acc)
        weights = new
    obj.vertex_groups.clear()
    groups = {}
    for i, w in enumerate(weights):
        top = sorted(w.items(), key=lambda kv: -kv[1])[:4]
        total = sum(x for _k, x in top) or 1.0
        for k, x in top:
            if k not in groups:
                groups[k] = obj.vertex_groups.new(name=k)
            groups[k].add([i], x / total, "REPLACE")


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


HORSE_BUDGET = {
    "horse_body": 4200,
    "horse_mane": 1000,
    "horse_tail": 600,
    "horse_eye_l": 80,
    "horse_eye_r": 80,
}

# Tail of each deforming bone for the bone-heat weights (the imported bones are stubs).
TAIL_TO = {
    "Body": "Back",
    "Back": "Tail2",
    "Torso": "Torso2",
    "Torso2": "Torso3",
    "Torso3": "Neck1",
    "Neck1": "Neck2",
    "Neck2": "Neck3",
    "Neck3": "Head",
}
for _s in "LR":
    TAIL_TO.update(
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
            f"Ear1.{_s}": f"Ear2.{_s}",
            f"Ear2.{_s}": f"Ear3.{_s}",
            f"Ear3.{_s}": f"Ear4.{_s}",
        }
    )
for _k in range(1, 7):
    TAIL_TO[f"Tail{_k}"] = f"Tail{_k + 1}"


def heat_rig(mount, deform):
    """Copy of the horse armature with real bone segments, for automatic weights."""
    src = mount.harm
    arm = src.copy()
    arm.data = src.data.copy()
    arm.animation_data_clear()
    bpy.context.scene.collection.objects.link(arm)
    heads = {b.name: b.head_local.copy() for b in src.data.bones}
    with bpy.context.temp_override(
        active_object=arm, object=arm, selected_objects=[arm]
    ):
        bpy.ops.object.mode_set(mode="EDIT")
        eb = arm.data.edit_bones
        for b in eb:
            b.use_connect = False
        for b in eb:
            b.use_deform = b.name in deform
            if b.name in TAIL_TO:
                t = heads[TAIL_TO[b.name]]
                if (t - b.head).length > 1e-3:
                    b.tail = t
            elif b.name in ("FF.L", "FF.R", "FFB.L", "FFB.R"):
                b.tail = (
                    b.head + Vector((0, -0.08, -0.06)) / src.matrix_world.to_scale().x
                )
            elif b.name == "Head":
                b.tail = (
                    b.head + Vector((0, -0.45, -0.3)) / src.matrix_world.to_scale().x
                )
            elif b.name in ("Ear4.L", "Ear4.R", "Tail7"):
                b.tail = b.head + (b.head - b.parent.head) * 0.8
        bpy.ops.object.mode_set(mode="OBJECT")
    return arm


def auto_weights(obj, arm):
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
    print(f"WEIGHTS {obj.name} groups={len(obj.vertex_groups)}")


def raise_neck(pieces, mount):
    """Swing the neck and head up about the withers onto the Quaternius head carriage."""
    body = pieces["horse_body"]
    pts = _verts(body)
    q_pts = [p for m in mount.hmeshes for p in _verts(m)]
    pivot = _withers(pts) + Vector((0, -0.05, -0.25))
    src = _head_centroid(pts) - pivot
    dst = _head_centroid(q_pts) - pivot
    a_src = math.atan2(src.z, -src.y)
    a_dst = math.atan2(dst.z, -dst.y)
    angle = a_dst - a_src
    y0 = pivot.y + 0.05
    y1 = pivot.y - 0.35
    print(f"HORSE neck raise={math.degrees(angle):.1f} deg")
    for o in pieces.values():
        for v in o.data.vertices:
            p = v.co
            if p.y > y0:
                continue
            f = min(max((y0 - p.y) / (y0 - y1), 0.0), 1.0)
            f = f * f * (3 - 2 * f)
            rot = Matrix.Rotation(-angle * f, 3, "X")
            v.co = pivot + rot @ (p - pivot)
        o.data.update()


def build_horse(mount):
    """New horse fitted and skinned onto the Quaternius horse bones; returns objects."""
    pieces = load_oga()
    similarity(pieces, mount)
    raise_neck(pieces, mount)
    rbf_warp(pieces, landmarks(pieces, mount))
    q_mesh = mount.hmeshes[0]
    deform = set(q_mesh.vertex_groups.keys())
    arm = heat_rig(mount, deform)
    out = []
    for name, o in pieces.items():
        tris = decimate(o, HORSE_BUDGET.get(name, 0))
        if name.startswith("horse_eye"):
            rigid(o, "Head")
        elif name == "horse_body":
            # Bone heat beat a height-band copy of the Quaternius leg weights (tried in
            # FG0: the draught legs are thicker, the copy twisted them at the gallop).
            auto_weights(o, arm)
        else:
            transfer_weights(o, q_mesh)
        mod = o.modifiers.new("arm", "ARMATURE")
        mod.object = mount.harm
        o.parent = mount.harm
        o.matrix_parent_inverse = mount.harm.matrix_world.inverted()
        o.data.shade_smooth()
        print(f"PIECE {name} tris={tris}")
        out.append(o)
    q_mesh.hide_render = True
    bpy.data.objects.remove(arm)
    return out


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
