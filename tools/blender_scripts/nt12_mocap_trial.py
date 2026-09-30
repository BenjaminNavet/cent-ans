"""Lot NT12: free motion capture trial, retargeted onto the fine battle figures.

Run from the repository root (CMU takes in ``$CENT_ANS_MOCAP_SRC/cmu``, default
``~/dev/cent-ans-mocap-src/cmu``, outside the repository)::

    blender -b --factory-startup --python tools/blender_scripts/nt12_mocap_trial.py
    blender -b --factory-startup --python tools/blender_scripts/nt12_mocap_trial.py -- render DIR

Source: CMU Graphics Lab Motion Capture Database (http://mocap.cs.cmu.edu, "free for all
uses", may be redistributed, not resold as is), ASF/AMC at 120 fps read by ``mocap_asf``.
Only the baked skinning frames leave this script (never the source takes).

Retargeting (rotation based, no IK): every source bone's global rotation ``S`` (rest =
identity) drives the target bone mapped to it in ``MAP``::

    target = Q · S · P⁻¹ · A · T

``P`` maps source axes to armature space (left / up vectors of both rest poses), ``Q = P · H``
also removes the take's heading at its first frame, ``T`` is the target bone's rest rotation and
``A`` the smallest rotation taking the target limb (joint to child joint) onto the source
limb at rest (T pose), so that the A-posed figure first adopts the source rest pose. Leaf
bones (head, wrists, feet) take their parent's ``A``. Positions follow the chain of
``MAP`` (the rig parents the feet to ``Root``, so they are placed from the shins); the body
height is scaled by the ratio of standing hip heights and the whole clip shifted so that the
lower foot of the first frame rests on the ground.

Output: ``game/assets/models/battle_fine/mocap_trial/`` — ``human.bones.bin`` (``CAB1``,
same bones as the fine ``human`` rig, only the substituted clips) and ``manifest.json``
(clip table, source of each clip, quality measures). ``BattleSkinned`` appends these rows to
the fine ``human`` texture and repoints the substituted clips with ``--mocap-trial`` after
``--`` (defaults unchanged without it).
"""

import json
import math
import os
import sys

import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_fine as bf  # noqa: E402
import battle_skinned as bs  # noqa: E402
import mocap_asf as asf  # noqa: E402

SRC_DIR = os.environ.get(
    "CENT_ANS_MOCAP_SRC", os.path.expanduser("~/dev/cent-ans-mocap-src")
)
OUT_DIR = os.path.join(bf.FINE_DIR, "mocap_trial")
STEP = asf.SOURCE_FPS // bs.FPS  # 120 fps -> 24 fps
GROUND_MARGIN = (
    0.07  # metres between a joint centre (other than the feet) and the ground
)
LOOP_BLEND = 6  # frames blended back to the first one at the end of a looped clip

# (clip substituted, subject, take, first and last source frame at 120 fps, loop, note)
CLIPS = [
    ("guard", "02", "02_09", 0, 120, True, "sword held low, calm (swordplay take 9)"),
    ("slash", "02", "02_08", 144, 288, False, "rising diagonal cut (swordplay take 8)"),
    ("overhead", "02", "02_07", 72, 216, False, "overhead cut (swordplay take 7)"),
    ("parry", "02", "02_07", 24, 120, False, "sword raised to a high guard (take 7)"),
    ("hit", "02", "02_09", 400, 480, False, "recoil and duck (take 9, no real impact)"),
    (
        "death",
        "90",
        "90_18",
        60,
        228,
        False,
        "rug pulled, falls on the back (take 90_18)",
    ),
]

# Clips whose left arm keeps the keyframed shield guard (see `shield_arm`).
SHIELD_CLIPS = {"guard", "slash", "overhead", "parry", "hit"}
SHIELD_ARM = ("Shoulder.L", "UpperArm.L", "LowerArm.L", "Wrist.L")

# target bone -> (source bone, target child joint giving the limb direction, chain parent)
MAP = {
    "Body": ("root", None, None),
    "Hips": ("root", None, "Body"),
    "Abdomen": ("lowerback", "Torso", "Hips"),
    "Torso": ("upperback", "Chest", "Abdomen"),
    "Chest": ("thorax", "Neck", "Torso"),
    "Neck": ("lowerneck", "Head", "Chest"),
    "Head": ("head", None, "Neck"),
    "Shoulder.L": ("lclavicle", "UpperArm.L", "Chest"),
    "UpperArm.L": ("lhumerus", "LowerArm.L", "Shoulder.L"),
    "LowerArm.L": ("lradius", "Wrist.L", "UpperArm.L"),
    "Wrist.L": ("lwrist", None, "LowerArm.L"),
    "Shoulder.R": ("rclavicle", "UpperArm.R", "Chest"),
    "UpperArm.R": ("rhumerus", "LowerArm.R", "Shoulder.R"),
    "LowerArm.R": ("rradius", "Wrist.R", "UpperArm.R"),
    "Wrist.R": ("rwrist", None, "LowerArm.R"),
    "UpperLeg.L": ("lfemur", "LowerLeg.L", "Body"),
    "LowerLeg.L": ("ltibia", "Foot.L", "UpperLeg.L"),
    "Foot.L": ("lfoot", None, "LowerLeg.L"),
    "UpperLeg.R": ("rfemur", "LowerLeg.R", "Body"),
    "LowerLeg.R": ("rtibia", "Foot.R", "UpperLeg.R"),
    "Foot.R": ("rfoot", None, "LowerLeg.R"),
}


def mat3(a):
    """`mathutils.Matrix` of a 3x3 numpy array."""
    return Matrix([list(map(float, row)) for row in a])


def vec(a):
    """`mathutils.Vector` of a numpy 3-vector."""
    return Vector([float(x) for x in a])


def frame_of(left, up):
    """Orthonormal frame (columns left, forward, up) from a left and an up vector."""
    up = up.normalized()
    left = (left - up * left.dot(up)).normalized()
    fwd = left.cross(up)
    return Matrix((left, fwd, up)).transposed()


class Target:
    """Rest data of the fine ``human`` armature (armature space)."""

    def __init__(self, arm):
        """Read rest matrices, heads and axes of `arm`."""
        self.arm = arm
        self.unit = arm.matrix_world.to_scale()[0] or 1.0  # metres per armature unit
        self.rest = {b.name: b.matrix_local.copy() for b in arm.data.bones}
        self.head = {n: m.to_translation() for n, m in self.rest.items()}
        self.up = (self.head["Head"] - self.head["Body"]).normalized()
        self.left = self.head["UpperLeg.L"] - self.head["UpperLeg.R"]
        self.frame = frame_of(self.left, self.up)
        feet = (self.head["Foot.L"] + self.head["Foot.R"]) * 0.5
        self.foot_rest = min(
            self.head["Foot.L"].dot(self.up), self.head["Foot.R"].dot(self.up)
        )
        self.body_height = (self.head["Body"] - feet).dot(self.up)
        self.shield_rel = shield_arm(arm)

    def limb(self, bone):
        """Rest direction from `bone`'s head to its child joint (None for leaves)."""
        child = MAP[bone][1]
        return (
            None if child is None else (self.head[child] - self.head[bone]).normalized()
        )


class Source:
    """One CMU subject (skeleton) and its rest measures."""

    def __init__(self, subject):
        """Parse ``<subject>.asf`` from the source folder."""
        self.skel = asf.Skeleton(os.path.join(SRC_DIR, "cmu", f"{subject}.asf"))
        rest = asf.rest(self.skel)
        self.up = Vector((0.0, 1.0, 0.0))
        self.left = vec(rest["lfemur"][1] - rest["rfemur"][1])
        self.frame = frame_of(self.left, self.up)
        ankle = min(rest["lfoot"][1][1], rest["rfoot"][1][1])
        self.hip_height = -ankle  # root above the ankles in the zero pose
        self.direction = {b.name: vec(b.direction) for b in self.skel.order[1:]}


def heading_fix(src, pose0):
    """Rotation about the source up axis cancelling the take's heading at its first frame."""
    left = vec(pose0["lfemur"][1] - pose0["rfemur"][1])
    left.y = 0.0
    rest = src.left.copy()
    rest.y = 0.0
    q = left.normalized().rotation_difference(rest.normalized())
    return q.to_matrix()


def alignments(tgt, src, p):
    """``A`` of every mapped bone (smallest rotation target limb -> source limb at rest)."""
    out = {}
    for bone, (sbone, child, parent) in MAP.items():
        if child is None:
            out[bone] = out[parent] if parent in out else Matrix.Identity(3)
            continue
        want = (p @ src.direction[sbone]).normalized()
        out[bone] = tgt.limb(bone).rotation_difference(want).to_matrix()
    return out


def shield_arm(arm):
    """Left arm of the keyframed guard relative to the chest (``SHIELD_ARM`` bones).

    CMU subject 2 fences with both hands on the hilt: on a sword-and-shield figure the left
    arm would carry the shield across the face. Clips of ``SHIELD_CLIPS`` keep the left arm
    of the keyframed guard (``Idle_Sword``, first frame) on the mocap chest instead.
    """
    import battle_skinned as bs_

    act = bs_.find_action("Idle_Sword", "CharacterArmature")
    bs_.set_action(arm, act)
    bpy.context.scene.frame_set(int(round(act.frame_range[0])))
    bpy.context.view_layer.update()
    chest = arm.pose.bones["Chest"].matrix.copy()
    rel = {b: chest.inverted() @ arm.pose.bones[b].matrix for b in SHIELD_ARM}
    bs_.rest_pose(arm)
    return rel


def solve_frame(tgt, src, pose, q, p, align, origin, scale, shield=False):
    """Armature-space pose matrix of every mapped bone for one source frame."""
    pinv = p.transposed()
    rot = {}
    pos = {}
    for bone, (sbone, _child, parent) in MAP.items():
        s = mat3(pose[sbone][0])
        rot[bone] = q @ s @ pinv @ align[bone] @ tgt.rest[bone].to_3x3()
        if parent is None:
            root = vec(pose["root"][1])
            flat = root - origin
            height = flat.y
            flat.y = 0.0
            pos[bone] = (
                tgt.head["Body"]
                - tgt.up * tgt.body_height
                + (q @ flat) * scale
                + tgt.up * (height * scale)
            )
        else:
            offset = tgt.rest[parent].to_3x3().transposed() @ (
                tgt.head[bone] - tgt.head[parent]
            )
            pos[bone] = pos[parent] + rot[parent] @ offset
    out = {b: Matrix.Translation(pos[b]) @ rot[b].to_4x4() for b in MAP}
    if shield:
        for b in SHIELD_ARM:
            out[b] = out["Chest"] @ tgt.shield_rel[b]
    return out


def to_basis(tgt, mats):
    """``matrix_basis`` of each bone giving the armature-space pose matrices `mats`."""
    out = {}
    posed = {}
    for pb in tgt.arm.pose.bones:
        name = pb.name
        rest = tgt.rest[name]
        parent = pb.parent.name if pb.parent else None
        if parent is None:
            parent_pose = Matrix.Identity(4)
            local = rest
        else:
            parent_pose = posed.get(parent)
            if parent_pose is None:
                parent_pose = tgt.rest[parent]
            local = tgt.rest[parent].inverted() @ rest
        want = mats.get(name)
        if want is None:
            posed[name] = parent_pose @ local
            continue
        out[name] = local.inverted() @ parent_pose.inverted() @ want
        posed[name] = want
    return out


def blend_basis(a, b, w):
    """Per-bone interpolation of two basis dictionaries (`w` = weight of `b`)."""
    out = {}
    for name, ma in a.items():
        la, ra, _sa = ma.decompose()
        lb, rb, _sb = b[name].decompose()
        out[name] = (
            Matrix.Translation(la.lerp(lb, w)) @ ra.slerp(rb, w).to_matrix().to_4x4()
        )
    return out


def clip_bases(tgt, clip):
    """Basis dictionaries (one per 24 fps frame) of a clip spec, and its setup."""
    name, subject, take, first, last, loop, _note = clip
    src = Source(subject)
    frames = asf.read_amc(os.path.join(SRC_DIR, "cmu", f"{take}.amc"))
    p = tgt.frame @ src.frame.transposed()
    pose0 = asf.pose(src.skel, frames[first])
    q = p @ heading_fix(src, pose0)
    align = alignments(tgt, src, p)
    scale = tgt.body_height / src.hip_height
    origin = vec(pose0["root"][1])
    origin.y = 0.0
    picks = list(range(first, min(last, len(frames) - 1) + 1, STEP))
    poses_ = [asf.pose(src.skel, frames[i]) for i in picks]
    if loop:  # no drift over a loop: the root ends where it started
        drift = vec(poses_[-1]["root"][1] - poses_[0]["root"][1])
        drift.y = 0.0
        for k, pz in enumerate(poses_):
            r, a, _b = pz["root"]
            fixed = a - np.array(drift) * (k / max(len(poses_) - 1, 1))
            pz["root"] = (r, fixed, fixed)
    shield = name in SHIELD_CLIPS
    solved = [
        solve_frame(tgt, src, pz, q, p, align, origin, scale, shield) for pz in poses_
    ]
    # Ground: the lower foot of the first frame rests where the rest feet are.
    low = min(solved[0][f].to_translation().dot(tgt.up) for f in ("Foot.L", "Foot.R"))
    shift = Matrix.Translation(tgt.up * (tgt.foot_rest - low))
    solved = [{b: shift @ m for b, m in s.items()} for s in solved]
    # No joint under the ground (falls): lift the frame by the deepest overshoot.
    lifted = []
    for s in solved:
        lift = 0.0
        for bone, (_sb, _c, _p) in MAP.items():
            margin = 0.0 if bone.startswith("Foot") else GROUND_MARGIN / tgt.unit
            depth = tgt.foot_rest + margin - s[bone].to_translation().dot(tgt.up)
            lift = max(lift, depth)
        up = Matrix.Translation(tgt.up * lift)
        lifted.append({b: up @ m for b, m in s.items()})
    solved = lifted
    bases = [to_basis(tgt, s) for s in solved]
    if loop and len(bases) > LOOP_BLEND + 1:
        n = len(bases)
        for j in range(LOOP_BLEND):
            k = n - LOOP_BLEND + j
            bases[k] = blend_basis(bases[k], bases[0], (j + 1) / (LOOP_BLEND + 1))
    return bases, solved


def quality(tgt, solved):
    """Measures of the retargeted clip: foot slide, ground penetration, wrist speed peaks."""
    up = tgt.up
    slide = 0.0
    for foot in ("Foot.L", "Foot.R"):
        planted = None
        for s in solved:
            p = s[foot].to_translation()
            if (
                p.dot(up) - tgt.foot_rest < 0.03 / tgt.unit
            ):  # ~3 cm above its rest height
                flat = p - up * p.dot(up)
                if planted is None:
                    planted = flat
                slide = max(slide, (flat - planted).length)
            else:
                planted = None
    below = (
        min(
            min(s[f].to_translation().dot(up) for f in ("Foot.L", "Foot.R"))
            for s in solved
        )
        - tgt.foot_rest
    )
    spin = 0.0
    for a, b in zip(solved, solved[1:], strict=False):
        qa = a["Wrist.R"].to_quaternion()
        qb = b["Wrist.R"].to_quaternion()
        angle = math.degrees(qa.rotation_difference(qb).angle)
        spin = max(spin, min(angle, 360.0 - angle))
    return {
        "foot_slide_m": round(slide * tgt.unit, 3),
        "foot_below_rest_m": round(below * tgt.unit, 3),
        "wrist_r_max_deg_per_frame": round(spin, 1),
    }


def bake():
    """Bake the substituted clips into ``OUT_DIR`` and write its manifest."""
    arm, _meshes = bf.load_fine_human()
    import battle_skinned_poses as poses

    tgt = Target(arm)
    rig = bs.Rig("human")
    for b in bs.HUMAN_BONES:
        rig.add(arm, b, b)
    bs.add_human_virtuals(rig, arm)
    rig.capture_rest()
    report = {}
    for clip in CLIPS:
        name, subject, take, first, last, loop, note = clip
        bases, solved = clip_bases(tgt, clip)
        start = rig.begin_clip()
        for basis in bases:
            for pb in arm.pose.bones:
                pb.matrix_basis = basis.get(pb.name, Matrix.Identity(4))
            poses.reset_state()
            bpy.context.view_layer.update()
            rig.add_frame(rig.frame_matrices())
        rig.end_clip(name, start, loop)
        q = quality(tgt, solved)
        report[name] = {
            "source": f"CMU {take}.amc frames {first}-{last} @120fps",
            "note": note,
            "quality": q,
        }
        print("QUALITY", name, json.dumps(q))
    os.makedirs(OUT_DIR, exist_ok=True)
    bs.OUT_DIR = OUT_DIR
    rig.write()
    manifest = rig.manifest()
    manifest["rig"] = "human"
    manifest["clip_sources"] = report
    manifest["source"] = (
        "tools/blender_scripts/nt12_mocap_trial.py - CMU Graphics Lab Motion Capture "
        "Database (mocap.cs.cmu.edu, NSF EIA-0196217), see docs/research/mocap-gratuite.md"
    )
    with open(os.path.join(OUT_DIR, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
    print("OK", OUT_DIR)


RENDER_FRACS = (0.0, 0.35, 0.7, 1.0)
RENDER_EYE = ((2.4, -4.2, 1.5), (0.0, -0.2, 0.85))


def render(out):
    """Workbench renders of each clip, keyframed (``k``) then mocap (``m``), on ``infantry_0``.

    Files ``<clip>_<k|m>_<i>.png`` in `out` (fractions ``RENDER_FRACS`` of the clip).
    """
    import battle_fine_figures as ff
    import battle_fine_proto as fp
    import battle_skinned_poses as poses

    os.makedirs(out, exist_ok=True)
    arm, objs, _recipe = ff.build_figure("infantry_0", 0)
    for o in objs:
        attr = o.data.attributes.get("vmask")
        for d in attr.data if attr else ():
            d.value &= 0b0011_1111
    objs = fp.apply_variant(objs, 0)
    ff._pose_objects(arm, objs)
    fp.PROBE["rig"] = fp.probe_rig(arm)
    fp.setup_workbench((360, 480))
    cam = fp.camera()
    fp.look_at(cam, RENDER_EYE[0], RENDER_EYE[1], 40)
    tgt = Target(arm)
    for clip in CLIPS:
        name = clip[0]
        bases, _solved = clip_bases(tgt, clip)
        for i, frac in enumerate(RENDER_FRACS):
            fp.pose_clip(arm, name, frac)
            fp.follow_prop(objs)
            fp.render(os.path.join(out, f"{name}_k_{i}.png"))
            basis = bases[int(round(frac * (len(bases) - 1)))]
            for pb in arm.pose.bones:
                pb.matrix_basis = basis.get(pb.name, Matrix.Identity(4))
            poses.reset_state()
            bpy.context.view_layer.update()
            fp.follow_prop(objs)
            fp.render(os.path.join(out, f"{name}_m_{i}.png"))


def main():
    """``bake`` (default) or ``render DIR`` after ``--``."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if args and args[0] == "render":
        render(args[1] if len(args) > 1 else "/tmp/nt12_render")
    else:
        bake()


if __name__ == "__main__":
    main()
