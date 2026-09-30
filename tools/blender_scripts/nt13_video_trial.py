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


def clip(name, video, first, last, speed, loop, yaw, note, **opts):
    """One substituted clip.

    `first`, `last`: video frames (at the video's own rate); `speed`: speed-up; `yaw`: direction
    of the video (degrees about up, 0 = towards the camera, 90 = the image's right) that becomes
    the figure's forward, or ``"auto"`` (``video_mocap_clean.facing_yaw``, over `yaw_range`
    video frames when given). Options (NT14): ``grip`` ``"two_hands"`` (NT13: the stick's line
    through both wrists) or ``"right_hand"`` (from the right hand's landmarks); ``shield``
    ``"keyframed"`` (NT12/NT13: keyframed guard arm on the video chest) or ``"disc"`` (left arm
    solved from the tracked red disc, ``tools/video_mocap/track_disc.py``); ``reach``
    (``"disc"``: the facing follows the shield, for a parry); ``set`` (``"nt13"``/``"nt14"``).
    """
    spec = {
        "name": name,
        "video": video,
        "first": first,
        "last": last,
        "speed": speed,
        "loop": loop,
        "yaw": yaw,
        "note": note,
        "grip": "two_hands",
        "shield": "keyframed",
        "reach": None,
        "yaw_range": None,
        "set": "nt13",
    }
    spec.update(opts)
    return spec


# NT13: first shoot (4K portrait 30 fps, stick in both hands, no shield).
CLIPS_NT13 = [
    clip(
        "guard",
        "IMG_6455",
        0,
        48,
        1.0,
        True,
        45.0,
        "stick on the right shoulder, still",
    ),
    clip(
        "overhead", "IMG_6455", 54, 135, 1.8, False, 45.0, "raised then cut down across"
    ),
    clip("slash", "IMG_6458", 12, 72, 1.4, False, 45.0, "horizontal cut and back"),
    clip("thrust", "IMG_6457", 57, 126, 1.4, False, 70.0, "two-handed lunge and back"),
]
# NT14: second shoot (1080p portrait 60 fps, stick in the right hand, red disc as a shield).
NT14 = {"grip": "right_hand", "shield": "disc", "set": "nt14"}
CLIPS_NT14 = [
    clip(
        "guard",
        "IMG_6461",
        12,
        72,
        1.0,
        True,
        "auto",
        "sword low, disc before the chest",
        yaw_range=(72, 165),
        **NT14,
    ),
    clip(
        "overhead",
        "IMG_6459",
        100,
        280,
        1.3,
        False,
        "auto",
        "raised, cut down, follow-through",
        **NT14,
    ),
    clip(
        "parry",
        "IMG_6460",
        62,
        175,
        1.2,
        False,
        "auto",
        "disc thrown up high, crouched",
        reach="disc",
        **NT14,
    ),
    clip(
        "thrust",
        "IMG_6461",
        72,
        165,
        1.2,
        False,
        "auto",
        "lunge and one-handed thrust",
        **NT14,
    ),
]
# Baked into ``video_trial/``: per gesture the better of NT13 and NT14 (docs/wip/nt14-video-set2.md).
CLIPS = [CLIPS_NT13[2]] + CLIPS_NT14

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
        # NT14: the shield's face at rest, from the keyframed guard (forearm across the belly,
        # shield facing forward): face = forward rotated back by the guard's forearm turn.
        guard = (tgt.rest["Chest"] @ tgt.shield_rel["LowerArm.L"]).to_3x3().normalized()
        turn = guard @ tgt.rest["LowerArm.L"].to_3x3().normalized().inverted()
        self.shield_face = (turn.inverted() @ self.fwd).normalized()
        self.arm_l = (
            (h["LowerArm.L"] - h["UpperArm.L"]).length,
            (h["Wrist.L"] - h["LowerArm.L"]).length,
        )

    @staticmethod
    def _lu(left, up):
        """Frame (left, forward, up) from a left and an up vector."""
        return nt12.frame_of(left.copy(), up.copy())


def lu(left, up):
    """Frame (left, forward, up) from a left and an up vector."""
    return nt12.frame_of(left.copy(), up.copy())


def disc_track(data, video, first, last, rate):
    """NT14: centre and front-face normal of the shield disc (hip-centred Z-up, source frames).

    ``<video>_disc.npz`` (``track_disc.py``) gives the disc in the image; the image is mapped
    onto MediaPipe's world frame by the trusted landmarks (``image_world_fit``), the depth
    comes from the disc's apparent size, anchored on the median depth of MediaPipe's (hidden,
    guessed) left wrist.
    """
    disc = np.load(os.path.join(POSE_DIR, f"{video}_disc.npz"))
    sl = slice(first, last + 1)
    world = data["world"][sl]
    w, h = (float(x) for x in data["size"])
    aspect = w / h
    fit = vc.image_world_fit(world, data["image"][sl], aspect, data["visibility"][sl])
    fit = tuple(vc.one_euro(f, rate, 1.0, 0.2) for f in fit)
    z = vc.to_zup(world)
    anchor = float(np.clip(np.median(z[:, vc.WRIST_L, 1]), -0.5, -0.15))
    axes = disc["axes"][sl]
    pts = vc.disc_points(disc["centre"][sl], axes[:, 0], fit, aspect, anchor)
    pts = vc.one_euro(pts, rate, 1.5, 0.3)
    chest = vc.one_euro(
        0.5 * (z[:, vc.SHOULDER_L] + z[:, vc.SHOULDER_R]), rate, 1.5, 0.3
    )
    normals = vc.disc_normals(axes[:, 0], axes[:, 1], disc["angle"][sl], pts, chest)
    return pts, vc.smooth_directions(normals, rate, 1.0, 0.3)


class Clip:
    """One cleaned video clip mapped to armature space."""

    def __init__(self, tgt, spec):
        """Clean the landmarks of `spec` and map them onto the target's frame and size."""
        video, first, last = spec["video"], spec["first"], spec["last"]
        speed, loop, yaw = spec["speed"], spec["loop"], spec["yaw"]
        t0 = time.time()
        data = np.load(os.path.join(POSE_DIR, f"{video}.npz"))
        rate = float(data["fps"])
        world = data["world"][first : last + 1]
        image = data["image"][first : last + 1]
        extra, normals = {}, None
        if spec["shield"] == "disc":
            extra["disc"], normals = disc_track(data, video, first, last, rate * speed)
        cleaned = vc.clean(world, image, rate * speed, float(bs.FPS), extra=extra)
        pts = cleaned["points"]
        disc = cleaned["extra"].get("disc")
        self.contacts = cleaned["contacts"]
        self.lengths = cleaned["lengths"]
        if loop:  # no drift over a loop: the root ends where it started
            drift = pts[-1].mean(axis=0) - pts[0].mean(axis=0)
            drift[2] = 0.0
            k = np.linspace(0.0, 1.0, len(pts))[:, None, None]
            pts = pts - drift[None, None, :] * k
            if disc is not None:
                disc = disc - drift[None, :] * k[:, 0]
        if yaw == "auto":  # NT14: towards the opponent (the strike, the chest's facing)
            if spec["yaw_range"]:
                a, b = spec["yaw_range"]
                other = vc.clean(
                    data["world"][a : b + 1], data["image"][a : b + 1], rate
                )
                yaw = vc.facing_yaw(other["points"])
            elif spec["reach"] == "disc" and disc is not None:
                yaw = vc.facing_yaw(pts, disc)
            else:
                yaw = vc.facing_yaw(pts)
        self.yaw = round(float(yaw), 1)
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
        self.disc = self.normals = None
        if disc is not None:
            self.disc = [ground + rot @ (vec(p - r0) * scale) for p in disc]
            levelled = normals @ cleaned["level"].T
            n24 = vc.resample(levelled, rate * speed, float(bs.FPS))[: len(pts)]
            self.normals = [(rot @ vec(n)).normalized() for n in n24]
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


def hand_sides(clip, blades):
    """NT14 (one-handed grip): roll reference of the fist, the hand's axis off the blade.

    The hand's axis (wrist to the middle of the index and pinky landmarks, smoothed) is never
    along a sword held in the fist, unlike the forearm (``blade_sides``), so no frame is
    ill-defined.
    """
    axes = np.array(
        [
            list((j[vc.INDEX_R] + j[vc.PINKY_R]) * 0.5 - j[vc.WRIST_R])
            for j in clip.frames
        ]
    )
    axes = vc.smooth_directions(axes, float(bs.FPS), 1.2, 0.3)
    out = []
    for a, blade in zip(axes, blades, strict=True):
        s = vec(a) - blade * vec(a).dot(blade)
        out.append(s.normalized() if s.length > 1e-6 else blade.orthogonal())
    return out


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


SHIELD_STANDOFF_M = 0.05  # fist behind the disc's rim plane
POLE_STEPS = 72


def shield_arm_rotations(rest, clip, index, j, state):
    """NT14: left arm deltas (upper, lower) putting the fist behind the tracked disc.

    Two-bone reach from the shoulder to the disc (distance scaled from the performer's arm to
    the figure's, capped to the reach); the elbow's turn about the shoulder-fist line is picked
    so that the forearm lies in the disc's plane (a strapped shield lies along the forearm),
    the elbow low and outwards, close to the previous frame's; the forearm's roll then turns
    the shield's face (``Rest.shield_face``) onto the disc's normal.
    """
    l1, l2 = rest.arm_l
    shoulder = j[vc.SHOULDER_L]
    normal = clip.normals[index]
    target = clip.disc[index] - normal * (SHIELD_STANDOFF_M * clip.scale)
    perf = (
        clip.lengths[(vc.SHOULDER_L, vc.ELBOW_L)]
        + clip.lengths[(vc.ELBOW_L, vc.WRIST_L)]
    ) * clip.scale
    reach = (target - shoulder) * ((l1 + l2) / max(perf, 1e-6))
    dist = min(max(reach.length, abs(l1 - l2) + 1e-4), (l1 + l2) * 0.985)
    axis = reach.normalized()
    along = (l1 * l1 - l2 * l2 + dist * dist) / (2.0 * dist)
    height = math.sqrt(max(l1 * l1 - along * along, 0.0))
    p1 = (-rest.up) - axis * (-rest.up).dot(axis)
    if p1.length < 1e-6:
        p1 = axis.orthogonal()
    p1.normalize()
    p2 = axis.cross(p1)
    prev = state.get("shield_pole")
    best, best_score = None, math.inf
    for k in range(POLE_STEPS):
        th = 2.0 * math.pi * k / POLE_STEPS
        dirn = p1 * math.cos(th) + p2 * math.sin(th)
        elbow = axis * along + dirn * height
        fore = (axis * dist - elbow).normalized()
        score = 2.0 * abs(fore.dot(normal))
        score += 0.6 * max(0.0, dirn.dot(rest.up)) - 0.3 * dirn.dot(rest.left)
        if prev is not None:
            score += 0.8 * (1.0 - dirn.dot(prev))
        if score < best_score:
            best, best_score = dirn, score
    state["shield_pole"] = best
    elbow = axis * along + best * height
    u = elbow
    lo = axis * dist - elbow
    u0, l0, h0 = rest.limb["arm.L"]
    hinge = u.cross(lo)
    if hinge.length < 1e-6:
        hinge = state.get("arm.L") or h0
    hinge.normalize()
    state["arm.L"] = hinge
    r_upper = basis(u, hinge) @ basis(u0, h0).transposed()
    r_lower = basis(lo, normal) @ basis(l0, rest.shield_face).transposed()
    return r_upper, r_lower


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
    if clip.disc is not None:
        r["UpperArm.L"], r["LowerArm.L"] = shield_arm_rotations(
            rest, clip, index, j, state
        )
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
        targets, weights = vc.step_targets(
            ankle, clip.contacts[:n, i], ground=ground, unit=clip.scale
        )
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
    one_hand = clip.spec["grip"] == "right_hand"
    blades = blade_directions(clip, math.inf if one_hand else 0.12 * clip.scale)
    if one_hand:  # NT14: the fist's landmarks are noisy frame to frame
        smooth = vc.smooth_directions(
            np.array([list(b) for b in blades]), float(bs.FPS), 1.2, 0.3
        )
        blades = [vec(b) for b in smooth]
        sides = hand_sides(clip, blades)
    else:
        sides = blade_sides(clip, blades)
    for i in range(len(clip.frames)):
        rot, pos, wrist = solve(rest, clip, i, state, blades[i], sides[i])
        rots.append(rot)
        poss.append(pos)
        wrists.append(wrist)
    wrists = filter_wrist(rest, rots, wrists)
    pin_feet(rest, clip, rots, poss)
    out = []
    for rot, pos in zip(rots, poss, strict=True):
        mats = {b: Matrix.Translation(pos[b]) @ rot[b].to_4x4() for b in ORDER}
        if clip.disc is None:
            for b in SHIELD_ARM:
                mats[b] = mats["Chest"] @ rest.tgt.shield_rel[b]
        out.append(mats)
    return out, wrists


WRIST_SPIKE_DEG = 12.0


def filter_wrist(rest, rots, wrists):
    """NT14: right wrist spikes removed and its turn smoothed (in place on `rots`).

    The wrist's turn relative to the forearm (``solve``'s capped deviation) loses its one-frame
    spikes (``despike_quats``) and is smoothed (zero-phase One-Euro); the wrist bone is rebuilt
    on its forearm. Returns the filtered deviations.
    """
    q = np.array([[w.w, w.x, w.y, w.z] for w in wrists])
    q = vc.smooth_quats(vc.despike_quats(q, WRIST_SPIKE_DEG), float(bs.FPS), 3.0, 0.3)
    tgt = rest.tgt
    lower_rest = tgt.rest["LowerArm.R"].to_3x3().inverted()
    wrist_rest = tgt.rest["Wrist.R"].to_3x3()
    out = []
    for t, row in enumerate(q):
        dev = Quaternion([float(x) for x in row])
        r_lower = rots[t]["LowerArm.R"] @ lower_rest
        rots[t]["Wrist.R"] = r_lower @ dev.to_matrix() @ wrist_rest
        out.append(dev)
    return out


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
        "tremor_deg": tremor(solved),
        "wrist_r_bend_max_deg": round(max(bend), 1),
    }


def tremor(solved):
    """NT14: high-frequency shake (degrees) of a solved clip.

    Mean angle between each bone's rotation and its 5-frame binomial average (1 4 6 4 1).
    Unlike the jitter (angular acceleration), a fast but smooth strike scores low: only
    frame-to-frame wobble remains after such a short average.
    """
    if len(solved) < 5:
        return 0.0
    res = []
    k = np.array([1.0, 4.0, 6.0, 4.0, 1.0]) / 16.0
    for bone in ORDER:
        q = vc.continuous_quats([list(m[bone].to_quaternion()) for m in solved])
        avg = sum(k[i] * q[i : len(q) - 4 + i] for i in range(5))
        avg /= np.linalg.norm(avg, axis=-1, keepdims=True)
        res.append(vc.quat_angle(q[2:-2], avg))
    return round(math.degrees(float(np.mean(res))), 2)


def clip_bases(tgt, rest, spec):
    """Basis dictionaries of a clip (24 fps), the solved matrices and the clip data."""
    clip = Clip(tgt, spec)
    solved, wrists = solve_clip(rest, clip)
    bases = [nt12.to_basis(tgt, s) for s in solved]
    if spec["loop"] and len(bases) > LOOP_BLEND + 1:
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


def measure(tgt, solved, wrists=None):
    """Quality measures of a solved clip (NT12's plus jitter and wrist bend)."""
    q = nt12.quality(tgt, solved)
    q.update(extra_quality(tgt, solved, wrists or [Quaternion()]))
    if wrists is None:
        q.pop("wrist_r_bend_max_deg", None)
    return q


def video_entry(tgt, rest, spec):
    """Bases, quality and manifest record of a video clip."""
    t0 = time.time()
    bases, solved, wrists, clip = clip_bases(tgt, rest, spec)
    q = measure(tgt, solved, wrists)
    q["contact_frames"] = [int(c) for c in clip.contacts.sum(axis=0)]
    fps = round(clip.rate)
    record = {
        "source": (
            f"player video {spec['video']} frames {spec['first']}-{spec['last']} "
            f"@{fps}fps, x{spec['speed']} speed"
        ),
        "set": spec["set"],
        "note": spec["note"],
        "grip": spec["grip"],
        "shield": spec["shield"],
        "yaw_deg": clip.yaw,
        "quality": q,
        "solve_s": round(time.time() - t0, 1),
    }
    return bases, spec["loop"], record


def cmu_entry(tgt, spec):
    """Bases, quality and manifest record of an NT12 CMU clip."""
    t0 = time.time()
    bases, solved = nt12.clip_bases(tgt, spec)
    record = {
        "source": f"CMU mocap subject {spec[1]} take {spec[2]} frames {spec[3]}-{spec[4]} @120fps",
        "set": "nt12",
        "note": spec[6],
        "quality": measure(tgt, solved),
        "solve_s": round(time.time() - t0, 1),
    }
    return bases, spec[5], record


def bake_entries(entries, out_dir, source):
    """Bake `entries` ((clip, kind ``"video"``/``"cmu"``, spec)) into `out_dir` + manifest."""
    import battle_skinned_poses as poses

    arm, rig, tgt, rest = setup()
    report = {}
    for name, kind, spec in entries:
        if kind == "cmu":
            bases, loop, record = cmu_entry(tgt, spec)
        else:
            bases, loop, record = video_entry(tgt, rest, spec)
        start = rig.begin_clip()
        for basis_ in bases:
            for pb in arm.pose.bones:
                pb.matrix_basis = basis_.get(pb.name, Matrix.Identity(4))
            poses.reset_state()
            bpy.context.view_layer.update()
            rig.add_frame(rig.frame_matrices())
        rig.end_clip(name, start, loop)
        report[name] = record
        print("QUALITY", name, record["set"], json.dumps(record["quality"]))
    os.makedirs(out_dir, exist_ok=True)
    bs.OUT_DIR = out_dir
    rig.write()
    manifest = rig.manifest()
    manifest["rig"] = "human"
    manifest["clip_sources"] = report
    manifest["source"] = source
    with open(os.path.join(out_dir, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
    print("OK", out_dir)


def bake():
    """Bake the substituted clips of the ``--video-trial`` option into ``OUT_DIR``."""
    bake_entries(
        [(s["name"], "video", s) for s in CLIPS],
        OUT_DIR,
        "tools/blender_scripts/nt13_video_trial.py - the player's own phone videos, "
        "MediaPipe Pose Landmarker heavy (Apache 2.0), see docs/wip/nt13-video-mocap.md "
        "and docs/wip/nt14-video-set2.md",
    )


# NT14 (ADR 0129, complement): default melee clips of the fine figures, per gesture the best
# source (keyframed clips are simply not substituted). `--keyframed-melee` restores them all.
MELEE_DIR = os.path.join(bf.FINE_DIR, "melee")
MELEE_DEFAULT = {
    "guard": ("video", CLIPS_NT14[0]),
    "overhead": ("video", CLIPS_NT14[1]),
    "parry": ("video", CLIPS_NT14[2]),
    "thrust": ("video", CLIPS_NT14[3]),
}


def bake_melee():
    """Bake the default melee clips (``MELEE_DEFAULT``) into ``MELEE_DIR``."""
    bake_entries(
        [(name, kind, spec) for name, (kind, spec) in MELEE_DEFAULT.items()],
        MELEE_DIR,
        "tools/blender_scripts/nt13_video_trial.py (bake-melee) - default melee clips chosen "
        "per gesture among keyframed / CMU (NT12) / player videos (NT13, NT14), see "
        "docs/wip/nt14-video-set2.md and ADR 0129",
    )


def keyframed_solved(arm, name):
    """Armature-space matrices of every frame of a keyframed ``human`` clip."""
    import battle_fine_proto as fp

    specs = {c[0]: c for c in bs.human_clip_specs()}
    _n, source, _loop, overrides, _mirror = specs[name]
    act = bs.find_action(source, "CharacterArmature")
    first, last = (int(round(v)) for v in act.frame_range)
    count = getattr(overrides, "frames", None) or (last - first + 1)
    out = []
    for i in range(count):
        fp.pose_clip(arm, name, i / max(count - 1, 1))
        out.append({b: arm.pose.bones[b].matrix.copy() for b in ORDER})
    bs.rest_pose(arm)
    return out


def measure_all(path):
    """Quality of every candidate of every melee gesture, to `path` (JSON) and stdout."""
    arm, _rig, tgt, rest = setup()
    import battle_fine_proto as fp

    fp.prime_virtuals(arm)
    res = {}
    gestures = ("guard", "slash", "overhead", "thrust", "parry", "hit", "death")
    for name in gestures:
        row = {}
        try:
            row["keyframed"] = measure(tgt, keyframed_solved(arm, name))
        except Exception as exc:  # noqa: BLE001 - report and go on
            row["keyframed"] = {"error": str(exc)}
        for spec in nt12.CLIPS:
            if spec[0] == name:
                row["nt12"] = cmu_entry(tgt, spec)[2]["quality"]
        for spec in CLIPS_NT13 + CLIPS_NT14:
            if spec["name"] == name:
                _b, _l, record = video_entry(tgt, rest, spec)
                row[spec["set"]] = dict(record["quality"], yaw_deg=record["yaw_deg"])
        res[name] = row
        for src, q in row.items():
            print("MEASURE", name, src, json.dumps(q))
    with open(path, "w") as f:
        json.dump(res, f, indent=1, sort_keys=True)
    print("OK", path)


RENDER_FRACS = (0.0, 0.25, 0.5, 0.75, 1.0)
RENDER_EYE = ((1.7, -3.0, 1.35), (0.0, -0.2, 0.85))


def render(out, specs=None):
    """Workbench renders of each clip: keyframed ``k``, CMU ``m`` (NT12), NT13 ``v``, NT14 ``w``.

    ``c`` shows the newest video clip from the source camera's side, to compare with the video.
    `specs` defaults to the gestures of ``CLIPS_NT14`` (plus NT13's own), each rendered from
    every source that has it.

    Files ``<clip>_<k|m|v|w|c>_<i>.png`` in `out` at ``RENDER_FRACS`` of the clip, plus
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
    nt13 = {c["name"]: c for c in CLIPS_NT13}
    nt14 = {c["name"]: c for c in CLIPS_NT14}
    names = specs or list(dict.fromkeys(list(nt14) + list(nt13)))
    index = {}

    def show(basis_):
        for pb in arm.pose.bones:
            pb.matrix_basis = basis_.get(pb.name, Matrix.Identity(4))
        poses.reset_state()
        bpy.context.view_layer.update()
        fp.follow_prop(objs)

    for name in names:
        newest = nt14.get(name) or nt13[name]
        video_bases = {}
        clip = None
        for tag, table in (("v", nt13), ("w", nt14)):
            if name in table:
                bases, _solved, _w, c = clip_bases(tgt, rest, table[name])
                video_bases[tag] = bases
                if table[name] is newest:
                    clip = c
        # Camera of the source video (in front of the performer), for the ``c`` renders.
        look = arm.matrix_world.to_3x3().normalized() @ (clip.rot @ Vector((0, -1, 0)))
        look.z = 0.0
        eye_c = Vector(RENDER_EYE[1]) + look.normalized() * 3.4 + Vector((0, 0, 0.45))
        cmu_bases = nt12.clip_bases(tgt, cmu[name])[0] if name in cmu else None
        newest_tag = "w" if name in nt14 else "v"
        for i, frac in enumerate(RENDER_FRACS):
            fp.pose_clip(arm, name, frac)
            fp.follow_prop(objs)
            fp.render(os.path.join(out, f"{name}_k_{i}.png"))
            if cmu_bases:
                show(cmu_bases[int(round(frac * (len(cmu_bases) - 1)))])
                fp.render(os.path.join(out, f"{name}_m_{i}.png"))
            for tag, bases in video_bases.items():
                show(bases[int(round(frac * (len(bases) - 1)))])
                fp.render(os.path.join(out, f"{name}_{tag}_{i}.png"))
            bases = video_bases[newest_tag]
            show(bases[int(round(frac * (len(bases) - 1)))])
            fp.look_at(cam, eye_c, RENDER_EYE[1], 40)
            fp.render(os.path.join(out, f"{name}_c_{i}.png"))
            fp.look_at(cam, RENDER_EYE[0], RENDER_EYE[1], 40)
            first, last = newest["first"], newest["last"]
            index[f"{name}_{i}"] = [
                newest["video"],
                int(round(first + frac * (last - first))),
            ]
    with open(os.path.join(out, "frames.json"), "w") as f:
        json.dump(index, f, indent=1)


def main():
    """``bake`` (default), ``bake-melee``, ``measure FILE`` or ``render DIR [clips]`` after ``--``."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if args and args[0] == "render":
        render(args[1] if len(args) > 1 else "/tmp/nt14_render", args[2:] or None)
    elif args and args[0] == "measure":
        measure_all(args[1] if len(args) > 1 else "/tmp/nt14_measures.json")
    elif args and args[0] == "bake-melee":
        bake_melee()
    else:
        bake()


if __name__ == "__main__":
    main()
