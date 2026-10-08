"""Horse gaits of the mounted figures (lot AS3): trot, turns, riderless horse.

The Quaternius horse has a walk (4-beat) and a gallop but no trot, no turn and no way to run
free once its rider has fallen. The missing clips are keyframed here on top of the walk
action, after the gait plates of Eadweard Muybridge, "The Horse in Motion" (1878, public
domain; nothing is downloaded, the phases are written down in this file).

Trot (Muybridge, plates "Abe Edgington, trotting"): the legs move in diagonal pairs, near
fore with off hind and off fore with near hind. The pairs land half a stride apart (two beats
per stride). Each pair is on the ground for about 40 % of the stride, so the horse is in the
air for about 10 % of the stride after each pair lifts (two moments of suspension, with the
body at its highest just before the next pair lands, at its lowest in mid-stance). The legs
swing wider and the knees fold higher than at the walk; the head nods twice per stride.

How it is built: each leg group (shoulder, leg chain, hoof controller) is sampled from the
`Walk` action at its own frame, shifted so that its touchdown (forward-most reach of the lower
leg, measured on the action itself) falls on the beat of its diagonal; the spine, neck and
tail come from the walk at the stride phase. A wider swing, a higher knee lift and the
vertical bounce of the body are then added, and the hooves follow their lower legs.

Turns bank the body into the turn about the ground line under the hips and bend the neck and
head towards the inside (left turn = counter-clockwise seen from above). The riderless horse
(`c_fall`) startles on the `Idle_HitReact_Right` action, then bolts at the `Gallop`; the shader
(`battle_soldier_skinned.gdshader`, code 6) carries it away from the fallen rider.

World space is Blender's: Z up, the horse faces -Y, its left side is at +X.
"""

import json
import math
import os

import battle_skinned as bs
import battle_skinned_poses as poses
import bpy
from mathutils import Vector

Y_AXIS = Vector((0, 1, 0))

WALK = "Walk"
GALLOP = "Gallop"
STARTLE = "Idle_HitReact_Right"
ANIMAL = "AnimalArmature"

# Tunables of the baked clips (the engine's tunables live in data/fx/battle_animation.json).
TROT_FRAMES = 18  # one stride at 24 fps = 0.75 s
TROT_SWING = 0.16  # rad, extra swing of the leg root (forward at touchdown)
TROT_LIFT = 0.45  # rad, extra knee fold during the forward swing
TROT_BOUNCE = 0.035  # m, rise of the body in suspension
TROT_STANCE = 0.2  # stride phase of mid-stance (body lowest); the pair lands at 0
TROT_NOD = 0.05  # rad, head nod (twice per stride)
TROT_TAIL = 0.16  # rad, tail carried higher
WALK_LEAN = 0.10  # rad, roll into the turn at the walk
TROT_LEAN = 0.15  # rad, roll into the turn at the trot
NECK_BEND = 0.30  # rad, yaw of the first neck bone towards the turn
HEAD_BEND = 0.14  # rad, extra yaw of the head
SPINE_BEND = 0.07  # rad, yaw of the upper spine
FALL_FRAMES = 108  # 4.5 s: the shader lets the free horse go at t = 4.5 s
FALL_SOURCE_FRAMES = 30  # the rider's fall lasts as before
STARTLE_FRAMES = 17
STARTLE_HOLD = 8 / 24.0  # s of pure startle before the bolt takes over
STARTLE_FADE = 6 / 24.0

# Diagonal pairs: near fore + off hind land together, off fore + near hind half a stride later.
LEG_BEAT = {"FL": 0.0, "BR": 0.0, "FR": 0.5, "BL": 0.5}
LEG_GROUP = {
    "FL": (
        "FrontShoulder.L",
        "FrontUpperLeg.L",
        "FrontLowerLeg.L",
        "IKFrontLeg.L",
        "FF.L",
        "PoleTarget.L",
    ),
    "FR": (
        "FrontShoulder.R",
        "FrontUpperLeg.R",
        "FrontLowerLeg.R",
        "IKFrontLeg.R",
        "FF.R",
        "PoleTarget.R",
    ),
    "BL": (
        "BackShoulder.L",
        "BackLeg.L",
        "BackUpperLeg.L",
        "BackLowerLeg.L",
        "IKBackLeg.L",
        "FFB.L",
        "PoleTargetBack.L",
    ),
    "BR": (
        "BackShoulder.R",
        "BackLeg.R",
        "BackUpperLeg.R",
        "BackLowerLeg.R",
        "IKBackLeg.R",
        "FFB.R",
        "PoleTargetBack.R",
    ),
}

_CACHE = {}


# --- Sampling the source actions ----------------------------------------------------------


def _frames(action):
    """(first frame, period) of `action`: the last frame repeats the first."""
    first, last = (int(round(v)) for v in bs.find_action(action, ANIMAL).frame_range)
    return first, max(last - first, 1)


def _sample(harm, action, frame):
    """Local pose (`matrix_basis` per bone) of `action` at the integer `frame`."""
    key = (harm.as_pointer(), action, frame)
    if key not in _CACHE:
        bs.set_action(harm, bs.find_action(action, ANIMAL))
        for pb in harm.pose.bones:
            pb.matrix_basis.identity()
        bpy.context.scene.frame_set(frame)
        _CACHE[key] = {pb.name: pb.matrix_basis.copy() for pb in harm.pose.bones}
    return _CACHE[key]


def _blend(a, b, w):
    """Pose `a` -> `b` by `w` (matrices interpolated by translation, rotation and scale)."""
    if w <= 1e-4:
        return a
    return {k: a[k].lerp(b[k], w) for k in a}


def _at(harm, action, frame):
    """Pose of `action` at a fractional, looping `frame` (relative to its first frame)."""
    first, period = _frames(action)
    x = frame % period
    f0 = int(math.floor(x))
    return _blend(
        _sample(harm, action, first + f0),
        _sample(harm, action, first + (f0 + 1) % period),
        x - f0,
    )


def touchdowns(harm):
    """Frame of the Walk at which each leg reaches forward most (touchdown), measured."""
    key = (harm.as_pointer(), "touchdowns")
    if key not in _CACHE:
        bs.set_action(harm, bs.find_action(WALK, ANIMAL))
        first, period = _frames(WALK)
        best = {}
        for f in range(period):
            for pb in harm.pose.bones:
                pb.matrix_basis.identity()
            bpy.context.scene.frame_set(first + f)
            for leg, bones in LEG_GROUP.items():
                lower = next(b for b in bones if "LowerLeg" in b)
                y = poses.pos(harm, lower).y  # forward = -Y
                if leg not in best or y < best[leg][0]:
                    best[leg] = (y, f)
        _CACHE[key] = {leg: v[1] for leg, v in best.items()}
    return _CACHE[key]


class _Entry:
    """The baker's action and frame, restored once the sampling is done."""

    def __init__(self, harm):
        self.harm = harm
        self.action = harm.animation_data.action if harm.animation_data else None
        self.frame = bpy.context.scene.frame_current

    def restore(self):
        if self.action is not None:
            bs.set_action(self.harm, self.action)
        bpy.context.scene.frame_set(self.frame)


def _commit(harm, pose):
    """Assign a sampled pose to the horse."""
    for name, m in pose.items():
        harm.pose.bones[name].matrix_basis = m
    bpy.context.view_layer.update()


# --- Trot -----------------------------------------------------------------------------------


def trot_phase(t, frames=TROT_FRAMES):
    """Stride phase (0-1, looping) of the frame at clip fraction `t`."""
    return (t * (frames - 1) / frames) % 1.0


def trot_pose(harm, phase):
    """Local pose of the trot at stride `phase`: diagonal legs, walk spine, neck and tail."""
    _first, period = _frames(WALK)
    pose = dict(_at(harm, WALK, phase * period))
    td = touchdowns(harm)
    for leg, bones in LEG_GROUP.items():
        legpose = _at(harm, WALK, td[leg] + (phase - LEG_BEAT[leg]) * period)
        for b in bones:
            pose[b] = legpose[b]
    return pose


def horse_trot(harm, t, frames=TROT_FRAMES):
    """Horse trotting: measured legs (lot AS8b), or the keyframed trot with AS8B_LEGACY=1."""
    if free_enabled():
        return horse_trot_free(harm, t, frames)
    return horse_trot_keyed(harm, t, frames)


def horse_trot_keyed(harm, t, frames=TROT_FRAMES):
    """Horse trotting: the legs by diagonal pairs, two suspensions, nodding head."""
    entry = _Entry(harm)
    phase = trot_phase(t, frames)
    pose = trot_pose(harm, phase)
    entry.restore()
    _commit(harm, pose)
    before = poses._horse_capture(harm)
    for leg, (chain, _hoof) in poses.HORSE_LEGS.items():
        local = (phase - LEG_BEAT[leg]) % 1.0
        reach = math.cos(2.0 * math.pi * local)  # 1 at touchdown, -1 at lift-off
        lift = max(0.0, -math.sin(2.0 * math.pi * local))  # forward swing in the air
        poses.rotate_about(harm, chain[0], poses.X_AXIS, -TROT_SWING * reach)
        poses.rotate_about(harm, chain[-1], poses.X_AXIS, TROT_LIFT * lift)
    rise = 0.5 * (1.0 - math.cos(4.0 * math.pi * (phase - TROT_STANCE)))
    poses.translate(harm, "Body", Vector((0.0, 0.0, TROT_BOUNCE * rise)))
    nod = TROT_NOD * math.sin(4.0 * math.pi * (phase - 0.1))
    poses.rotate_about(harm, "Neck1", poses.X_AXIS, nod)
    poses.rotate_about(harm, "Tail1", poses.X_AXIS, TROT_TAIL)
    poses._horse_feet(harm, before)


# --- Gaits measured on free films (lot AS8b) -------------------------------------------------
#
# The legs are driven by hoof trajectories measured on Muybridge's plates (public domain):
# `tools/video_mocap/track_quadruped.py` -> `data/horse_gaits_free.json` (see data/SOURCE.md).
# Each hoof is placed relative to the root of its leg (leg lengths forward `u`, below `d`) and
# the leg is bent to reach it (sagittal CCD); the body rises and pitches as measured. The
# spine, neck and tail still come from the Quaternius actions (Walk for the trot, Gallop).

FREE_PATH = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "data", "horse_gaits_free.json"
)
FREE_LEG = {
    "trot": {"FL": "F1", "BR": "H1", "FR": "F2", "BL": "H2"},
    "gallop": {"FL": "F1", "FR": "F2", "BL": "H1", "BR": "H2"},
}
# Share of the measured stride amplitude kept (the drawn trot is an extended racing trot).
FREE_REACH = {"trot": 1.0, "gallop": 1.0}
FREE_DUTY = {"trot": 0.36}  # planted share of the stride of every leg (diagonal pairs)
FREE_LIFT = {"trot": 0.5, "gallop": 0.8}  # share of the measured hoof height (drawn high knees)
FREE_CROUCH = {"trot": 0.14, "gallop": 0.14}  # m, body lowered so the legs can reach the stride
FREE_RISE = {"trot": 1.0, "gallop": 0.6}  # share of the measured rise of the body
FREE_PITCH = {"trot": 1.0, "gallop": 0.6}
GALLOP_FRAMES = 15  # the clip keeps its length (cadence data)
_FREE = {}


def free_enabled():
    """True unless AS8B_LEGACY=1 asks for the AS3 keyframed gaits (A/B comparison)."""
    return os.environ.get("AS8B_LEGACY") != "1"


def free_gaits():
    """Measured gait table (cached)."""
    if not _FREE:
        with open(FREE_PATH) as f:
            _FREE.update(json.load(f)["gaits"])
        for name, table in _FREE.items():
            table["name"] = name
    return _FREE


def _with_stance(table, gait_duty=None):
    """Hoof curves with a flat, uniform stance (cached on the table).

    The low-harmonic fit rounds the stance off. Where the fitted hoof height is under the
    planted tolerance the hoof is put on the ground (height 0) and its `u` is replaced by a
    straight line between the touchdown and lift-off values: the planted hoof then travels
    backwards at one constant speed.
    """
    if "eff" in table:
        return table["eff"]
    gait_duty = FREE_DUTY.get(table["name"])
    tol = table["planted_tol"]
    eff = {}
    for name, curve in table["legs"].items():
        n = len(curve["u"])
        u = list(curve["u"])
        leg_tol = tol
        if gait_duty:  # diagonal pairs share their stance: every leg gets the same duty
            leg_tol = sorted(curve["h"])[min(int(gait_duty * n), n - 1)]
        top = max(max(curve["h"]) - leg_tol, 1e-6)
        # Square root: the hoof leaves the ground briskly instead of skimming it.
        h = [top * math.sqrt(max(v - leg_tol, 0.0) / top) for v in curve["h"]]
        planted = [v <= 0.0 for v in h]
        if all(planted) or not any(planted):
            eff[name] = {"u": u, "h": h}
            continue
        start = next(i for i in range(n) if planted[i] and not planted[i - 1])
        i = start
        while i < start + n:
            if not planted[i % n]:
                i += 1
                continue
            end = i
            while planted[(end + 1) % n] and end + 1 < start + n:
                end += 1
            first, last = curve["u"][i % n], curve["u"][end % n]
            for k in range(end - i + 1):
                u[(i + k) % n] = first + (last - first) * k / max(end - i, 1)
            i = end + 1
        eff[name] = {"u": u, "h": h}
    table["eff"] = eff
    return eff


SHOULDER = {
    "FL": "FrontShoulder.L",
    "FR": "FrontShoulder.R",
    "BL": "BackShoulder.L",
    "BR": "BackShoulder.R",
}
JOINT_KICK = 1.0  # rad
SHOULDER_SHARE = 0.6  # share of the needed angle the shoulder / hip blade takes


def _plant_with_shoulder(harm, leg, before, target, iterations=24):
    """`poses._plant` with the shoulder (or hip) blade in the chain: a longer lever arm."""
    chain = (SHOULDER[leg], *poses.HORSE_LEGS[leg][0])
    # Start with the joint flexed the right way (foreleg: hoof back; hind leg: hoof forward) so
    # that the solver does not settle on a hyperextended leg.
    kick = JOINT_KICK if leg[0] == "F" else -JOINT_KICK
    poses.rotate_about(harm, chain[-1], poses.X_AXIS, kick)
    for _ in range(iterations):
        for bone in reversed(chain):
            end = poses._hoof_at(harm, leg, before).to_translation()
            pivot = poses.pos(harm, bone)
            a = end - pivot
            b = target - pivot
            angle = math.atan2(b.z, b.y) - math.atan2(a.z, a.y)
            angle = (angle + math.pi) % (2.0 * math.pi) - math.pi
            if bone == SHOULDER[leg]:
                angle *= SHOULDER_SHARE
            poses.rotate_about(harm, bone, poses.X_AXIS, max(-0.3, min(0.3, angle)), pivot)


def stride_gain(gait, rest):
    """Share of the measured stride the rig's legs can reach (the same for the four legs).

    The measured stride is scaled by one factor so that every planted hoof still travels at
    the same speed (the speed of the ground under the horse); the factor is the largest one
    at which the shortest leg, with the body lowered by the crouch, reaches the stride.
    """
    key = "gain_" + gait
    if key not in rest:
        table = free_gaits()[gait]
        gain = FREE_REACH[gait]
        for leg, name in FREE_LEG[gait].items():
            curve = _with_stance(table)[name]
            mean_u = sum(curve["u"]) / len(curve["u"])
            half = max(abs(v - mean_u) for v in curve["u"]) * rest["unit"]
            dz = rest[leg][0] - rest[leg][1] - FREE_CROUCH[gait]
            allowed = math.sqrt(max(rest[leg + "_reach"] ** 2 - dz * dz, 0.0))
            gain = min(gain, 0.92 * allowed / max(half, 1e-6))
        rest[key] = gain
    return rest[key]


def _periodic(values, phase):
    """Linear interpolation of a looping table at `phase` (0-1)."""
    n = len(values)
    x = (phase % 1.0) * n
    i = int(math.floor(x))
    w = x - i
    return values[i % n] * (1.0 - w) + values[(i + 1) % n] * w


def touchdown_phase(gait):
    """Phase of the table at which the near-fore touches down (maximum reach of F1)."""
    u = free_gaits()[gait]["legs"]["F1"]["u"]
    return max(range(len(u)), key=u.__getitem__) / len(u)


def apply_free_legs(harm, gait, phase, before, rest):
    """Body motion then hooves of the measured `gait` at table phase `phase`.

    `rest` = standing geometry from `_rest_geometry`.
    """
    table = free_gaits()[gait]
    unit = rest["unit"]
    lift = _periodic(table["rise"], phase) * FREE_RISE[gait] * unit - FREE_CROUCH[gait]
    hip = (poses.pos(harm, "BackLeg.L") + poses.pos(harm, "BackLeg.R")) / 2
    mean_pitch = sum(table["pitch"]) / len(table["pitch"])
    pitch = _periodic(table["pitch"], phase) - mean_pitch
    poses.rotate_about(harm, "Body", poses.X_AXIS, -FREE_PITCH[gait] * pitch, hip)
    poses.translate(harm, "Body", Vector((0.0, 0.0, lift)))
    for leg, name in FREE_LEG[gait].items():
        chain, _hoof = poses.HORSE_LEGS[leg]
        curve = _with_stance(table)[name]
        mean_u = sum(curve["u"]) / len(curve["u"])
        u = (_periodic(curve["u"], phase) - mean_u) * stride_gain(gait, rest)
        height = _periodic(curve["h"], phase) * FREE_LIFT[gait]
        root = poses.pos(harm, chain[0])
        _root_height, hoof_height = rest[leg]
        dz = root.z - (hoof_height + height * unit)  # root above the hoof
        reach = rest[leg + "_reach"]
        dy = u * unit
        if dz * dz + dy * dy > reach * reach:  # keep the hoof height, shorten the stride
            dy = math.copysign(math.sqrt(max(reach * reach - dz * dz, 0.0)), dy)
        target = Vector(
            (
                rest[leg + "_x"],
                root.y + rest[leg + "_dy"] - dy,  # the horse faces -Y
                root.z - dz,
            )
        )
        _plant_with_shoulder(harm, leg, before, target)
        if os.environ.get("AS8B_DEBUG"):
            got = poses._hoof_at(harm, leg, before).to_translation()
            print("PLANT", gait, leg, round(phase % 1, 2), "want", tuple(round(v, 2) for v in target), "got", tuple(round(v, 2) for v in got))
    poses._horse_feet(harm, before)


def _rest_geometry(harm):
    """Standing geometry of the legs (cached on the armature)."""
    key = (harm.as_pointer(), "free_rest")
    if key not in _CACHE:
        entry = _Entry(harm)
        for pb in harm.pose.bones:
            pb.matrix_basis.identity()
        bpy.context.view_layer.update()
        legs = poses.HORSE_LEGS
        rest = {"unit": sum(poses.pos(harm, c[0]).z for c, _h in legs.values()) / 4.0}
        for leg, (chain, hoof) in legs.items():
            rest[leg] = (poses.pos(harm, chain[0]).z, poses.pos(harm, hoof).z)
            rest[leg + "_x"] = poses.pos(harm, hoof).x
            rest[leg + "_dy"] = poses.pos(harm, hoof).y - poses.pos(harm, chain[0]).y
            heads = [poses.pos(harm, b) for b in (SHOULDER[leg], *chain)] + [
                poses.pos(harm, hoof)
            ]
            length = sum((b - a).length for a, b in zip(heads, heads[1:], strict=False))
            rest[leg + "_reach"] = 0.97 * length
        entry.restore()
        _CACHE[key] = rest
    return _CACHE[key]


def horse_trot_free(harm, t, frames=TROT_FRAMES):
    """Trot from the measured table: walk spine, legs placed on the measured hoof paths."""
    rest = _rest_geometry(harm)
    entry = _Entry(harm)
    phase = trot_phase(t, frames)
    pose = dict(_at(harm, WALK, phase * _frames(WALK)[1]))
    entry.restore()
    _commit(harm, pose)
    before = poses._horse_capture(harm)
    apply_free_legs(harm, "trot", phase + touchdown_phase("trot"), before, rest)
    poses.rotate_about(harm, "Tail1", poses.X_AXIS, TROT_TAIL)


def gallop_free(count=GALLOP_FRAMES, frames=GALLOP_FRAMES):
    """Horse override: the measured gallop (a stride lasts `frames` of the clip's `count`)."""

    def override(harm, t):
        rest = _rest_geometry(harm)
        entry = _Entry(harm)
        phase = (t * (count - 1) / frames) % 1.0
        pose = _at(harm, GALLOP, phase * _frames(GALLOP)[1])
        entry.restore()
        _commit(harm, pose)
        before = poses._horse_capture(harm)
        apply_free_legs(harm, "gallop", phase + touchdown_phase("gallop"), before, rest)

    return override


# --- Turns ----------------------------------------------------------------------------------


def _turn(harm, side, lean):
    """Bank the horse into a turn: `side` +1 = left, -1 = right (applied on a posed horse)."""
    before = poses._horse_capture(harm)
    hip = (poses.pos(harm, "BackLeg.L") + poses.pos(harm, "BackLeg.R")) / 2
    # Roll about the ground line under the hips (rotation about +Y: left side down).
    poses.rotate_about(harm, "Body", Y_AXIS, side * lean, Vector((hip.x, hip.y, 0.0)))
    poses.rotate_about(harm, "Torso3", poses.Z_AXIS, side * SPINE_BEND)
    poses.rotate_about(harm, "Neck1", poses.Z_AXIS, side * NECK_BEND)
    poses.rotate_about(harm, "Head", poses.Z_AXIS, side * HEAD_BEND)
    poses._horse_feet(harm, before)


def horse_turn_walk(side):
    """Horse override: the baker's walk frame, banked into the turn."""

    def override(harm, t):
        _turn(harm, side, WALK_LEAN)

    return override


def horse_turn_trot(side):
    """Horse override: the trot, banked into the turn."""

    def override(harm, t):
        horse_trot(harm, t)
        _turn(harm, side, TROT_LEAN)

    return override


# --- Riderless horse ------------------------------------------------------------------------


def horse_riderless(harm, t):
    """Horse whose rider has fallen: startles, then bolts at the gallop."""
    entry = _Entry(harm)
    i = t * (FALL_FRAMES - 1)
    startle = _sample(
        harm, STARTLE, _frames(STARTLE)[0] + min(int(i), STARTLE_FRAMES - 1)
    )
    bolt = _at(harm, GALLOP, i)
    w = poses.smooth(STARTLE_HOLD, STARTLE_HOLD + STARTLE_FADE, i / 24.0)
    pose = _blend(startle, bolt, w)
    entry.restore()
    _commit(harm, pose)


# --- Rider poses (the rider keeps his gear, the horse changes) ------------------------------


def with_horse(base, horse, frames=None):
    """Rider pose `base` paired with the horse override `horse` (and a clip length)."""

    def pose(arm, t):
        base(arm, t)

    pose.horse = horse
    if frames:
        pose.frames = frames
    pose.__doc__ = f"{base.__name__} on a horse with a modified gait."
    return pose


def ride_fall_free(arm, t):
    """Unhorsed rider over the long clip: he falls as before (first 30 frames), then lies."""
    scale = (FALL_FRAMES - 1) / (FALL_SOURCE_FRAMES - 1)
    poses.ride_fall(arm, min(t * scale, 1.0))


ride_fall_free.horse = horse_riderless
ride_fall_free.frames = FALL_FRAMES

# Looping clips (names only; the rows are built by `clip_specs`).
KITS = (
    ("", "ride_lance_up"),
    ("bow_", "ride_bow_rest"),
    ("javelin_", "ride_javelin_rest"),
)


def clip_specs():
    """AS3 rows of `battle_skinned_cavalry.clip_specs`.

    Each row is (clip, horse action, rider action, rider pose, mirror, frames).
    """
    rows = []
    for prefix, rider_name in KITS:
        rider = getattr(poses, rider_name)
        rows.append(
            (
                f"c_{prefix}trot",
                WALK,
                "Idle",
                with_horse(rider, horse_trot),
                False,
                TROT_FRAMES,
            )
        )
    rows.append(
        (
            "c_std_trot",
            WALK,
            "Idle",
            with_horse(poses.ride_std_up, horse_trot),
            False,
            TROT_FRAMES,
        )
    )
    for prefix, rider_name in KITS:
        rider = getattr(poses, rider_name)
        for name, side in (("l", 1), ("r", -1)):
            rows.append(
                (
                    f"c_{prefix}turn_{name}",
                    WALK,
                    "Idle",
                    with_horse(rider, horse_turn_walk(side)),
                    False,
                    None,
                )
            )
            rows.append(
                (
                    f"c_{prefix}trot_turn_{name}",
                    WALK,
                    "Idle",
                    with_horse(rider, horse_turn_trot(side)),
                    False,
                    TROT_FRAMES,
                )
            )
    return rows


FREE_GALLOP_ROWS = {"c_gallop": 15, "c_charge": 15, "c_std_gallop": 15, "c_std_charge": 30}


def free_rows(rows):
    """Rows of `battle_skinned_cavalry.clip_specs` with the gallops of the measured table."""
    if not free_enabled():
        return rows
    out = []
    for row in rows:
        clip, horse_act, rider_act, pose, mirror, frames = row
        if clip in FREE_GALLOP_ROWS:
            count = FREE_GALLOP_ROWS[clip]
            pose = with_horse(pose, gallop_free(count), frames)
            row = (clip, horse_act, rider_act, pose, mirror, frames)
        out.append(row)
    return out


def loop_names():
    """Names of the AS3 clips that loop (all but the riderless fall)."""
    return tuple(row[0] for row in clip_specs())
