"""Lot NT13: the player's own phone videos turned into melee clips of the fine figures.

Run from the repository root, after ``tools/video_mocap/extract_pose.py`` wrote the pose
landmarks of each video to ``$CENT_ANS_MOCAP_SRC/work/poses/<video>.npz`` (default
``~/dev/cent-ans-mocap-src``, outside the repository: the videos are personal)::

    blender -b --factory-startup --python tools/blender_scripts/nt13_video_trial.py
    blender -b --factory-startup --python tools/blender_scripts/nt13_video_trial.py -- render DIR

Pipeline: ``video_mocap_clean`` (smoothing, constant limb lengths, foot contacts, root motion,
24 fps) then, per frame, bone rotations from the landmarks, retargeted onto the fine ``human``
rig with the NT12 machinery (``nt12_mocap_trial``: rest data, basis conversion, loop blend,
quality measures):

* pelvis and chest frames from the hips and shoulders (spine bones interpolated), head from
  the ears and nose;
* arms and legs: each limb pair is aimed at its landmarks with the elbow / knee hinge taken
  from the bend plane (from the parent when the limb is nearly straight);
* the right wrist is turned so that the sword's blade follows the stick, whose line is given
  by both wrists (both hands on the stick) and its tip side by the right hand's index and
  pinky landmarks;
* the left arm keeps the keyframed shield guard on the video chest (as in NT12: a stick held
  in both hands would carry the shield across the face);
* feet: two-bone IK pins each planted foot (contacts from the video) where it stands.

Output: ``game/assets/models/battle_fine/video_trial/`` (``human.bones.bin`` ``CAB1`` +
``manifest.json`` with the source and quality measures of each clip), read by
``BattleSkinned`` with ``--video-trial`` after ``--`` (defaults unchanged without it).
"""

import json
import math
import os
import sys
import time

import bpy
import numpy as np
from mathutils import Matrix, Quaternion, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_fine as bf  # noqa: E402
import battle_skinned as bs  # noqa: E402
import nt12_mocap_trial as nt12  # noqa: E402
import video_mocap_clean as vc  # noqa: E402

SRC_DIR = os.environ.get(
    "CENT_ANS_MOCAP_SRC", os.path.expanduser("~/dev/cent-ans-mocap-src")
)
POSE_DIR = os.path.join(SRC_DIR, "work", "poses")
OUT_DIR = os.path.join(bf.FINE_DIR, "video_trial")
LOOP_BLEND = 6
# Caps of the right wrist's turn away from the keyframed grip: bend, and twist about the forearm.
WRIST_SWING_DEG = 65.0
WRIST_TWIST_DEG = 110.0

# (clip substituted, video, first and last video frame at 30 fps, speed-up, loop,
#  yaw: direction of the video (degrees about up, 0 = towards the camera, 90 = the image's
#  right) that becomes the figure's forward, note)
CLIPS = [
    ("guard", "IMG_6455", 0, 48, 1.0, True, 45.0, "stick on the right shoulder, still"),
    ("overhead", "IMG_6455", 54, 135, 1.8, False, 45.0, "raised then cut down across"),
    ("slash", "IMG_6458", 12, 72, 1.4, False, 45.0, "horizontal cut and back"),
    ("thrust", "IMG_6457", 57, 126, 1.4, False, 70.0, "two-handed lunge and back"),
]

SHIELD_ARM = nt12.SHIELD_ARM
LIMBS = {
    # upper bone, lower bone, landmarks (root, middle, end), rest joints, bend towards
    "arm.L": ("UpperArm.L", "LowerArm.L", (vc.SHOULDER_L, vc.ELBOW_L, vc.WRIST_L)),
    "arm.R": ("UpperArm.R", "LowerArm.R", (vc.SHOULDER_R, vc.ELBOW_R, vc.WRIST_R)),
    "leg.L": ("UpperLeg.L", "LowerLeg.L", (vc.HIP_L, vc.KNEE_L, vc.ANKLE_L)),
    "leg.R": ("UpperLeg.R", "LowerLeg.R", (vc.HIP_R, vc.KNEE_R, vc.ANKLE_R)),
}
CHILD = {
    "UpperArm.L": "LowerArm.L",
    "LowerArm.L": "Wrist.L",
    "UpperArm.R": "LowerArm.R",
    "LowerArm.R": "Wrist.R",
    "UpperLeg.L": "LowerLeg.L",
    "LowerLeg.L": "Foot.L",
    "UpperLeg.R": "LowerLeg.R",
    "LowerLeg.R": "Foot.R",
}
# Chain parents used to place the joints (same as NT12's MAP).
PARENT = {b: spec[2] for b, spec in nt12.MAP.items()}
ORDER = list(nt12.MAP.keys())


def vec(a):
    """`mathutils.Vector` of a numpy 3-vector."""
    return Vector([float(x) for x in a])


def basis(d, h):
    """Rotation whose columns are `d`, `h` made orthogonal to it, and their cross product."""
    d = d.normalized()
    h = h - d * h.dot(d)
    if h.length < 1e-8:
        h = d.orthogonal()
    h.normalize()
    return Matrix((d, h, d.cross(h))).transposed()


def smoothstep(a, b, x):
    """Hermite step from 0 at `a` to 1 at `b`."""
    t = min(max((x - a) / (b - a), 0.0), 1.0)
    return t * t * (3.0 - 2.0 * t)


def slerp_m(a, b, w):
    """Spherical interpolation of two rotation matrices."""
    return a.to_quaternion().slerp(b.to_quaternion(), w).to_matrix()


class Rest:
    """Rest frames of the target used by the landmark solver (armature space)."""

    def __init__(self, tgt, arm):
        """Joint positions, frames and the blade of the keyframed grip."""
        h = tgt.head
        self.tgt = tgt
        self.up = tgt.up
        self.left = tgt.frame.col[0].copy()
        self.fwd = tgt.frame.col[1].copy()
        self.hip_mid = (h["UpperLeg.L"] + h["UpperLeg.R"]) * 0.5
        self.sho_mid = (h["UpperArm.L"] + h["UpperArm.R"]) * 0.5
        spine = (self.sho_mid - self.hip_mid).normalized()
        self.pelvis = self._lu(self.left, (spine + self.up).normalized())
        self.chest = self._lu(h["UpperArm.L"] - h["UpperArm.R"], spine)
        self.head = self._lu(self.left, self.up)
        self.limb = {}
        for key, (upper, lower, _lm) in LIMBS.items():
            u0 = h[CHILD[upper]] - h[upper]
            l0 = h[CHILD[lower]] - h[lower]
            bend = self.fwd if key.startswith("arm") else -self.fwd
            hinge = u0.cross(bend).normalized()
            self.limb[key] = (u0.normalized(), l0.normalized(), hinge)
        self.foot = self._lu(self.left, self.up)
        # Keyframed grip: blade (prop Y) and hand (wrist bone Y) in the wrist's local frame.
        import battle_skinned_poses as poses

        prop_rest = arm.matrix_world.inverted() @ poses.REST["prop"]
        wrist = tgt.rest["Wrist.R"]
        k = (wrist.inverted() @ prop_rest).to_3x3().normalized()
        self.grip_blade = (k @ Vector((0.0, 1.0, 0.0))).normalized()
        self.grip_edge = (k @ Vector((0.0, 0.0, 1.0))).normalized()

    @staticmethod
    def _lu(left, up):
        """Frame (left, forward, up) from a left and an up vector."""
        return nt12.frame_of(left.copy(), up.copy())


def lu(left, up):
    """Frame (left, forward, up) from a left and an up vector."""
    return nt12.frame_of(left.copy(), up.copy())


class Clip:
    """One cleaned video clip mapped to armature space."""

    def __init__(self, tgt, spec):
        """Clean the landmarks of `spec` and map them onto the target's frame and size."""
        name, video, first, last, speed, loop, yaw, note = spec
        t0 = time.time()
        data = np.load(os.path.join(POSE_DIR, f"{video}.npz"))
        rate = float(data["fps"])
        world = data["world"][first : last + 1]
        image = data["image"][first : last + 1]
        cleaned = vc.clean(world, image, rate * speed, float(bs.FPS))
        pts = cleaned["points"]
        self.contacts = cleaned["contacts"]
        self.lengths = cleaned["lengths"]
        if loop:  # no drift over a loop: the root ends where it started
            drift = pts[-1].mean(axis=0) - pts[0].mean(axis=0)
            drift[2] = 0.0
            k = np.linspace(0.0, 1.0, len(pts))[:, None, None]
            pts = pts - drift[None, None, :] * k
        # Source frame: performer's left = image right (+X), forward = towards the camera.
        c, s = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
        fwd = Vector((s, -c, 0.0))
        left = Vector((c, s, 0.0))
        src = Matrix((left, fwd, Vector((0.0, 0.0, 1.0)))).transposed()
        rot = tgt.frame @ src.transposed()
        hip_height = float(np.median(cleaned["root"][:, 2]))
        scale = tgt.body_height / max(hip_height, 0.3)  # armature units per metre
        ground = tgt.head["Body"] - tgt.up * tgt.body_height
        r0 = cleaned["root"][0].copy()
        r0[2] = 0.0
        self.frames = [
            [ground + rot @ (vec(p - r0) * scale) for p in frame] for frame in pts
        ]
        self.rot = rot
        self.spec = spec
        self.rate = rate
        self.src_frames = (first, last)
        self.clean_s = time.time() - t0
        self.hip_height = hip_height
        self.scale = scale


def cap_swing_twist(q, axis, swing_max, twist_max):
    """`q` with its swing (off `axis`) and twist (about `axis`) angles capped, in degrees."""
    axis = axis.normalized()
    proj = axis * Vector(q[1:]).dot(axis)
    twist = Quaternion((q.w, proj.x, proj.y, proj.z))
    if twist.magnitude < 1e-9:
        twist = Quaternion()
    twist.normalize()
    swing = q @ twist.inverted()

    def cap(r, limit):
        angle = math.degrees(r.angle)
        if angle > 180.0:
            angle = 360.0 - angle
            r = Quaternion((-r.w, -r.x, -r.y, -r.z))
        if angle <= limit or r.axis.length < 1e-9:
            return r
        return Quaternion(r.axis, math.radians(limit))

    return cap(swing, swing_max) @ cap(twist, twist_max)


def limb_rotations(rest, key, j, parent_rot, prev_hinge):
    """Armature-space delta rotations of a limb pair (upper, lower) and the hinge used."""
    _upper, _lower, (a, b, c) = LIMBS[key]
    u0, l0, h0 = rest.limb[key]
    u = j[b] - j[a]
    lo = j[c] - j[b]
    carried = parent_rot @ h0
    bend = math.degrees(u.angle(lo, 0.0))
    hinge = carried - u.normalized() * carried.dot(u.normalized())
    if bend > 1.0:
        measured = u.cross(lo).normalized()
        w = smoothstep(8.0, 30.0, bend)
        if measured.dot(hinge) < 0.0 and w < 1.0:
            w *= 0.5  # unsure side on a nearly straight limb: lean on the carried hinge
        hinge = hinge.normalized() * (1.0 - w) + measured * w
    if prev_hinge is not None and hinge.dot(prev_hinge) < -0.2 and bend < 30.0:
        hinge = prev_hinge.copy()
    r_upper = basis(u, hinge) @ basis(u0, h0).transposed()
    r_lower = basis(lo, hinge) @ basis(l0, h0).transposed()
    return r_upper, r_lower, hinge.normalized()


def blade_directions(clip, min_gap):
    """Stick direction of every frame of `clip` (armature space).

    The line through both wrists (both hands hold the stick; the right hand's own landmarks
    when the hands are closer than `min_gap`, armature units) is kept continuous from frame to
    frame (a stick cannot swap ends in 1/24 s); its tip side is the one most frames' right
    hand points to (index and thumb side of the fist, landmarks too noisy frame by frame).
    """
    lines, votes = [], 0.0
    for j in clip.frames:
        side = j[vc.INDEX_R] - j[vc.PINKY_R]
        thumb = j[vc.THUMB_R] - j[vc.WRIST_R]
        hint = side.normalized() + thumb.normalized() * 0.5
        line = j[vc.WRIST_L] - j[vc.WRIST_R]
        line = hint.normalized() if line.length < min_gap else line.normalized()
        if lines and line.dot(lines[-1]) < 0.0:
            line = -line
        lines.append(line)
        votes += line.dot(hint.normalized())
    return lines if votes >= 0.0 else [-v for v in lines]


def blade_sides(clip, blades):
    """Roll reference of the fist about the blade, per frame: the forearm off the blade.

    Where the stick runs along the forearm this is ill-defined: such frames take the axis of
    the nearest well-defined frames (interpolated in time, made perpendicular to their own
    blade), so the fist never spins when the stick passes along the forearm.
    """
    raw, strength = [], []
    for j, blade in zip(clip.frames, blades, strict=True):
        fore = (j[vc.WRIST_R] - j[vc.ELBOW_R]).normalized()
        side = fore - blade * fore.dot(blade)
        raw.append(side.normalized() if side.length > 1e-6 else blade.orthogonal())
        strength.append(side.length)
    good = [i for i, s_ in enumerate(strength) if s_ >= 0.6]
    if not good:
        j = clip.frames[0]
        hinge = (j[vc.ELBOW_R] - j[vc.SHOULDER_R]).cross(j[vc.WRIST_R] - j[vc.ELBOW_R])
        good_axis = [hinge.normalized()] * len(blades)
    else:
        good_axis = []
        for i in range(len(blades)):
            before = [g for g in good if g <= i]
            after = [g for g in good if g >= i]
            if before and after and before[-1] != after[0]:
                p, n = before[-1], after[0]
                w = (i - p) / (n - p)
                good_axis.append(raw[p] * (1.0 - w) + raw[n] * w)
            else:
                good_axis.append(raw[before[-1] if before else after[0]])
    out = []
    for i, blade in enumerate(blades):
        filled = good_axis[i] - blade * good_axis[i].dot(blade)
        filled = filled.normalized() if filled.length > 1e-6 else raw[i]
        w = smoothstep(0.3, 0.6, strength[i])
        out.append((raw[i] * w + filled * (1.0 - w)).normalized())
    return out


def solve(rest, clip, index, state, blade, side):
    """Armature-space pose matrices of every mapped bone for one clip frame."""
    tgt = rest.tgt
    j = clip.frames[index]
    hip_mid = (j[vc.HIP_L] + j[vc.HIP_R]) * 0.5
    sho_mid = (j[vc.SHOULDER_L] + j[vc.SHOULDER_R]) * 0.5
    spine = (sho_mid - hip_mid).normalized()
    up = tgt.up
    r_pelvis = (
        lu(j[vc.HIP_L] - j[vc.HIP_R], (spine + up).normalized())
        @ rest.pelvis.transposed()
    )
    r_chest = lu(j[vc.SHOULDER_L] - j[vc.SHOULDER_R], spine) @ rest.chest.transposed()
    ear_mid = (j[vc.EAR_L] + j[vc.EAR_R]) * 0.5
    look = (j[vc.NOSE] - ear_mid) + up * (0.03 * clip.scale)
    head_left = j[vc.EAR_L] - j[vc.EAR_R]
    head_up = look.cross(head_left)
    r_head = lu(head_left, head_up) @ rest.head.transposed()
    r = {
        "Body": r_pelvis,
        "Hips": r_pelvis,
        "Abdomen": slerp_m(r_pelvis, r_chest, 0.33),
        "Torso": slerp_m(r_pelvis, r_chest, 0.66),
        "Chest": r_chest,
        "Neck": slerp_m(r_chest, r_head, 0.5),
        "Head": r_head,
        "Shoulder.L": r_chest,
        "Shoulder.R": r_chest,
    }
    for key in LIMBS:
        upper, lower, _lm = LIMBS[key]
        parent = r_chest if key.startswith("arm") else r_pelvis
        ru, rl, hinge = limb_rotations(rest, key, j, parent, state.get(key))
        state[key] = hinge
        r[upper] = ru
        r[lower] = rl
    r["Wrist.L"] = r["LowerArm.L"]
    # Right wrist: blade along the stick, hand as close as possible to the forearm line.
    local = basis(rest.grip_blade, Vector((0.0, 1.0, 0.0)))
    want = basis(blade, side)
    wrist_abs = want @ local.transposed()
    # Deviation from the keyframed hand-forearm relation, split into a twist about the
    # forearm (pronation, carried by the wrist bone here) and a swing (the fist bending),
    # each capped: a stick running along the forearm would otherwise fold the fist back.
    dev = (
        r["LowerArm.R"].inverted() @ wrist_abs @ tgt.rest["Wrist.R"].to_3x3().inverted()
    ).to_quaternion()
    dev = cap_swing_twist(dev, rest.limb["arm.R"][1], WRIST_SWING_DEG, WRIST_TWIST_DEG)
    r["Wrist.R"] = r["LowerArm.R"] @ dev.to_matrix()
    for side in ("L", "R"):
        heel = j[vc.HEEL_L if side == "L" else vc.HEEL_R]
        toe = j[vc.TOE_L if side == "L" else vc.TOE_R]
        foot_fwd = toe - heel
        foot_left = foot_fwd.cross(up)  # left = forward x up
        r["Foot." + side] = lu(foot_left, up) @ rest.foot.transposed()
    rot = {b: r[b] @ tgt.rest[b].to_3x3() for b in ORDER}
    wrist_rel = dev
    pos = {}
    for b in ORDER:
        p = PARENT[b]
        if p is None:
            pos[b] = hip_mid + (tgt.head["Body"] - rest.hip_mid)
        else:
            offset = tgt.rest[p].to_3x3().transposed() @ (tgt.head[b] - tgt.head[p])
            pos[b] = pos[p] + rot[p] @ offset
    return rot, pos, wrist_rel


def to_frame_coords(rest, v):
    """Coordinates (left, forward, up) of an armature-space vector."""
    return np.array(rest.tgt.frame.transposed() @ v)


def from_frame_coords(rest, c):
    """Armature-space vector of (left, forward, up) coordinates."""
    return rest.tgt.frame @ vec(c)


def pin_feet(rest, clip, rots, poss):
    """Two-bone IK keeping each planted foot where it stands; returns foot slide measures."""
    tgt = rest.tgt
    n = len(rots)
    for i, side in enumerate(("L", "R")):
        up_leg, low_leg, foot = f"UpperLeg.{side}", f"LowerLeg.{side}", f"Foot.{side}"
        ankle = np.array([to_frame_coords(rest, poss[t][foot]) for t in range(n)])
        ground = float(to_frame_coords(rest, tgt.head[foot])[2])
        targets, weights = vc.pin_targets(ankle, clip.contacts[:n, i], ground=ground)
        for t in range(n):
            want = ankle[t].copy()
            if weights[t] > 0.0:
                want = ankle[t] * (1.0 - weights[t]) + targets[t] * weights[t]
            want[2] = max(want[2], ground)
            if np.allclose(want, ankle[t], atol=1e-6):
                continue
            a = to_frame_coords(rest, poss[t][up_leg])
            b = to_frame_coords(rest, poss[t][low_leg])
            b2, c2 = vc.two_bone_ik(a, b, ankle[t], want)
            old_u = from_frame_coords(rest, b - a)
            new_u = from_frame_coords(rest, b2 - a)
            old_l = from_frame_coords(rest, ankle[t] - b)
            new_l = from_frame_coords(rest, c2 - b2)
            su = old_u.rotation_difference(new_u).to_matrix()
            sl = old_l.rotation_difference(new_l).to_matrix()
            rots[t][up_leg] = su @ rots[t][up_leg]
            rots[t][low_leg] = sl @ rots[t][low_leg]
            poss[t][low_leg] = from_frame_coords(rest, b2)
            poss[t][foot] = from_frame_coords(rest, c2)


def solve_clip(rest, clip):
    """Pose matrices (armature space) of every frame of a clip, feet pinned."""
    state = {}
    rots, poss, wrists = [], [], []
    blades = blade_directions(clip, 0.12 * clip.scale)
    sides = blade_sides(clip, blades)
    for i in range(len(clip.frames)):
        rot, pos, wrist = solve(rest, clip, i, state, blades[i], sides[i])
        rots.append(rot)
        poss.append(pos)
        wrists.append(wrist)
    pin_feet(rest, clip, rots, poss)
    out = []
    for rot, pos in zip(rots, poss, strict=True):
        mats = {b: Matrix.Translation(pos[b]) @ rot[b].to_4x4() for b in ORDER}
        for b in SHIELD_ARM:
            mats[b] = mats["Chest"] @ rest.tgt.shield_rel[b]
        out.append(mats)
    return out, wrists


def extra_quality(tgt, solved, wrists):
    """Jitter (mean angular acceleration of the bones) and wrist bend range."""
    acc = []
    for a, b, c in zip(solved, solved[1:], solved[2:], strict=False):
        for bone in ORDER:
            qa, qb, qc = (m[bone].to_quaternion() for m in (a, b, c))
            d1 = qa.rotation_difference(qb).angle
            d2 = qb.rotation_difference(qc).angle
            acc.append(abs(d2 - d1))
    bend = [math.degrees(Quaternion().rotation_difference(w).angle) for w in wrists]
    return {
        "jitter_deg_per_frame2": round(
            math.degrees(float(np.mean(acc))) if acc else 0.0, 2
        ),
        "wrist_r_bend_max_deg": round(max(bend), 1),
    }


def clip_bases(tgt, rest, spec):
    """Basis dictionaries of a clip (24 fps), the solved matrices and the clip data."""
    clip = Clip(tgt, spec)
    solved, wrists = solve_clip(rest, clip)
    bases = [nt12.to_basis(tgt, s) for s in solved]
    if spec[5] and len(bases) > LOOP_BLEND + 1:
        n = len(bases)
        for j in range(LOOP_BLEND):
            k = n - LOOP_BLEND + j
            bases[k] = nt12.blend_basis(bases[k], bases[0], (j + 1) / (LOOP_BLEND + 1))
    return bases, solved, wrists, clip


def setup():
    """Fine human armature, NT12 target data and the NT13 rest frames."""
    arm, _meshes = bf.load_fine_human()
    rig = bs.Rig("human")
    for b in bs.HUMAN_BONES:
        rig.add(arm, b, b)
    bs.add_human_virtuals(rig, arm)
    rig.capture_rest()
    tgt = nt12.Target(arm)
    return arm, rig, tgt, Rest(tgt, arm)


def bake():
    """Bake the substituted clips into ``OUT_DIR`` and write its manifest."""
    import battle_skinned_poses as poses

    arm, rig, tgt, rest = setup()
    report = {}
    for spec in CLIPS:
        name, video, first, last, speed, loop, yaw, note = spec
        t0 = time.time()
        bases, solved, wrists, clip = clip_bases(tgt, rest, spec)
        start = rig.begin_clip()
        for basis_ in bases:
            for pb in arm.pose.bones:
                pb.matrix_basis = basis_.get(pb.name, Matrix.Identity(4))
            poses.reset_state()
            bpy.context.view_layer.update()
            rig.add_frame(rig.frame_matrices())
        rig.end_clip(name, start, loop)
        q = nt12.quality(tgt, solved)
        q.update(extra_quality(tgt, solved, wrists))
        q["contact_frames"] = [int(c) for c in clip.contacts.sum(axis=0)]
        report[name] = {
            "source": f"player video {video} frames {first}-{last} @30fps, x{speed} speed",
            "note": note,
            "yaw_deg": yaw,
            "quality": q,
            "solve_s": round(time.time() - t0, 1),
        }
        print("QUALITY", name, json.dumps(q), f"{time.time() - t0:.1f}s")
    os.makedirs(OUT_DIR, exist_ok=True)
    bs.OUT_DIR = OUT_DIR
    rig.write()
    manifest = rig.manifest()
    manifest["rig"] = "human"
    manifest["clip_sources"] = report
    manifest["source"] = (
        "tools/blender_scripts/nt13_video_trial.py - the player's own phone videos, "
        "MediaPipe Pose Landmarker heavy (Apache 2.0), see docs/wip/nt13-video-mocap.md"
    )
    with open(os.path.join(OUT_DIR, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
    print("OK", OUT_DIR)


RENDER_FRACS = (0.0, 0.25, 0.5, 0.75, 1.0)
RENDER_EYE = ((2.4, -4.2, 1.5), (0.0, -0.2, 0.85))


def render(out):
    """Workbench renders of each clip: keyframed ``k``, CMU ``m`` (NT12 clips), video ``v``.

    Files ``<clip>_<k|m|v>_<i>.png`` in `out` at ``RENDER_FRACS`` of the clip, plus
    ``frames.json`` (video and source frame of each render, for the contact sheet).
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
    tgt = nt12.Target(arm)
    rest = Rest(tgt, arm)
    cmu = {c[0]: c for c in nt12.CLIPS}
    index = {}

    def show(basis_):
        for pb in arm.pose.bones:
            pb.matrix_basis = basis_.get(pb.name, Matrix.Identity(4))
        poses.reset_state()
        bpy.context.view_layer.update()
        fp.follow_prop(objs)

    for spec in CLIPS:
        name, video, first, last, speed, _loop, _yaw, _note = spec
        bases, _solved, _w, _clip = clip_bases(tgt, rest, spec)
        cmu_bases = nt12.clip_bases(tgt, cmu[name])[0] if name in cmu else None
        for i, frac in enumerate(RENDER_FRACS):
            fp.pose_clip(arm, name, frac)
            fp.follow_prop(objs)
            fp.render(os.path.join(out, f"{name}_k_{i}.png"))
            if cmu_bases:
                show(cmu_bases[int(round(frac * (len(cmu_bases) - 1)))])
                fp.render(os.path.join(out, f"{name}_m_{i}.png"))
            show(bases[int(round(frac * (len(bases) - 1)))])
            fp.render(os.path.join(out, f"{name}_v_{i}.png"))
            index[f"{name}_{i}"] = [video, int(round(first + frac * (last - first)))]
    with open(os.path.join(out, "frames.json"), "w") as f:
        json.dump(index, f, indent=1)


def main():
    """``bake`` (default) or ``render DIR`` after ``--``."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if args and args[0] == "render":
        render(args[1] if len(args) > 1 else "/tmp/nt13_render")
    else:
        bake()


if __name__ == "__main__":
    main()
