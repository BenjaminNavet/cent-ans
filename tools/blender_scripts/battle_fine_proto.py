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
            s = head_scale * scale if b.name == "head" else scale
            sy = sxz = s
        else:
            loc = rest.to_translation() * scale + offset
            rot = rest_rot
            sy = sxz = scale
        m = Matrix.LocRotScale(loc, rot.to_quaternion(), Vector((sxz, sy, sxz)))
        posed[b.name] = m
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


def main():
    """Parse the step and run it."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = "/tmp/fg0"
    if "--out" in args:
        out = args[args.index("--out") + 1]
    os.makedirs(out, exist_ok=True)
    step = args[0] if args else "fit"
    {"fit": step_fit}[step](out)
    print("OK")


if __name__ == "__main__":
    main()
