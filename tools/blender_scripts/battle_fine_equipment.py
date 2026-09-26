"""Lot FG0: finer equipment for the prototype man-at-arms (c. 1340).

Garments are *shells* of the fitted MakeHuman body: the body faces of a region are copied,
relaxed (anatomy smoothed away), pushed out along their normals and keep the body's
weights, so they deform exactly like the body. Hard pieces (bassinet, aventail, sword,
shield, belt, scabbard) are modelled by script around the fitted body's landmarks.

Every builder returns mesh objects in world space (bind pose of the ``human`` rig) with
vertex groups named after the rig's bones and one of the ``MATS`` materials.
"""

import contextlib
import math

import battle_skinned_equipment as eq
import bmesh
import bpy
from mathutils import Vector
from mathutils.kdtree import KDTree

# name -> (shader code of battle_soldier_skinned, linear colour, roughness, metallic)
MATS = {
    "skin": (eq.C_SKIN, (0.50, 0.33, 0.24), 0.55, 0.0),
    "eye": (eq.C_EXACT, (0.22, 0.2, 0.18), 0.3, 0.0),
    "mail": (eq.C_MAIL, (0.26, 0.26, 0.27), 0.55, 1.0),
    "steel": (eq.C_PLATE, (0.55, 0.56, 0.58), 0.30, 1.0),
    "livery": (eq.C_LIVERY, (0.85, 0.85, 0.85), 0.85, 0.0),
    "leather": (eq.C_LEATHER, (0.13, 0.075, 0.035), 0.6, 0.0),
    "cloth": (eq.C_CLOTH, (0.16, 0.12, 0.08), 0.9, 0.0),
    "wood": (eq.C_WOOD, eq.WOOD, 0.7, 0.0),
    "arms": (eq.C_ARMS, (1.0, 1.0, 1.0), 0.6, 0.0),
    "brass": (eq.C_TRIM, (0.55, 0.40, 0.14), 0.35, 1.0),
    "hair": (eq.C_HAIR, (0.07, 0.04, 0.02), 0.7, 0.0),
}

TORSO = ("Body", "Hips", "Abdomen", "Torso", "Chest")


def mat(name):
    """Prototype material (custom props `code`, `rgb`, `rough`, `metal`)."""
    m = bpy.data.materials.get("fg_" + name)
    if m is None:
        code, rgb, rough, metal = MATS[name]
        m = bpy.data.materials.new("fg_" + name)
        m["code"] = code
        m["rgb"] = list(rgb)
        m["rough"] = rough
        m["metal"] = metal
        m["fg"] = name
        m.diffuse_color = (*eq.srgb_preview(rgb), 1.0)
    return m


# --- Body analysis ----------------------------------------------------------------------


def dominant(obj):
    """Dominant bone name per vertex."""
    names = {g.index: g.name for g in obj.vertex_groups}
    out = []
    for v in obj.data.vertices:
        best = max(
            (
                g
                for g in v.groups
                if names[g.group] not in ("body", "helper-l-eye", "helper-r-eye")
            ),
            key=lambda g: g.weight,
            default=None,
        )
        out.append(names[best.group] if best else "")
    return out


class Landmarks:
    """Measurements of the fitted body used to place the hard pieces."""

    def __init__(self, body, arm):
        """Measure the head, eyes, chin, waist and knees of the fitted `body`."""
        mw = body.matrix_world
        dom = dominant(body)
        head = [
            mw @ v.co
            for v, d in zip(body.data.vertices, dom, strict=True)
            if d == "Head"
        ]
        self.head_min = Vector(
            (min(p.x for p in head), min(p.y for p in head), min(p.z for p in head))
        )
        self.head_max = Vector(
            (max(p.x for p in head), max(p.y for p in head), max(p.z for p in head))
        )
        eyes = []
        for side in ("helper-l-eye", "helper-r-eye"):
            g = body.vertex_groups.get(side)
            if g is None:
                continue
            pts = [
                mw @ v.co
                for v in body.data.vertices
                if any(x.group == g.index and x.weight > 0.5 for x in v.groups)
            ]
            if pts:
                eyes.append(sum(pts, Vector()) / len(pts))
        self.eye = sum(eyes, Vector()) / len(eyes) if eyes else self.head_max * 0.5
        front = [p for p in head if p.y < self.eye.y + 0.02]
        self.chin = min(front, key=lambda p: p.z)
        self.top = max(head, key=lambda p: p.z)
        self.bone = {b.name: arm.matrix_world @ b.head_local for b in arm.data.bones}
        self.centre = Vector(
            (
                (self.head_min.x + self.head_max.x) / 2,
                (self.head_min.y + self.head_max.y) / 2,
                0,
            )
        )
        self.waist_z = self.bone["Abdomen"].z + 0.02
        self.knee_z = (self.bone["LowerLeg.L"].z + self.bone["LowerLeg.R"].z) / 2
        self.shoulder_z = (self.bone["UpperArm.L"].z + self.bone["UpperArm.R"].z) / 2


# --- Shells -----------------------------------------------------------------------------


def shell(body, name, material, keep, offset, relax=6, relax_factor=0.5):
    """Copy of the body faces for which `keep(face_centre, dominant bones)` is true.

    The copy is relaxed `relax` times (anatomy smoothed), pushed out by `offset` along its
    normals and keeps the body's weights.
    """
    dom = dominant(body)
    obj = body.copy()
    obj.data = body.data.copy()
    obj.name = obj.data.name = name
    obj.modifiers.clear()
    bpy.context.scene.collection.objects.link(obj)
    mw = obj.matrix_world
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    doomed = []
    for f in bm.faces:
        c = mw @ f.calc_center_median()
        if not keep(c, {dom[v.index] for v in f.verts}):
            doomed.append(f)
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    inner = [v for v in bm.verts if not v.is_boundary]
    for _ in range(relax):
        bmesh.ops.smooth_vert(
            bm,
            verts=inner,
            factor=relax_factor,
            use_axis_x=True,
            use_axis_y=True,
            use_axis_z=True,
        )
    # Clean the cut: relax the boundary loops along themselves (no jagged hems).
    border = [v for v in bm.verts if v.is_boundary]
    for _ in range(12):
        moved = {}
        for v in border:
            ns = [e.other_vert(v) for e in v.link_edges if e.is_boundary]
            if len(ns) == 2:
                moved[v] = v.co * 0.5 + (ns[0].co + ns[1].co) * 0.25
        for v, co in moved.items():
            v.co = co
    bm.normal_update()
    for v in bm.verts:
        v.co += v.normal * offset
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.materials.clear()
    obj.data.materials.append(material)
    for p in obj.data.polygons:
        p.material_index = 0
    return obj


def trim_body(body, keep_bones=("Head", "Neck", "Wrist.L", "Wrist.R")):
    """Delete the body faces hidden under the garments (keeps head, neck and hands)."""
    dom = dominant(body)
    bm = bmesh.new()
    bm.from_mesh(body.data)
    doomed = [
        f for f in bm.faces if not any(dom[v.index] in keep_bones for v in f.verts)
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    bm.to_mesh(body.data)
    bm.free()


def assign_body_materials(body):
    """Skin, eyes and gloved hands on the fitted body."""
    body.data.materials.clear()
    for name in ("skin", "eye", "leather"):
        body.data.materials.append(mat(name))
    dom = dominant(body)
    eyes = [body.vertex_groups.get(n) for n in ("helper-l-eye", "helper-r-eye")]
    eye_idx = {g.index for g in eyes if g is not None}
    eye_verts = {
        v.index
        for v in body.data.vertices
        if any(g.group in eye_idx and g.weight > 0.5 for g in v.groups)
    }
    for p in body.data.polygons:
        if all(i in eye_verts for i in p.vertices):
            p.material_index = 1
        elif any(dom[i].startswith("Wrist") for i in p.vertices):
            p.material_index = 2
        else:
            p.material_index = 0


# --- Weights helpers --------------------------------------------------------------------


def set_weights(obj, fn):
    """Replace the weights of `obj`: `fn(world position) -> {bone: weight}`."""
    obj.vertex_groups.clear()
    eq.bind_by(obj, fn)


def weights_from(body_kd, body_weights, lowerleg_to_upper=True):
    """Weight function copying the nearest body vertex (Data Transfer, nearest vertex)."""

    def fn(p):
        _co, idx, _d = body_kd.find(p)
        w = dict(body_weights[idx])
        if lowerleg_to_upper:
            for side in "LR":
                k = f"LowerLeg.{side}"
                if k in w:
                    w[f"UpperLeg.{side}"] = w.get(f"UpperLeg.{side}", 0.0) + w.pop(k)
        return w

    return fn


def body_lookup(body):
    """KD-tree and per-vertex weight dicts of the (untrimmed) body."""
    names = {g.index: g.name for g in body.vertex_groups}
    mw = body.matrix_world
    kd = KDTree(len(body.data.vertices))
    weights = []
    for v in body.data.vertices:
        kd.insert(mw @ v.co, v.index)
        weights.append(
            {
                names[g.group]: g.weight
                for g in v.groups
                if g.weight > 0
                and not names[g.group].startswith("helper")
                and names[g.group] != "body"
            }
        )
    kd.balance()
    return kd, weights


# --- Garments ---------------------------------------------------------------------------


def hauberk(body, lm):
    """Mail shirt over a padded aketon: torso, sleeves to the wrist, down to mid-thigh."""
    mid_thigh = lm.knee_z + (lm.bone["UpperLeg.L"].z - lm.knee_z) * 0.3

    def keep(c, bones):
        if bones & {"Head", "Neck", "Wrist.L", "Wrist.R", "Foot.L", "Foot.R"}:
            return False
        if bones & {"LowerLeg.L", "LowerLeg.R"}:
            return False
        return c.z > mid_thigh

    return [shell(body, "hauberk", mat("mail"), keep, 0.022, relax=40)]


def chausses(body, lm):
    """Mail chausses from the thigh to the instep, leather shoes."""
    top = lm.knee_z + (lm.bone["UpperLeg.L"].z - lm.knee_z) * 0.7

    def keep_legs(c, bones):
        legs = {"UpperLeg.L", "UpperLeg.R", "LowerLeg.L", "LowerLeg.R"}
        return bool(bones & legs) and not bones & {"Foot.L", "Foot.R"} and c.z < top

    def keep_feet(c, bones):
        return bool(bones & {"Foot.L", "Foot.R"})

    legs = shell(body, "chausses", mat("mail"), keep_legs, 0.008, relax=4)
    return [legs, shoes(body)]


def shoes(body):
    """Pointed leather shoes: convex hull of each foot, subdivided, weights from the body."""
    dom = dominant(body)
    mw = body.matrix_world
    kd, weights = body_lookup(body)
    out = []
    for side in "LR":
        pts = [
            mw @ v.co
            for v, d in zip(body.data.vertices, dom, strict=True)
            if d == f"Foot.{side}"
        ]
        bm = bmesh.new()
        for p in pts:
            bm.verts.new(p)
        hull = bmesh.ops.convex_hull(bm, input=bm.verts)
        bmesh.ops.delete(
            bm,
            geom=[
                g for g in hull["geom_interior"] if isinstance(g, bmesh.types.BMVert)
            ],
            context="VERTS",
        )
        bmesh.ops.dissolve_limit(
            bm, angle_limit=math.radians(8), verts=bm.verts, edges=bm.edges
        )
        bmesh.ops.triangulate(bm, faces=bm.faces)
        bm.normal_update()
        for v in bm.verts:
            v.co += v.normal * 0.005
        obj = to_object_smooth(f"shoe_{side}", bm, [mat("leather")], levels=2)
        eq.bind_by(obj, weights_from(kd, weights, lowerleg_to_upper=False))
        out.append(obj)
    return join(out, "shoes")


def to_object_smooth(name, bm, materials, levels=1):
    """Mesh object from a bmesh, Catmull-Clark subdivided `levels` times (applied)."""
    obj = eq.to_object(name, bm, materials)
    mod = obj.modifiers.new("sub", "SUBSURF")
    mod.levels = levels
    mod.render_levels = levels
    with bpy.context.temp_override(
        object=obj, active_object=obj, selected_objects=[obj]
    ):
        bpy.ops.object.modifier_apply(modifier=mod.name)
    return obj


def join(objs, name):
    """Join mesh objects into the first one (renamed)."""
    base = objs[0]
    if len(objs) > 1:
        with bpy.context.temp_override(
            active_object=base,
            object=base,
            selected_objects=objs,
            selected_editable_objects=objs,
        ):
            bpy.ops.object.join()
    base.name = base.data.name = name
    return base


def _ray_radius(bvh, centre, direction, fallback):
    hit = bvh.ray_cast(centre + direction * 0.6, -direction, 0.6)
    if hit[0] is None:
        return fallback
    return (hit[0] - centre).length


def surcoat(body, lm, garment, bvh):
    """Sleeveless surcoat in livery over the hauberk, split skirt to the knee."""
    sleeve_cut = lm.shoulder_z - 0.02

    def keep(c, bones):
        if not bones <= set(TORSO) | {
            "Shoulder.L",
            "Shoulder.R",
            "UpperLeg.L",
            "UpperLeg.R",
        }:
            return False
        # Wide armholes: the shoulder yoke only near the neck.
        if bones & {"Shoulder.L", "Shoulder.R"} and (
            c.z < sleeve_cut or abs(c.x) > 0.12
        ):
            return False
        return c.z > lm.waist_z - 0.03

    top = shell(garment, "surcoat", mat("livery"), keep, 0.014, relax=30)
    skirt = _skirt(lm, bvh)
    return [top, skirt]


def _skirt(lm, bvh, n=40, rows=10):
    """Skirt from the waist to the knees, slit front and back over the lower 55 %."""
    waist = lm.waist_z
    hem = lm.knee_z + 0.05
    hip_l = lm.bone["UpperLeg.L"]
    hip_r = lm.bone["UpperLeg.R"]
    knee_l = lm.bone["LowerLeg.L"]
    knee_r = lm.bone["LowerLeg.R"]
    bm = bmesh.new()
    grid = []
    for r in range(rows + 1):
        t = r / rows
        z = waist + (hem - waist) * t
        # Centre follows the stance (right leg forward): between the two thighs at height z.
        f_l = (hip_l.z - z) / max(hip_l.z - knee_l.z, 1e-3)
        f_r = (hip_r.z - z) / max(hip_r.z - knee_r.z, 1e-3)
        pl = hip_l.lerp(knee_l, min(max(f_l, 0.0), 1.0))
        pr = hip_r.lerp(knee_r, min(max(f_r, 0.0), 1.0))
        centre = Vector(((pl.x + pr.x) / 2, (pl.y + pr.y) / 2, z))
        spread = Vector(((pl.x - pr.x) / 2, (pl.y - pr.y) / 2, 0))
        row = []
        for i in range(n):
            a = 2 * math.pi * i / n
            d = Vector((math.cos(a), math.sin(a), 0))
            base = (
                _ray_radius(bvh, Vector((centre.x, centre.y, z)), d, 0.17)
                if t < 0.35
                else None
            )
            flare = 0.02 + 0.035 * t
            # Ellipse around both legs: wider across the stance.
            rx = 0.15 + abs(spread.x) * 0.5 + flare
            ry = 0.11 + abs(spread.y) * 0.8 + flare
            rad = math.hypot(rx * math.cos(a), ry * math.sin(a))
            if base is not None:
                blend = t / 0.35
                rad = (base + 0.012) * (1 - blend) + rad * blend
            # Soft folds, deeper towards the hem.
            rad += 0.012 * t * math.sin(a * 9 + 1.3) + 0.006 * t * math.sin(a * 17)
            row.append(bm.verts.new(centre + d * rad))
        grid.append(row)
    slit_rows = int(rows * 0.6)
    front = round(n * 3 / 4) % n  # -Y (sin = -1)
    back = round(n / 4) % n  # +Y
    for r in range(rows):
        for i in range(n):
            j = (i + 1) % n
            if r >= slit_rows and i in (front, back):
                continue
            bm.faces.new((grid[r][i], grid[r][j], grid[r + 1][j], grid[r + 1][i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    obj = eq.to_object("surcoat_skirt", bm, [mat("livery")])

    def weigh(p):
        t = min(max((waist - p.z) / (waist - hem), 0.0), 1.0)
        leg = t**1.2 * 0.85
        rel = p - Vector(((hip_l.x + hip_r.x) / 2, (hip_l.y + hip_r.y) / 2, 0))
        side_l = eq.smoothstep(-0.06, 0.06, rel.x)
        return {
            "Hips": 1.0 - leg,
            "UpperLeg.L": leg * side_l,
            "UpperLeg.R": leg * (1 - side_l),
        }

    eq.bind_by(obj, weigh)
    return obj


def belt(lm, bvh, n=40):
    """Sword belt worn low on the hips over the surcoat, with brass mounts."""
    z = lm.waist_z - 0.07
    centre = (lm.bone["UpperLeg.L"] + lm.bone["UpperLeg.R"]) / 2
    centre = Vector((centre.x, centre.y, z))
    bm = bmesh.new()
    rings = []
    for dz in (-0.022, 0.022):
        row = []
        for i in range(n):
            a = 2 * math.pi * i / n
            d = Vector((math.cos(a), math.sin(a), 0))
            r = _ray_radius(bvh, centre + Vector((0, 0, dz)), d, 0.17) + 0.02
            row.append(bm.verts.new(centre + Vector((0, 0, dz)) + d * r))
        rings.append(row)
    eq.bridge(bm, rings[0], rings[1], 0)
    # Brass mounts: small bosses every other segment.
    for i in range(0, n, 3):
        p = (rings[0][i].co + rings[1][i].co) / 2
        out = p - centre
        out.z = 0
        out.normalize()
        eq.box(
            bm,
            p + out * 0.004,
            (0.009, 0.004, 0.012),
            (Vector((-out.y, out.x, 0)), out, Vector((0, 0, 1))),
            1,
        )
    eq.finish(bm)
    obj = eq.to_object("belt", bm, [mat("leather"), mat("brass")])
    eq.bind_rigid(obj, "Hips")
    return [obj]


def scabbard(lm):
    """Scabbard hanging at the left hip, point back and down."""
    hip = lm.bone["UpperLeg.L"]
    top = Vector((hip.x + 0.1, hip.y - 0.03, lm.waist_z - 0.1))
    tip = top + Vector((0.03, 0.55, -0.83)).normalized() * 0.8
    bm = bmesh.new()
    eq.tube(bm, top, tip, 0.028, 0.012, 8, 0)
    eq.tube(bm, tip - (tip - top).normalized() * 0.06, tip, 0.016, 0.004, 8, 1)  # chape
    eq.tube(bm, top, top + (top - tip).normalized() * 0.03, 0.03, 0.03, 8, 1)  # locket
    eq.finish(bm)
    obj = eq.to_object("scabbard", bm, [mat("leather"), mat("brass")])
    eq.bind_rigid(obj, "Hips")
    return [obj]


# --- Head -------------------------------------------------------------------------------


def _rim_z(lm, a):
    """Lower edge of the bassinet at angle `a` (sin a = -1: face)."""
    front = max(0.0, -math.sin(a))
    brow = lm.eye.z + 0.035
    jaw = lm.chin.z + 0.035
    # Face opening: brow at the front, down to the jaw on the sides and at the back.
    return brow * eq.smoothstep(0.55, 0.9, front) + jaw * (
        1 - eq.smoothstep(0.55, 0.9, front)
    )


def bassinet(lm, n=36, rows=12):
    """Pointed bassinet of the 1340s, the skull drawn back to a point, open face."""
    cx = (lm.head_min.x + lm.head_max.x) / 2
    cy = (lm.head_min.y + lm.head_max.y) / 2 + 0.005
    rx = (lm.head_max.x - lm.head_min.x) / 2 + 0.006
    ry = (lm.head_max.y - lm.head_min.y) / 2 + 0.010
    top = lm.top.z + 0.014
    apex = Vector((cx, cy + ry * 0.35, top + 0.045))
    bm = bmesh.new()
    rings = []
    for r in range(rows):
        t = r / rows
        row = []
        for i in range(n):
            a = 2 * math.pi * i / n
            z0 = _rim_z(lm, a)
            # Dome profile: straight sides, then a gothic arch to the apex.
            zc = lm.eye.z + 0.035
            if t < 0.35:
                z = z0 + (zc + 0.02 - z0) * (t / 0.35)
                k = 1.0 + 0.02 * (t / 0.35)
            else:
                u = (t - 0.35) / 0.65
                z = zc + 0.02 + (top - zc - 0.02) * math.sin(u * math.pi / 2) ** 0.9
                k = 1.02 * math.cos(u * math.pi / 2) ** 0.75
            c = Vector((cx, cy + (apex.y - cy) * max(0.0, t - 0.35) * 1.2, z))
            row.append(
                bm.verts.new(
                    c + Vector((rx * k * math.cos(a), ry * k * math.sin(a), 0))
                )
            )
        rings.append(row)
    for a_, b_ in zip(rings, rings[1:], strict=False):
        eq.bridge(bm, a_, b_, 0)
    ap = bm.verts.new(apex)
    for i in range(n):
        bm.faces.new((rings[-1][i], rings[-1][(i + 1) % n], ap))
    # Rolled edge: a thin band turned inwards along the rim.
    inner = []
    for i, v in enumerate(rings[0]):
        a = 2 * math.pi * i / n
        d = Vector((math.cos(a), math.sin(a), 0))
        inner.append(bm.verts.new(v.co - d * 0.006 + Vector((0, 0, 0.004))))
    eq.bridge(bm, inner, rings[0], 0)
    # Vervelles (staples holding the aventail) along the sides and back.
    for i in range(0, n, 2):
        a = 2 * math.pi * i / n
        if -math.sin(a) > 0.55:
            continue
        v = rings[1][i].co
        d = Vector((math.cos(a), math.sin(a), 0))
        eq.box(
            bm,
            v + d * 0.003,
            (0.004, 0.003, 0.006),
            (Vector((-d.y, d.x, 0)), d, Vector((0, 0, 1))),
            1,
        )
    eq.finish(bm)
    obj = eq.to_object("bassinet", bm, [mat("steel"), mat("brass")])
    eq.bind_rigid(obj, "Head")
    return [obj], (cx, cy, rx, ry)


def aventail(lm, frame, bvh, extra=(), n=36, rows=12):
    """Mail aventail laced to the bassinet, draped over the shoulders as a short cape.

    Each column hangs from the helmet rim (under the chin at the face) and follows the
    larger of its free hang and the hauberk surface below (ray cast), 14 mm off it.
    """
    cx, cy, rx, ry = frame
    neck = lm.bone["Neck"]
    axis = Vector((neck.x, neck.y + 0.005, 0))
    bm = bmesh.new()
    grid = []
    for r in range(rows + 1):
        t = r / rows
        row = []
        for i in range(n):
            a = 2 * math.pi * i / n
            front = max(0.0, -math.sin(a))
            d = Vector((math.cos(a), math.sin(a), 0))
            face = eq.smoothstep(0.45, 0.75, front)
            z_top = (_rim_z(lm, a) + 0.004) * (1 - face) + (lm.chin.z + 0.012) * face
            k = 0.97 + 0.03 * face
            top_r = math.hypot(rx * k * d.x, ry * k * d.y)
            z_bot = lm.shoulder_z - 0.07 - 0.07 * abs(d.y)
            z = z_top + (z_bot - z_top) * t
            hang = top_r + (0.085 - top_r) * eq.smoothstep(0.0, 0.35, t)
            surf = _ray_radius(bvh, Vector((axis.x, axis.y, z)), d, 0.0)
            surf = max(
                surf,
                *(_ray_radius(b, Vector((axis.x, axis.y, z)), d, 0.0) for b in extra),
            )
            rad = max(hang, surf + 0.014) if t > 0.15 else hang
            rad = min(rad, 0.2)
            row.append(bm.verts.new(Vector((axis.x, axis.y, z)) + d * rad))
        grid.append(row)
    # The top ring hangs from the helmet's centre, not the neck axis.
    for i, v in enumerate(grid[0]):
        a = 2 * math.pi * i / n
        d = Vector((math.cos(a), math.sin(a), 0))
        k = 0.97 + 0.03 * eq.smoothstep(0.45, 0.75, -math.sin(a))
        v.co = Vector((cx, cy, v.co.z)) + Vector((rx * k * d.x, ry * k * d.y, 0))
    for a_, b_ in zip(grid, grid[1:], strict=False):
        eq.bridge(bm, a_, b_, 0)
    # Open surface: orient every face away from the neck axis (outside = visible side).
    for f in bm.faces:
        c = f.calc_center_median()
        if f.normal.dot(Vector((c.x - axis.x, c.y - axis.y, 0))) < 0:
            f.normal_flip()
    obj = eq.to_object("aventail", bm, [mat("mail")])
    top_z = lm.chin.z
    bot_z = lm.shoulder_z

    def weigh(p):
        t = eq.smoothstep(top_z - 0.02, bot_z + 0.02, p.z)  # 1 near the head
        side_l = eq.smoothstep(0.04, 0.16, p.x)
        side_r = eq.smoothstep(0.04, 0.16, -p.x)
        low = 1 - t
        return {
            "Head": t * 0.8,
            "Neck": t * 0.2 + low * 0.3,
            "Chest": low * 0.7 * (1 - 0.5 * (side_l + side_r)),
            "Shoulder.L": low * 0.35 * side_l,
            "Shoulder.R": low * 0.35 * side_r,
        }

    eq.bind_by(obj, weigh)
    return [obj]


# --- Arms -------------------------------------------------------------------------------


def sword(ctx, length=0.92, fist=None):
    """Arming sword (type XVI): fullered, tapering to a point; wheel pommel, straight guard.

    `fist` = (centre, axis from the little finger to the index) of the closed right hand.
    """
    c, along, up, out = eq.grip(ctx, "R")
    if fist is not None:
        c, along = fist
        fore = (ctx.head("Wrist.R") - ctx.head("LowerArm.R")).normalized()
        up = (fore - along * fore.dot(along)).normalized()
        out = along.cross(up).normalized()
    bm = bmesh.new()
    eq.tube(bm, c - along * 0.075, c + along * 0.07, 0.014, 0.015, 10, 1)  # grip
    # Wheel pommel.
    eq.tube(bm, c - along * 0.105, c - along * 0.078, 0.028, 0.028, 16, 2)
    eq.tube(bm, c - along * 0.114, c - along * 0.105, 0.012, 0.012, 8, 2)
    # Cross-guard: tapering bar.
    g = c + along * 0.082
    eq.tube(bm, g - up * 0.11, g + up * 0.11, 0.009, 0.009, 8, 2)
    # Blade: flattened hexagon section, fuller down two thirds.
    base = c + along * 0.095
    steps = 8
    rings = []
    for s in range(steps + 1):
        t = s / steps
        p = base + along * (length - 0.095) * t
        w = 0.026 * (1 - t) ** 0.55 + 0.001
        th = 0.0055 * (1 - t) + 0.001
        fuller = 0.0025 if t < 0.66 else 0.0
        pts = [
            p + up * w,
            p + up * w * 0.4 + out * th,
            p + out * (th - fuller),
            p - up * w * 0.4 + out * th,
            p - up * w,
            p - up * w * 0.4 - out * th,
            p - out * (th - fuller),
            p + up * w * 0.4 - out * th,
        ]
        rings.append([bm.verts.new(q) for q in pts])
    for a_, b_ in zip(rings, rings[1:], strict=False):
        eq.bridge(bm, a_, b_, 0)
    eq.cap(bm, rings[0], 0)
    tip = bm.verts.new(base + along * (length - 0.095 + 0.03))
    for i in range(8):
        bm.faces.new((rings[-1][i], rings[-1][(i + 1) % 8], tip))
    eq.finish(bm)
    obj = eq.to_object("sword", bm, [mat("steel"), mat("leather"), mat("brass")])
    eq.bind_rigid(obj, "Wrist.R")
    return [obj]


def heater_shield(ctx, width=0.52, height=0.66, cols=14, rows=16):
    """Heater shield on the left forearm: curved board, raw-hide rim, arms on the face."""
    lower = ctx.head("LowerArm.L")
    wrist = ctx.head("Wrist.L")
    fore = (wrist - lower).normalized()
    centre = lower.lerp(wrist, 0.45) + Vector((0, 0, 0.08))
    # Face turned outwards and a little forwards, the board along the forearm.
    normal = Vector((1.0, -0.35, 0.0))
    normal = (normal - fore * normal.dot(fore)).normalized()
    centre = lower.lerp(wrist, 0.45) + normal * 0.05
    side = normal.cross(fore).normalized()

    def outline_half(v):
        # v in [0, 1] from the point (0) to the top edge (1).
        return width / 2 * math.sin(min(v / 0.62, 1.0) * math.pi / 2) ** 0.75

    bm = bmesh.new()
    uv_front = []
    front, back = [], []
    for j in range(rows + 1):
        v = j / rows
        half = max(outline_half(v), 0.004)
        rf, rb = [], []
        for i in range(cols + 1):
            u = i / cols
            sx = (u - 0.5) * 2 * half
            p = centre + side * sx + fore * (-height * 0.35 + height * v)
            bulge = 0.045 * (1 - (2 * sx / width) ** 2)
            rf.append(bm.verts.new(p + normal * (0.014 + bulge)))
            rb.append(bm.verts.new(p + normal * bulge))
            uv_front.append((rf[-1], 0.5 + sx / width, v))
        front.append(rf)
        back.append(rb)
    for j in range(rows):
        for i in range(cols):
            bm.faces.new(
                (front[j][i], front[j][i + 1], front[j + 1][i + 1], front[j + 1][i])
            ).material_index = 0
            bm.faces.new(
                (back[j][i], back[j + 1][i], back[j + 1][i + 1], back[j][i + 1])
            ).material_index = 1
    # Edges (rim).
    border_f = (
        [front[j][0] for j in range(rows + 1)]
        + front[rows][1:]
        + [front[j][cols] for j in range(rows - 1, -1, -1)]
        + [front[0][i] for i in range(cols - 1, 0, -1)]
    )
    border_b = (
        [back[j][0] for j in range(rows + 1)]
        + back[rows][1:]
        + [back[j][cols] for j in range(rows - 1, -1, -1)]
        + [back[0][i] for i in range(cols - 1, 0, -1)]
    )
    m = len(border_f)
    for i in range(m):
        k = (i + 1) % m
        a, b, c, d = border_f[i], border_f[k], border_b[k], border_b[i]
        if len({a, b, c, d}) == 4:
            with contextlib.suppress(ValueError):  # face already exists at the tip
                bm.faces.new((a, d, c, b)).material_index = 2
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    eq.finish(bm)
    obj = eq.to_object("shield", bm, [mat("arms"), mat("wood"), mat("leather")])
    uv = obj.data.uv_layers.new(name="heraldry")
    mw_inv = obj.matrix_world.inverted()
    for loop in obj.data.loops:
        co = mw_inv @ obj.data.vertices[loop.vertex_index].co
        rel = co - centre
        uv.data[loop.index].uv = (
            0.5 + rel.dot(side) / width,
            0.35 + rel.dot(fore) / height,
        )
    eq.bind_rigid(obj, "LowerArm.L")
    return [obj]
