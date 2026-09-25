"""Lot FG0: finer battle figure prototype and style sheet (not wired into the game).

Run from the repository root:

    blender --background --factory-startup --python tools/blender_scripts/battle_fine_proto.py -- <step>

Steps (after ``--``):
    fit       fit the MakeHuman body to the ``human`` rig, render the clip test sheet
    ...       (more steps are added as the prototype grows, see docs/wip/fg0-prototype.md)

Method (human):
1. The CC0 MakeHuman body (``fg_base_male.blend``, made by ``fg_makehuman_base.py``) carries
   MPFB's ``game_engine`` armature and weights.
2. That armature is posed so that each of its joints lands on the matching joint of the
   Quaternius ``human`` rig at its bind pose (the posed stance of ``adventurer.glb``); the pose
   is applied to the mesh, which then *is* in the Quaternius bind pose.
3. The MakeHuman weights are renamed to the Quaternius bones (pelvis -> Body/Hips, fingers ->
   Wrist...), so the mesh deforms with the unchanged ``human`` bone texture and all its clips.

Nothing here writes to ``game/``: outputs go to ``docs/img/fg/`` (renders) and to a scratch
directory given by ``--out`` (default ``/tmp/fg0``).
"""

import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned as bs  # noqa: E402

ROOT = bs.ROOT
MH_BLEND = os.path.join(
    ROOT, "game", "assets", "third_party", "characters", "makehuman_base", "fg_base_male.blend"
)
IMG_DIR = os.path.join(ROOT, "docs", "img", "fg")

# MakeHuman game_engine bone -> (Quaternius joint at the bone head, joint the bone points to).
# Only the limbs are fitted joint to joint; the trunk, neck and head keep their MakeHuman
# shape (uniform scale, placed so that the hip joints match): their joints are spaced
# differently from the Quaternius spine, and stretching them bone by bone gives a slab.
CHAIN = {
    "clavicle_l": ("Shoulder.L", "UpperArm.L"),
    "upperarm_l": ("UpperArm.L", "LowerArm.L"),
    "lowerarm_l": ("LowerArm.L", "Wrist.L"),
    "clavicle_r": ("Shoulder.R", "UpperArm.R"),
    "upperarm_r": ("UpperArm.R", "LowerArm.R"),
    "lowerarm_r": ("LowerArm.R", "Wrist.R"),
    "thigh_l": ("UpperLeg.L", "LowerLeg.L"),
    "calf_l": ("LowerLeg.L", "ankle.L"),
    "thigh_r": ("UpperLeg.R", "LowerLeg.R"),
    "calf_r": ("LowerLeg.R", "ankle.R"),
}

# Closed fists at bind pose (degrees about each finger bone's X axis): the figures hold
# a weapon or a shield's enarmes in every clip.
FIST_CURL = {
    "index_01": 50,
    "index_02": 80,
    "index_03": 55,
    "middle_01": 55,
    "middle_02": 80,
    "middle_03": 55,
    "ring_01": 60,
    "ring_02": 80,
    "ring_03": 55,
    "pinky_01": 65,
    "pinky_02": 80,
    "pinky_03": 55,
    "thumb_02": 25,
    "thumb_03": 35,
}
CURL_SIGN = 1.0
FIST = {}  # side -> (fist centre, grip axis from the little finger to the index), world

# Bones whose head is pinned on the Quaternius joint; the others hang from their parent.
ANCHORED = {"thigh_l", "thigh_r"}

# MakeHuman weight group -> {Quaternius bone: share}.
WEIGHT_MAP = {
    "pelvis": {"Body": 0.5, "Hips": 0.5},
    "spine_01": {"Abdomen": 1.0},
    "spine_02": {"Torso": 1.0},
    "spine_03": {"Chest": 1.0},
    "neck_01": {"Neck": 1.0},
    "head": {"Head": 1.0},
}
for _s in "lr":
    _S = _s.upper()
    WEIGHT_MAP.update(
        {
            f"clavicle_{_s}": {f"Shoulder.{_S}": 1.0},
            f"upperarm_{_s}": {f"UpperArm.{_S}": 1.0},
            f"lowerarm_{_s}": {f"LowerArm.{_S}": 1.0},
            f"hand_{_s}": {f"Wrist.{_S}": 1.0},
            f"thigh_{_s}": {f"UpperLeg.{_S}": 1.0},
            f"calf_{_s}": {f"LowerLeg.{_S}": 1.0},
            f"foot_{_s}": {f"Foot.{_S}": 1.0},
            f"ball_{_s}": {f"Foot.{_S}": 1.0},
        }
    )
    for _f in ("thumb", "index", "middle", "ring", "pinky"):
        for _k in ("01", "02", "03"):
            WEIGHT_MAP[f"{_f}_{_k}_{_s}"] = {f"Wrist.{_S}": 1.0}

# Clips checked by the fit test: (label, clip of human_clip_specs, frames as fractions).
TEST_CLIPS = [
    ("marche", "walk", (0.0, 0.25, 0.5)),
    ("estoc", "thrust", (0.0, 0.4, 0.6)),
    ("arc long", "bow_shoot", (0.1, 0.45, 0.6)),
    ("mort", "death", (0.2, 0.5, 1.0)),
]


# --- Quaternius rig ---------------------------------------------------------------------


def load_human_rig(keep_meshes=False):
    """Fresh scene with the Quaternius ``human`` armature at its bind pose.

    Returns (armature, meshes); meshes are deleted unless `keep_meshes`.
    """
    bs.reset_scene()
    arm, meshes, roots = bs.import_glb(os.path.join(bs.CHARS, "adventurer.glb"))
    for r in roots:
        r.scale = (bs.HUMAN_SCALE,) * 3
    bs.rest_pose(arm)
    if not keep_meshes:
        for m in meshes:
            bpy.data.objects.remove(m)
        meshes = []
    bpy.context.view_layer.update()
    return arm, meshes


def q_joints(arm, meshes=None):
    """World positions of the Quaternius joints at bind pose (plus estimated ankles)."""
    joints = {b: bs.bone_world(arm, b).to_translation() for b in bs.HUMAN_BONES}
    # No ankle bone: the shin/foot weight boundary of the Quaternius legs sits low above
    # the heel; the MakeHuman ankle is put at 8 cm above it, on the shin line.
    for side in "LR":
        heel = joints[f"Foot.{side}"]
        knee = joints[f"LowerLeg.{side}"]
        joints[f"ankle.{side}"] = Vector((heel.x, heel.y, heel.z)) + Vector(
            ((knee.x - heel.x) * 0.1, (knee.y - heel.y) * 0.1 - 0.035, 0.075)
        )
    return joints


# --- MakeHuman body ---------------------------------------------------------------------


def append_makehuman():
    """Append the MakeHuman body and its game_engine armature; returns (rig, body)."""
    with bpy.data.libraries.load(MH_BLEND, link=False) as (src, dst):
        dst.objects = [n for n in src.objects if n in ("MH_Body", "MH_Rig")]
    for obj in dst.objects:
        bpy.context.scene.collection.objects.link(obj)
    rig = bpy.data.objects["MH_Rig"]
    body = bpy.data.objects["MH_Body"]
    return rig, body


def _mh_head(rig, bone):
    return rig.matrix_world @ rig.data.bones[bone].head_local


def fit_scale(rig, joints):
    """Uniform scale bringing the MakeHuman trunk (hip joint to neck) to the rig's."""
    q = (joints["Neck"].z - joints["UpperLeg.L"].z + joints["Neck"].z - joints["UpperLeg.R"].z) / 2
    mh = _mh_head(rig, "neck_01").z - _mh_head(rig, "thigh_l").z
    return q / mh


def fit_pose(rig, joints, scale, head_scale=1.0):
    """Pose the MakeHuman armature onto the Quaternius joints (armature space)."""
    inv = rig.matrix_world.inverted()
    # Trunk placement: the scaled MakeHuman hip joints land on the Quaternius ones.
    q_hips = (inv @ joints["UpperLeg.L"] + inv @ joints["UpperLeg.R"]) / 2
    mh_hips = (rig.data.bones["thigh_l"].head_local + rig.data.bones["thigh_r"].head_local) / 2
    offset = q_hips - mh_hips * scale
    posed = {}  # bone -> armature-space pose matrix
    order = []

    def visit(b):
        order.append(b)
        for c in b.children:
            visit(c)

    for b in rig.data.bones:
        if b.parent is None:
            visit(b)
    for b in order:
        rest = b.matrix_local.copy()
        rest_rot = rest.to_3x3().normalized()
        if b.name in CHAIN:
            j0, j1 = CHAIN[b.name]
            if b.name in ANCHORED or b.parent is None or b.parent.name not in posed:
                h = inv @ joints[j0]
            else:
                # Connected chain: the head follows the parent, only the aim is fitted.
                p = b.parent
                h = (posed[p.name] @ p.matrix_local.inverted() @ rest).to_translation()
            t = inv @ joints[j1]
            d_rest = rest_rot.col[1].normalized()
            d_new = t - h
            rot = d_rest.rotation_difference(d_new.normalized()).to_matrix() @ rest_rot
            sy = d_new.length / b.length
            sxz = scale
            loc = h
        elif b.parent is not None and b.parent.name in posed:
            p = b.parent
            delta = posed[p.name] @ p.matrix_local.inverted()
            m = delta @ rest
            loc = m.to_translation()
            prot = posed[p.name].to_3x3().normalized() @ p.matrix_local.to_3x3().normalized().inverted()
            rot = prot @ rest_rot
            curl = FIST_CURL.get(b.name[:-2] if b.name[-2:] in ("_l", "_r") else "", 0.0)
            if curl:
                rot = rot @ Matrix.Rotation(math.radians(curl * CURL_SIGN), 3, "X")
            s = head_scale * scale if b.name == "head" else scale
            sy = sxz = s
        else:
            loc = rest.to_translation() * scale + offset
            rot = rest_rot
            sy = sxz = scale
        m = Matrix.LocRotScale(loc, rot.to_quaternion(), Vector((sxz, sy, sxz)))
        posed[b.name] = m
    mw = rig.matrix_world
    for s in "lr":
        idx = mw @ posed[f"index_01_{s}"].to_translation()
        pinky = mw @ posed[f"pinky_01_{s}"].to_translation()
        tip_b = rig.data.bones[f"middle_02_{s}"]
        tip = mw @ (posed[tip_b.name] @ Vector((0, tip_b.length, 0)))
        FIST[s.upper()] = ((idx + pinky) / 2 * 0.5 + tip * 0.5, (idx - pinky).normalized())
    return posed


def apply_pose_to_mesh(rig, body, posed):
    """Linear-blend-skin the body with the fitted bone matrices, then drop the MakeHuman rig.

    Done by hand rather than through a Blender pose: pose matrices with non-uniform scale
    do not survive Blender's parent-relative decomposition.
    """
    deform = {
        name: rig.matrix_world @ m @ rig.data.bones[name].matrix_local.inverted()
        @ rig.matrix_world.inverted()
        for name, m in posed.items()
    }
    names = {g.index: g.name for g in body.vertex_groups}
    mw = body.matrix_world
    mw_inv = mw.inverted()
    for v in body.data.vertices:
        world = mw @ v.co
        acc = Vector((0.0, 0.0, 0.0))
        total = 0.0
        for g in v.groups:
            m = deform.get(names[g.group])
            if m is None or g.weight <= 0:
                continue
            acc += (m @ world) * g.weight
            total += g.weight
        if total > 0:
            v.co = mw_inv @ (acc / total)
    body.modifiers.clear()
    body.parent = None
    body.matrix_world = mw
    bpy.data.objects.remove(rig)


def remap_weights(body):
    """Rename the MakeHuman weight groups to the Quaternius bones (summed, normalised)."""
    names = {g.index: g.name for g in body.vertex_groups}
    per_vertex = []
    for v in body.data.vertices:
        acc = {}
        for g in v.groups:
            share = WEIGHT_MAP.get(names[g.group])
            if not share or g.weight <= 0:
                continue
            for q, k in share.items():
                acc[q] = acc.get(q, 0.0) + g.weight * k
        total = sum(acc.values()) or 1.0
        per_vertex.append({q: w / total for q, w in acc.items()})
    kept = {"helper-l-eye", "helper-r-eye", "body"}
    for g in list(body.vertex_groups):
        if g.name not in kept:
            body.vertex_groups.remove(g)
    groups = {}
    for vi, ws in enumerate(per_vertex):
        if not ws:
            ws = {"Head": 1.0}
        for q, w in ws.items():
            if q not in groups:
                groups[q] = body.vertex_groups.new(name=q)
            groups[q].add([vi], w, "REPLACE")
    smooth_weights(body)


def smooth_weights(obj, factor=0.5, repeat=4, bones=("Chest", "Shoulder.L", "Shoulder.R", "UpperArm.L", "UpperArm.R", "Neck")):
    """Relax the weights around the shoulders (Laplacian, normalised).

    The Quaternius pivots of the chest and shoulders sit elsewhere than MakeHuman's; the
    sharp weight borders of the renamed groups then crease the shoulders in the thrust.
    """
    me = obj.data
    names = {g.index: g.name for g in obj.vertex_groups}
    index = {g.name: g.index for g in obj.vertex_groups}
    weights = [
        {names[g.group]: g.weight for g in v.groups if names[g.group] not in ("body",)}
        for v in me.vertices
    ]
    adj = [[] for _ in me.vertices]
    for e in me.edges:
        a, b = e.vertices
        adj[a].append(b)
        adj[b].append(a)
    region = [any(weights[i].get(bn, 0) > 0.01 for bn in bones) for i in range(len(weights))]
    for _ in range(repeat):
        new = []
        for i, w in enumerate(weights):
            if not region[i] or not adj[i]:
                new.append(w)
                continue
            acc = {k: v * (1 - factor) for k, v in w.items()}
            share = factor / len(adj[i])
            for j in adj[i]:
                for k, v in weights[j].items():
                    acc[k] = acc.get(k, 0.0) + v * share
            total = sum(acc.values()) or 1.0
            new.append({k: v / total for k, v in acc.items()})
        weights = new
    for i, w in enumerate(weights):
        if not region[i]:
            continue
        for k, v in w.items():
            if k not in index:
                index[k] = obj.vertex_groups.new(name=k).index
            obj.vertex_groups[index[k]].add([i], v, "REPLACE")


def attach(obj, arm):
    """Deform `obj` by the Quaternius armature (world transform kept)."""
    mw = obj.matrix_world.copy()
    obj.parent = arm
    obj.matrix_world = mw
    mod = obj.modifiers.new("arm", "ARMATURE")
    mod.object = arm


def fitted_body(head_scale=1.0):
    """Quaternius armature and the MakeHuman body fitted and skinned to it."""
    arm, meshes = load_human_rig(keep_meshes=True)
    joints = q_joints(arm, meshes)
    for m in meshes:
        bpy.data.objects.remove(m)
    rig, body = append_makehuman()
    scale = fit_scale(rig, joints)
    print(f"FIT scale={scale:.3f}")
    posed = fit_pose(rig, joints, scale, head_scale)
    apply_pose_to_mesh(rig, body, posed)
    remap_weights(body)
    attach(body, arm)
    return arm, body


# --- Clip test --------------------------------------------------------------------------


def pose_clip(arm, clip, frac):
    """Pose the armature at a fraction of a ``human`` clip (overrides included)."""
    import battle_skinned_poses as poses

    specs = {c[0]: c for c in bs.human_clip_specs()}
    _name, source, _loop, overrides, _mirror = specs[clip]
    act = bs.find_action(source, "CharacterArmature")
    bs.set_action(arm, act)
    first, last = (int(round(v)) for v in act.frame_range)
    count = getattr(overrides, "frames", None) or (last - first + 1)
    i = int(round(frac * (count - 1)))
    remap = getattr(overrides, "source_frame", None)
    f = remap(i, count, first, last) if remap else first + (i % (last - first + 1))
    for pb in arm.pose.bones:
        pb.matrix_basis.identity()
    bpy.context.scene.frame_set(f)
    poses.reset_state()
    if overrides:
        overrides(arm, i / max(count - 1, 1))
    # Freeze the pose: a render re-evaluates the action and would drop the overrides.
    arm.animation_data.action = None
    bpy.context.view_layer.update()


def prime_virtuals(arm):
    """Fill the rest matrices the pose overrides need (bow, prop)."""
    rig = bs.Rig("probe")
    bs.add_human_virtuals(rig, arm)


def setup_workbench(res=(420, 560)):
    """Quick solid render settings."""
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.render.resolution_x, scene.render.resolution_y = res
    scene.render.film_transparent = False
    scene.world = scene.world or bpy.data.worlds.new("w")
    return scene


def camera(name="cam"):
    """Scene camera (created once)."""
    scene = bpy.context.scene
    cam = bpy.data.objects.get(name)
    if cam is None:
        cam = bpy.data.objects.new(name, bpy.data.cameras.new(name))
        scene.collection.objects.link(cam)
    scene.camera = cam
    return cam


def look_at(cam, eye, target, lens=50.0):
    """Place the camera at `eye` looking at `target`."""
    cam.location = eye
    d = Vector(target) - Vector(eye)
    cam.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    cam.data.lens = lens


def render(path):
    """Render the current scene to `path`."""
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    print("RENDER", path)


def step_fit(out):
    """Fit the body and render it at bind pose and on the test clips."""
    arm, body = fitted_body()
    prime_virtuals(arm)
    mat = bpy.data.materials.new("skin")
    mat.diffuse_color = (0.75, 0.6, 0.5, 1)
    body.data.materials.clear()
    body.data.materials.append(mat)
    setup_workbench()
    cam = camera()
    bs.rest_pose(arm)
    look_at(cam, (1.8, -3.6, 1.1), (0, 0, 0.85), 55)
    render(os.path.join(out, "fit_bind.png"))
    look_at(cam, (0.0, -2.2, 1.25), (0, 0, 1.15), 60)
    render(os.path.join(out, "fit_bind_front.png"))
    look_at(cam, (0.0, 2.2, 1.25), (0, 0, 1.15), 60)
    render(os.path.join(out, "fit_bind_back.png"))
    for label, clip, fracs in TEST_CLIPS:
        for k, fr in enumerate(fracs):
            pose_clip(arm, clip, fr)
            look_at(cam, (1.6, -2.2, 1.4), (0, 0, 1.0), 45)
            if clip == "thrust":
                look_at(cam, (0.9, 1.6, 1.6), (0, 0, 1.25), 50)
            render(os.path.join(out, f"fit_{clip}_{k}.png"))
        print("CLIP", label, clip)


# --- Prototype man-at-arms ---------------------------------------------------------------

# LOD0 triangle budget per piece (0 = as modelled). The body keeps its head, neck and hands.
FOOT_BUDGET = {
    "MH_Body": 2600,
    "hauberk": 2600,
    "chausses": 1300,
    "shoes": 360,
    "surcoat": 1100,
}


def _tris(obj):
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def build_infantry():
    """Fitted body dressed as a man-at-arms c. 1340; returns (armature, objects)."""
    from mathutils.bvhtree import BVHTree

    import battle_fine_equipment as fe
    import battle_skinned_equipment as eq

    arm, body = fitted_body()
    lm = fe.Landmarks(body, arm)
    hauberk = fe.hauberk(body, lm)[0]
    objs = [hauberk]
    objs += fe.chausses(body, lm)
    bvh = BVHTree.FromObject(hauberk, bpy.context.evaluated_depsgraph_get())
    surcoat = fe.surcoat(body, lm, hauberk, bvh)
    objs += surcoat
    bvh_coat = BVHTree.FromObject(surcoat[0], bpy.context.evaluated_depsgraph_get())
    objs += fe.belt(lm, bvh)
    objs += fe.scabbard(lm)
    helm, frame = fe.bassinet(lm)
    objs += helm
    objs += fe.aventail(lm, frame, bvh, extra=(bvh_coat,))
    ctx = eq.Context(arm, 0, bs.material, bs.bone_world)
    objs += fe.sword(ctx, fist=FIST.get("R"))
    objs += fe.heater_shield(ctx)
    fe.trim_body(body)
    fe.assign_body_materials(body)
    objs.insert(0, body)
    for o in objs:
        target = FOOT_BUDGET.get(o.name.split(".")[0], 0)
        tris = bs.weld_and_decimate(o, target) if target else _tris(o)
        print(f"PIECE {o.name} tris={tris}")
        if o.parent is None:
            attach(o, arm)
        o.data.shade_smooth()
        o.data.set_sharp_from_angle(angle=math.radians(60))
    print(f"FIGURE infantry_fine LOD0 tris={sum(_tris(o) for o in objs)}")
    return arm, objs


# --- Eevee look -------------------------------------------------------------------------

LIVERY = (0.431, 0.0097, 0.0242)  # gueules #B0182B, linear
OR = (0.888, 0.539, 0.0296)  # or #F2C230, linear

# Shader codes of the current pipeline -> (roughness, metallic) for the Blender renders.
CODE_LOOK = {
    0: (0.85, 0.0),
    1: (0.4, 1.0),
    2: (0.3, 1.0),
    3: (0.45, 1.0),
    4: (0.9, 0.0),
    7: (0.55, 0.0),
    8: (0.6, 0.0),
    9: (0.7, 0.0),
    11: (0.7, 0.0),
    12: (0.9, 0.0),
}


def _principled(m):
    m.use_nodes = True
    nt = m.node_tree
    bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
    return nt, bsdf


def _math(nt, op, a, b=None):
    n = nt.nodes.new("ShaderNodeMath")
    n.operation = op
    for i, x in enumerate((a, b)):
        if x is None:
            continue
        if isinstance(x, (int, float)):
            n.inputs[i].default_value = x
        else:
            nt.links.new(x, n.inputs[i])
    return n.outputs[0]


def heraldry_factor(nt, uv_name):
    """Chevron mask (1 on the charge) from a shield UV layer."""
    uv = nt.nodes.new("ShaderNodeUVMap")
    uv.uv_map = uv_name
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    nt.links.new(uv.outputs[0], sep.inputs[0])
    dist = _math(nt, "ABSOLUTE", _math(nt, "SUBTRACT", sep.outputs[0], 0.5))
    h = _math(nt, "MULTIPLY_ADD", dist, -1.3)
    nt.links.new(sep.outputs[1], h.node.inputs[2])
    return _math(nt, "MULTIPLY", _math(nt, "GREATER_THAN", h, 0.08), _math(nt, "LESS_THAN", h, 0.25))


def _heraldry(nt, bsdf, field, charge, uv_name="heraldry"):
    band = heraldry_factor(nt, uv_name)
    mix = nt.nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    nt.links.new(band, mix.inputs[0])
    mix.inputs[6].default_value = (*field, 1)
    mix.inputs[7].default_value = (*charge, 1)
    nt.links.new(mix.outputs[2], bsdf.inputs["Base Color"])


def look_fine(m, maps=None):
    """Eevee material of a prototype piece (optionally with the baked maps)."""
    name = m.get("fg", "")
    nt, bsdf = _principled(m)
    rgb = LIVERY if name == "livery" else tuple(m["rgb"])
    bsdf.inputs["Base Color"].default_value = (*rgb, 1)
    bsdf.inputs["Roughness"].default_value = m["rough"]
    bsdf.inputs["Metallic"].default_value = m["metal"]
    if name == "skin":
        bsdf.inputs["Subsurface Weight"].default_value = 0.15
        bsdf.inputs["Subsurface Radius"].default_value = (0.04, 0.015, 0.008)
    if name == "arms":
        _heraldry(nt, bsdf, LIVERY, OR)
    if maps:
        maps(nt, bsdf, name)


def look_current(m, uv_name="UVMap"):
    """Eevee material of a current figure's coded material (flat colour per code)."""
    if "code" not in m:
        return
    code = int(m["code"])
    nt, bsdf = _principled(m)
    rgb = LIVERY if code == 0 else tuple(m["rgb"])
    rough, metal = CODE_LOOK.get(code, (0.6, 0.0))
    bsdf.inputs["Base Color"].default_value = (*rgb, 1)
    bsdf.inputs["Roughness"].default_value = rough
    bsdf.inputs["Metallic"].default_value = metal
    if code == 6:
        _heraldry(nt, bsdf, LIVERY, OR, uv_name)


def setup_eevee(res=(800, 1000)):
    """Soft daylight studio: one warm sun, sky-coloured world, neutral floor."""
    scene = bpy.context.scene
    try:
        scene.render.engine = "BLENDER_EEVEE"
    except TypeError:
        scene.render.engine = "BLENDER_EEVEE_NEXT"
    scene.render.resolution_x, scene.render.resolution_y = res
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    if hasattr(scene.eevee, "taa_render_samples"):
        scene.eevee.taa_render_samples = 64
    world = scene.world or bpy.data.worlds.new("fg_world")
    scene.world = world
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs[0].default_value = (0.42, 0.45, 0.50, 1)
    bg.inputs[1].default_value = 0.9
    if bpy.data.objects.get("fg_sun") is None:
        light = bpy.data.lights.new("fg_sun", "SUN")
        light.energy = 3.2
        light.angle = math.radians(12)
        light.color = (1.0, 0.95, 0.86)
        sun = bpy.data.objects.new("fg_sun", light)
        scene.collection.objects.link(sun)
        sun.rotation_euler = (math.radians(52), 0, math.radians(-35))
    if bpy.data.objects.get("fg_floor") is None:
        me = bpy.data.meshes.new("fg_floor")
        s_ = 60.0
        me.from_pydata(
            [(-s_, -s_, 0), (s_, -s_, 0), (s_, s_, 0), (-s_, s_, 0)], [], [(0, 1, 2, 3)]
        )
        floor = bpy.data.objects.new("fg_floor", me)
        scene.collection.objects.link(floor)
        fm = bpy.data.materials.new("fg_floor")
        _nt, b = _principled(fm)
        b.inputs["Base Color"].default_value = (0.20, 0.19, 0.16, 1)
        b.inputs["Roughness"].default_value = 0.95
        me.materials.append(fm)
    scene.view_settings.view_transform = "AgX"
    return scene


VIEWS = {
    # name: (eye, target, lens, resolution)
    "face": ((0.0, -3.6, 1.05), (0.0, 0.0, 0.92), 50, (700, 1000)),
    "trois_quarts": ((-2.2, -2.8, 1.3), (0.0, 0.0, 0.9), 50, (700, 1000)),
    "tete": ((-0.35, -1.05, 1.62), (0.0, -0.05, 1.5), 85, (700, 700)),
}


def render_views(prefix, out, views=("face", "trois_quarts", "tete"), shift=Vector()):
    """Render the named views of the current scene to `out/<prefix>_<view>.png`."""
    cam = camera()
    paths = []
    for v in views:
        eye, target, lens, res = VIEWS[v]
        setup_eevee(res)
        look_at(cam, Vector(eye) + shift, Vector(target) + shift, lens)
        path = os.path.join(out, f"{prefix}_{v}.png")
        render(path)
        paths.append(path)
    return paths


def render_far(prefix, out, distance=30.0, target=(0, 0, 0.9)):
    """Game-like view at `distance` (70 degree FOV, 1920x1080), full frame."""
    cam = camera()
    setup_eevee((1920, 1080))
    target = Vector(target)
    eye = target + Vector((-0.45, -0.85, 0.35)).normalized() * distance
    look_at(cam, eye, target, 25.2)  # 25.2 mm on a 36 mm sensor = 70 degrees horizontal
    path = os.path.join(out, f"{prefix}_30m_full.png")
    render(path)
    return path


def step_infantry(out):
    """Prototype man-at-arms: build, count, render (no baked maps)."""
    build_infantry()
    for m in bpy.data.materials:
        if m.get("fg"):
            look_fine(m)
    render_views("proto_infantry", out)


def step_current_infantry(out):
    """Current ``infantry_0`` (LOD0) rendered with the same light."""
    import battle_skinned_figures as figures

    _arm, objs = bs.build_human(figures.FIGURES["infantry_0"], 0)
    for m in bpy.data.materials:
        look_current(m)
    print(f"FIGURE infantry_0 LOD0 tris={sum(_tris(o) for o in objs)}")
    render_views("current_infantry", out)


STEPS = {
    "fit": step_fit,
    "infantry": step_infantry,
    "current_infantry": step_current_infantry,
}


def main():
    """Parse the step and run it."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = "/tmp/fg0"
    if "--out" in args:
        out = args[args.index("--out") + 1]
    os.makedirs(out, exist_ok=True)
    step = args[0] if args else "fit"
    STEPS[step](out)
    print("OK")


if __name__ == "__main__":
    main()
