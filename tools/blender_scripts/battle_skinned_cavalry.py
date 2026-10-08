"""Mounted figures of the skinned battle rendering (lot V2): horse and rider in one mesh.

The Quaternius horse (50 bones, 13 actions) and a Quaternius man share one bone texture
(`cavalry.bones.bin`): horse bones keep their names, rider bones are prefixed `R:`. The rider's
armature rides on the horse's `Torso2` bone (it bobs and pitches with the horse), his legs
reach the stirrups by IK and his arms hold the reins, the lance (virtual bone `R:Prop`) or
the bow. Clips pair a horse action with rider overrides (`battle_skinned_poses`).
"""

import math
import os

import battle_skinned as bs
import battle_skinned_gaits as gaits
import battle_skinned_equipment as eq
import battle_skinned_poses as poses
import battle_skinned_weapons as weapons
import bmesh
import bpy
from mathutils import Matrix, Vector
from mathutils.kdtree import KDTree

SADDLE_BONE = "Torso2"
STIRRUP_HALF_WIDTH = 0.36  # m from the horse's midline (FG4; 0.30 before)
TORSO_BONES = {
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
HORSE_COLORS = {
    "Main": (eq.C_COAT, (0.62, 0.62, 0.62)),
    "Main_Dark": (eq.C_COAT, (0.40, 0.40, 0.40)),
    "Main_Light": (eq.C_COAT, (0.85, 0.85, 0.85)),
    "Muzzle": (eq.C_COAT, (0.22, 0.22, 0.22)),
    "Hair": (eq.C_HAIR, (0.05, 0.04, 0.03)),
    "Hooves": (eq.C_EXACT, (0.07, 0.06, 0.05)),
    "Eye_Black": (eq.C_EXACT, (0.01, 0.01, 0.01)),
    "Eye_White": (eq.C_EXACT, (0.25, 0.22, 0.2)),
}


class Mount:
    """Horse and rider armatures at rest, with the saddle and stirrup points."""

    def __init__(self):
        """Import the horse and rider glTFs, seat the rider and compute the rest transforms."""
        bs.reset_scene()
        self.harm, self.hmeshes, hroots = bs.import_glb(
            os.path.join(bs.ANIMALS, "horse.glb")
        )
        for r in hroots:
            r.scale = (bs.HORSE_SCALE,) * 3
        bs.rest_pose(self.harm)
        # Centre the horse between its fore and hind legs, hooves on the ground.
        front = bs.bone_world(self.harm, "FrontUpperLeg.L").to_translation()
        back = bs.bone_world(self.harm, "BackUpperLeg.L").to_translation()
        for r in hroots:
            r.location = (0.0, -(front.y + back.y) / 2, 0.0)
        bpy.context.view_layer.update()
        self.b_rest = bs.bone_world(self.harm, SADDLE_BONE).copy()
        # Seat: top of the back above the saddle bone.
        seat_y = bs.bone_world(self.harm, SADDLE_BONE).to_translation().y - 0.08
        top = 0.0
        for m in self.hmeshes:
            for v in m.data.vertices:
                w = m.matrix_world @ v.co
                if abs(w.y - seat_y) < 0.12 and abs(w.x) < 0.1:
                    top = max(top, w.z)
        self.seat = Vector((0.0, seat_y, top + 0.1))
        self.rarm, meshes, rroots = bs.import_glb(
            os.path.join(bs.CHARS, "adventurer.glb")
        )
        for m in meshes:
            bpy.data.objects.remove(m)
        for r in rroots:
            r.scale = (bs.HUMAN_SCALE,) * 3
        bs.rest_pose(self.rarm)
        hips = bs.bone_world(self.rarm, "Body").to_translation()
        for r in rroots:
            r.location = self.seat - hips
        bpy.context.view_layer.update()
        self.r_rest = self.rarm.matrix_world.copy()
        # FG4: stirrups 6 cm wider than the Quaternius mount's (0.30 m) so that the rider's
        # legs clear the fine horse's broader barrel (same bones, same clips; the thinner
        # Quaternius horse just shows a little air between boot and flank).
        self.stirrups = {
            s: self.seat + Vector((STIRRUP_HALF_WIDTH * sx, -0.08, -0.84))
            for s, sx in (("L", 1), ("R", -1))
        }
        self.pommel = self.seat + Vector((0.0, -0.3, 0.12))
        poses.RIDE.update({"mount": self})

    def delta(self):
        """Current saddle transform relative to the rest pose."""
        return (
            bs.bone_world(self.harm, SADDLE_BONE, posed=True) @ self.b_rest.inverted()
        )

    def seat_rider(self, placement=None):
        """Put the rider on the saddle (or at `placement`, a world matrix delta)."""
        self.rarm.matrix_world = (
            placement if placement is not None else self.delta()
        ) @ self.r_rest
        bpy.context.view_layer.update()


def horse_bones(mount):
    """Horse bones that carry weights, in armature order."""
    used = set()
    for m in mount.hmeshes:
        for v in m.data.vertices:
            for g in v.groups:
                if g.weight > 0:
                    used.add(m.vertex_groups[g.group].name)
    return [b.name for b in mount.harm.data.bones if b.name in used]


def cavalry_alias(name):
    """Map a cavalry bone name to its export alias, keeping the rider `R:` prefix."""
    if name.startswith("R:"):
        return "R:" + bs.human_bone_alias(name[2:])
    return name


def clip_specs():
    """(clip, horse action, rider action, rider pose, mirror, frames)."""
    return gaits.free_rows(_clip_rows())


def _clip_rows():
    """Rows of `clip_specs` before the AS8b gallops are substituted."""
    return (
        [
            ("c_idle", "Idle", "Idle", poses.ride_lance_up, False, None),
            ("c_walk", "Walk", "Idle", poses.ride_lance_up, False, None),
            ("c_gallop", "Gallop", "Idle", poses.ride_lance_raised, False, None),
            ("c_charge", "Gallop", "Idle", poses.ride_lance_couched, False, None),
            ("c_thrust", "Idle_2", "Idle", poses.ride_lance_thrust, False, 28),
            ("c_bow_idle", "Idle", "Idle", poses.ride_bow_rest, False, None),
            ("c_bow_walk", "Walk", "Idle", poses.ride_bow_rest, False, None),
            ("c_bow_shoot", "Idle", "Idle", poses.ride_bow_shoot, False, 60),
            # UR2: jinetes throw javelins rather than shoot a bow (skirmish ability).
            ("c_javelin_idle", "Idle", "Idle", poses.ride_javelin_rest, False, None),
            ("c_javelin_walk", "Walk", "Idle", poses.ride_javelin_rest, False, None),
            ("c_javelin_throw", "Idle", "Idle", poses.ride_javelin_throw, False, 30),
            ("c_death", "Death", "Death", poses.ride_death, False, 30),
            ("c_death_m", "Death", "Death", poses.ride_death, True, 30),
            # Lot AS3: the unhorsed rider falls as before, then the clip goes on for the horse that bolts
            # riderless (startle, then gallop; the shader carries it away at code 6).
            (
                "c_fall",
                "Idle_HitReact_Right",
                "Death",
                gaits.ride_fall_free,
                False,
                gaits.FALL_FRAMES,
            ),
            # Lot EP5: mounted standard bearer (appended: earlier rows keep their place).
            ("c_std_idle", "Idle", "Idle", poses.ride_std_up, False, None),
            ("c_std_walk", "Walk", "Idle", poses.ride_std_up, False, None),
            ("c_std_gallop", "Gallop", "Idle", poses.ride_std_gallop, False, None),
            ("c_std_wave", "Idle_2", "Idle", poses.ride_std_wave, False, 58),
            ("c_std_death", "Death", "Death", poses.ride_std_death, False, 30),
            # Lot AN1b: horse rearing before pikes, stumbling at the charge, mounted victory
            # (the rider poses carry a `horse` override run before the rider is seated).
            ("c_rear", "Idle", "Idle", poses.ride_rear, False, 32),
            ("c_stumble", "Gallop", "Idle", poses.ride_stumble, False, 90),
            ("c_victory", "Idle_2", "Idle", poses.ride_victory, False, None),
            # Lot NT7: mounted standard bearer at the charge (two gallop strides).
            ("c_std_charge", "Gallop", "Idle", poses.ride_std_charge, False, 30),
        ]
        + gaits.clip_specs()
    )  # Lot AS3: trot, turns (appended: earlier rows keep their place)


def bake_cavalry_rig():
    """Bone texture of the mounted figures."""
    mount = Mount()
    rig = bs.Rig("cavalry")
    for b in horse_bones(mount):
        rig.add(mount.harm, b, b)
    for b in bs.HUMAN_BONES:
        rig.add(mount.rarm, b, "R:" + b)
    bs.add_human_virtuals(rig, mount.rarm, prefix="R:")
    rig.capture_rest()
    for clip, horse_act, rider_act, pose, mirror, frames in clip_specs():
        start = rig.begin_clip()
        h_act = bs.find_action(horse_act, "AnimalArmature")
        r_act = bs.find_action(rider_act, "CharacterArmature")
        bs.set_action(mount.harm, h_act)
        bs.set_action(mount.rarm, r_act)
        first, last = (int(round(v)) for v in h_act.frame_range)
        length = last - first + 1
        count = frames or length
        for i in range(count):
            t = i / max(count - 1, 1)
            for arm in (mount.harm, mount.rarm):
                for pb in arm.pose.bones:
                    pb.matrix_basis.identity()
            bpy.context.scene.frame_set(
                first
                + (
                    i % length
                    if frames is None or rider_act != "Death"
                    else min(i, length - 1)
                )
            )
            poses.reset_state()
            horse_pose = getattr(pose, "horse", None)
            if horse_pose is not None:
                horse_pose(mount.harm, t)
            mount.seat_rider()
            pose(mount.rarm, t)
            bpy.context.view_layer.update()
            mats = rig.frame_matrices()
            rig.add_frame(rig.mirrored(mats) if mirror else mats)
        rig.end_clip(
            clip,
            start,
            clip
            in (
                "c_idle",
                "c_walk",
                "c_gallop",
                "c_charge",
                "c_bow_idle",
                "c_bow_walk",
                "c_javelin_idle",
                "c_javelin_walk",
                "c_std_idle",
                "c_std_walk",
                "c_std_gallop",
                "c_std_wave",
                "c_victory",
                "c_std_charge",
                *gaits.loop_names(),
            ),
        )
    rig.write()
    return rig


# --- Mesh ---------------------------------------------------------------------------------


def _rename_groups(obj, prefix):
    for g in obj.vertex_groups:
        if not g.name.startswith(prefix):
            g.name = prefix + g.name


def caparison(mount, ctx, hem=0.5):
    """Trapper hanging from the horse's back to `hem` metres, arms painted on both flanks."""
    body = []
    for m in mount.hmeshes:
        names = {g.index: g.name for g in m.vertex_groups}
        for v in m.data.vertices:
            best = max(v.groups, key=lambda g: g.weight, default=None)
            if best is not None and names[best.group] in TORSO_BONES:
                body.append(
                    (
                        m.matrix_world @ v.co,
                        [(names[g.group], g.weight) for g in v.groups if g.weight > 0],
                    )
                )
    ys = [p.y for p, _w in body]
    y0, y1 = min(ys) + 0.1, max(ys) - 0.12
    stations = ctx.seg(9, 5, 3)
    around = ctx.seg(12, 6, 4)
    bm = bmesh.new()
    rings = []
    for k in range(stations + 1):
        y = y0 + (y1 - y0) * k / stations
        near = [p for p, _w in body if abs(p.y - y) < 0.12]
        half = max((abs(p.x) for p in near), default=0.3) + 0.05
        top = max((p.z for p in near), default=1.4) + 0.04
        ring = []
        for j in range(around + 1):
            a = math.pi * j / around  # 0 = left hem, pi = right hem
            side = math.cos(a)
            lift = math.sin(a)
            if lift > 0.5:
                z = top - (1 - lift) * 0.35
                x = half * side * 1.05
            else:
                z = hem + (top - 0.35 - hem) * (lift / 0.5)
                x = (
                    half
                    * (1.0 if side > 0 else -1.0)
                    * (1.02 + 0.06 * (1 - lift / 0.5))
                )
            ring.append(bm.verts.new(Vector((x, y, z))))
        rings.append(ring)
    for a, b in zip(rings, rings[1:], strict=False):
        for j in range(around):
            bm.faces.new((a[j], a[j + 1], b[j + 1], b[j]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for f in bm.faces:
        if (
            f.normal.dot(
                f.calc_center_median() - Vector((0, f.calc_center_median().y, 1.0))
            )
            < 0
        ):
            f.normal_flip()
    uv = bm.loops.layers.uv.new("UVMap")
    ymid = (y0 + y1) / 2
    zmid = hem + 0.45
    for f in bm.faces:
        for loop in f.loops:
            p = loop.vert.co
            u = (p.y - ymid) / 0.7 + 0.5
            loop[uv].uv = (u if p.x > 0 else 1 - u, 0.5 - (p.z - zmid) / 0.7)
    obj = eq.to_object("caparison", bm, [ctx.material(eq.C_ARMS, (1, 1, 1))])
    tree = KDTree(len(body))
    for i, (p, _w) in enumerate(body):
        tree.insert(p, i)
    tree.balance()
    groups = {}
    for v in obj.data.vertices:
        p = obj.matrix_world @ v.co
        _co, idx, _d = tree.find(Vector((p.x * 0.6, p.y, max(p.z, 1.0))))
        for name, w in body[idx][1]:
            if name not in groups:
                groups[name] = obj.vertex_groups.new(name=name)
            groups[name].add([v.index], w, "REPLACE")
    return [obj]


def _bone_points(mount, bones):
    """World positions of the horse vertices dominated by one of `bones`."""
    out = []
    for m in mount.hmeshes:
        names = {g.index: g.name for g in m.vertex_groups}
        for v in m.data.vertices:
            best = max(v.groups, key=lambda g: g.weight, default=None)
            if best is not None and names[best.group] in bones:
                out.append(m.matrix_world @ v.co)
    return out


BARD_STEEL = (0.62, 0.63, 0.66)


def _conformal_plate(mount, ctx, name, bones, keep, offset, bind):
    """Steel bard shaped on the horse: copies of the (decimated) horse faces dominated by
    `bones` for which `keep(centre, normal)` holds, pushed out by `offset` along the
    vertex normals and weighted like the horse vertices they come from.
    """
    bm = bmesh.new()
    weights = []
    for m in mount.hmeshes:
        names = {g.index: g.name for g in m.vertex_groups}
        mw = m.matrix_world
        rot = mw.to_3x3()
        me = m.data
        dominant = {}
        for v in me.vertices:
            best = max(v.groups, key=lambda g: g.weight, default=None)
            dominant[v.index] = names.get(best.group) if best is not None else None
        remap = {}
        for poly in me.polygons:
            if not all(dominant[i] in bones for i in poly.vertices):
                continue
            centre = mw @ poly.center
            normal = (rot @ poly.normal).normalized()
            if not keep(centre, normal):
                continue
            verts = []
            for i in poly.vertices:
                if i not in remap:
                    v = me.vertices[i]
                    n = (rot @ v.normal).normalized()
                    remap[i] = bm.verts.new(mw @ v.co + n * offset)
                    weights.append(
                        [(names[g.group], g.weight) for g in v.groups if g.weight > 0]
                    )
                verts.append(remap[i])
            try:
                bm.faces.new(verts)
            except ValueError:
                pass
    bm.verts.index_update()
    obj = eq.to_object(name, bm, [ctx.material(eq.C_PLATE, BARD_STEEL)])
    if bind is not None:
        eq.bind_rigid(obj, bind)
        return [obj]
    groups = {}
    for vi, ws in enumerate(weights):
        for gname, w in ws:
            if gname not in groups:
                groups[gname] = obj.vertex_groups.new(name=gname)
            groups[gname].add([vi], w, "REPLACE")
    return [obj]


def chanfron(mount, ctx):
    """Chanfron (chanfrein, lot UR1): steel plate over the brow and the face."""
    pts = _bone_points(mount, {"Head"})
    if not pts:
        return []
    top = max(p.z for p in pts)
    low = min(p.z for p in pts)
    y_front = min(p.y for p in pts)

    def keep(c, n):
        # Front and top of the face, not the jaw nor the nostrils.
        return (
            (n.z > 0.25 or n.y < -0.5)
            and c.z > low + (top - low) * 0.35
            and c.y > y_front + 0.04
        )

    return _conformal_plate(mount, ctx, "chanfron", {"Head"}, keep, 0.012, "Head")


def flanchards(mount, ctx):
    """Flanchards (flançois, lot UR1): steel plates on both flanks behind the saddle."""
    seat = mount.seat

    def keep(c, n):
        return (
            abs(n.x) > 0.45
            and seat.y - 0.05 < c.y < seat.y + 0.45
            and seat.z - 0.62 < c.z < seat.z - 0.18
        )

    return _conformal_plate(mount, ctx, "flanchards", TORSO_BONES, keep, 0.02, None)


def saddle(mount, ctx):
    """Build the saddle and stirrup leathers as boxes anchored on the mount's seat."""
    bm = bmesh.new()
    eq.box(bm, mount.seat + Vector((0, 0.02, -0.08)), (0.2, 0.26, 0.05), mat=0)
    eq.box(bm, mount.seat + Vector((0, -0.24, 0.0)), (0.16, 0.03, 0.09), mat=0)
    eq.box(bm, mount.seat + Vector((0, 0.26, -0.02)), (0.18, 0.03, 0.07), mat=0)
    obj = eq.to_object("saddle", bm, [ctx.material(eq.C_LEATHER, (0.12, 0.06, 0.03))])
    eq.bind_rigid(obj, SADDLE_BONE)
    return [obj]


def lance(ctx, pennon=True):
    """Lance on the rider's `Prop` (3.6 m, grip 1.1 m from the butt), livery pennon."""
    fr = weapons.prop_frame(ctx)
    bm = bmesh.new()
    n = ctx.seg(6, 4, 3)
    at = weapons._at
    eq.tube(bm, at(fr, (0, -1.1, 0)), at(fr, (0, 2.4, 0)), 0.03, 0.018, n, 0)
    eq.tube(
        bm, at(fr, (0, -0.1, 0)), at(fr, (0, 0.05, 0)), 0.05, 0.03, n, 0
    )  # vamplate
    eq.tube(
        bm,
        at(fr, (0, 2.4, 0)),
        at(fr, (0, 2.62, 0)),
        0.02,
        0.002,
        max(n - 2, 3),
        1,
        caps=False,
    )
    if pennon and ctx.level < 2:
        o, x, y, z = fr
        p0 = at(fr, (0, 2.3, 0))
        p1 = at(fr, (0, 1.9, 0))
        tail = at(fr, (0, 2.0, 0)) + (x * 0.0 - z * 0.0)
        for flip in (False, True):
            tri = [
                bm.verts.new(p0),
                bm.verts.new(p1),
                bm.verts.new(tail + x * 0.45 + z * (0.002 if flip else 0.0)),
            ]
            bm.faces.new(tri[::-1] if flip else tri).material_index = 2
    eq.finish(bm)
    obj = eq.to_object(
        "lance",
        bm,
        [
            ctx.material(eq.C_WOOD, eq.WOOD_LIGHT),
            ctx.material(eq.C_PLATE, eq.IRON),
            ctx.material(eq.C_LIVERY, (1, 1, 1)),
        ],
    )
    eq.bind_rigid(obj, "Prop")
    return [obj]


def build_cavalry(recipe, level):
    """Horse, harness and rider of a mounted figure at `level`."""
    sources = {}
    for file_name, part_names in recipe["parts"]:
        parts = bs.extract_parts(os.path.join(bs.CHARS, file_name))
        for pn in part_names:
            sources[pn] = parts[pn]
    mount = Mount()
    out = []
    hb = recipe["horse_budget"][level]
    for m in mount.hmeshes:
        for mod in list(m.modifiers):
            m.modifiers.remove(mod)
        bs.recolor(m, HORSE_COLORS, {})
        bs.weld_and_decimate(m, hb)
        bs.set_face_mask(m, 0)
        out.append(m)
    ctx_h = eq.Context(mount.harm, level, bs.material, bs.bone_world)
    for item in recipe.get("horse_equipment", []):
        name, mask = item[0], item[1]  # CR3: optional kwargs (fine pipeline only)
        builder = {
            "caparison": lambda c: caparison(mount, c),
            "chanfron": lambda c: chanfron(mount, c),
            "flanchards": lambda c: flanchards(mount, c),
            "saddle": lambda c: saddle(mount, c),
        }[name]
        for obj in builder(ctx_h):
            bs.set_face_mask(obj, mask)
            out.append(obj)
    # Rider: parts rebuilt on the rider armature (standing on the saddle at rest).
    rider = [bs.create_part(name, data, mount.rarm) for name, data in sources.items()]
    bpy.context.view_layer.update()
    budget = recipe["budget"][level]
    for m in rider:
        bs.recolor(m, recipe.get("colors", {}), eq.DEFAULT_COLORS)
        bs.weld_and_decimate(m, budget.get(bs.clean_name(m.name), budget.get("*", 0)))
        bs.set_face_mask(m, recipe.get("masks", {}).get(bs.clean_name(m.name), 0))
    ctx = eq.Context(mount.rarm, level, bs.material, bs.bone_world)
    for item in recipe.get("equipment", []):
        name, mask = item[0], item[1]
        kwargs = item[2] if len(item) > 2 else {}
        builder = (
            globals().get(name) or getattr(weapons, name, None) or getattr(eq, name)
        )
        for obj in builder(ctx, **kwargs):
            bs.set_face_mask(obj, mask)
            rider.append(obj)
    for m in rider:
        _rename_groups(m, "R:")
    return out + rider


def export_cavalry(fig_name, recipe, rig):
    """Export all LOD meshes of a mounted figure and return its figure manifest entry."""
    files, tris = [], []
    for level in range(3):
        objs = build_cavalry(recipe, level)
        name = f"{fig_name}_lod{level}.mesh.bin"
        tris.append(
            bs.export_mesh(
                objs,
                rig,
                cavalry_alias,
                os.path.join(bs.OUT_DIR, name),
                influences=bs.INFLUENCES[level],
            )
        )
        files.append(name)
    entry = {
        "rig": "cavalry",
        "lods": files,
        "tris": tris,
        "variants": recipe.get("variants", 1),
        "style": recipe.get("style", ""),
        "noble": recipe.get("noble", False),
    }
    # Lot EP5: pole tip of a mounted standard bearer (rider armature of the last build).
    entry.update(bs.pole_entry(recipe, poses.RIDE["mount"].rarm))
    return entry


_ = Matrix  # re-exported for pose helpers
