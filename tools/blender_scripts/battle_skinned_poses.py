"""Bone overrides that turn Quaternius actions into the missing clips (lot V2).

A pose function `pose(arm, t)` runs after the source action frame is evaluated (`t` in
[0, 1] over the clip) and moves bones in world space: aim a bone at a point, two-bone IK for
arms and legs, rotations about a pivot. It may also place the virtual bones of the rig
through `STATE` (reset before every frame):

- `prop`: world matrix of the right-hand prop (pike, crossbow) or None (follows the right
  wrist); axes X = hand direction at rest, Y = weapon axis (towards the tip), Z = up;
- `nock`: world position of the bow string's nock point or None (string at rest on the bow);
- `arrow`: True to show the nocked arrow.

Optional attributes of a pose function: `frames` (clip length, default = source length),
`source_frames` (callable t -> source frame, e.g. reversed playback).
World space is Blender's: Z up, the figure faces -Y, its right hand is at -X.
"""

import math

import bpy
from mathutils import Matrix, Vector

STATE = {"prop": None, "nock": None, "arrow": False}
REST = {}  # filled by the baker: rest world matrices ("prop", bone names), grip offsets


def reset_state():
    """Clear the virtual prop/nock/arrow state before baking a new frame."""
    STATE["prop"] = None
    STATE["nock"] = None
    STATE["arrow"] = False


def smooth(a, b, t):
    """Smoothstep of `t` between `a` and `b`, clamped to [0, 1]."""
    x = min(max((t - a) / (b - a), 0.0), 1.0) if b != a else float(t >= a)
    return x * x * (3 - 2 * x)


def lerp(a, b, t):
    """Linear interpolation between `a` and `b` by `t`."""
    return a + (b - a) * t


# --- World-space bone helpers -----------------------------------------------------------


def world(arm, bone):
    """Current world matrix of a pose bone."""
    return arm.matrix_world @ arm.pose.bones[bone].matrix


def set_world(arm, bone, m):
    """Set a pose bone's world matrix and refresh the dependency graph."""
    arm.pose.bones[bone].matrix = arm.matrix_world.inverted() @ m
    bpy.context.view_layer.update()


def pos(arm, bone):
    """Current world position of a pose bone."""
    return world(arm, bone).to_translation()


# Child whose head gives the direction of a limb (the imported bones' own axes are not
# reliably along the limbs).
LIMB_CHILD = {
    "UpperArm.L": "LowerArm.L",
    "LowerArm.L": "Wrist.L",
    "UpperArm.R": "LowerArm.R",
    "LowerArm.R": "Wrist.R",
    "UpperLeg.L": "LowerLeg.L",
    "UpperLeg.R": "LowerLeg.R",
}


def limb_dir(arm, bone):
    """Current direction of a limb.

    Towards its child's head, else the forearm's for a wrist (the hand extends the forearm
    at rest), else the bone's own axis.
    """
    child = LIMB_CHILD.get(bone)
    if child:
        return (pos(arm, child) - pos(arm, bone)).normalized()
    if bone.startswith("Wrist."):
        rest_rot = REST[bone].to_3x3().normalized()
        local = rest_rot.inverted() @ REST["forearm." + bone[-1]]
        return (world(arm, bone).to_3x3().normalized() @ local).normalized()
    return (world(arm, bone).to_3x3().normalized() @ Vector((0, 1, 0))).normalized()


def aim(arm, bone, direction):
    """Rotate `bone` (minimal rotation) so that its limb points along `direction`."""
    m = world(arm, bone)
    loc, rot, scale = m.decompose()
    y = limb_dir(arm, bone)
    q = y.rotation_difference(direction.normalized())
    set_world(arm, bone, Matrix.LocRotScale(loc, q @ rot, scale))


def rotate_about(arm, bone, axis, angle, pivot=None):
    """Rotate `bone` (and its children) by `angle` about a world axis through `pivot`."""
    m = world(arm, bone)
    p = pivot if pivot is not None else m.to_translation()
    r = Matrix.Translation(p) @ Matrix.Rotation(angle, 4, axis) @ Matrix.Translation(-p)
    set_world(arm, bone, r @ m)


def translate(arm, bone, offset):
    """Translate `bone` by `offset` in world space."""
    m = world(arm, bone)
    set_world(arm, bone, Matrix.Translation(offset) @ m)


def ik2(arm, upper, lower, end, target, pole):
    """Two-bone IK: `upper`/`lower` reach `target` with the joint bent towards `pole`."""
    s = pos(arm, upper)
    a = (pos(arm, lower) - s).length
    b = (pos(arm, end) - pos(arm, lower)).length
    to_t = target - s
    d = min(to_t.length, (a + b) * 0.999)
    dir_t = to_t.normalized()
    # Joint position: distance along the target line and bend towards the pole.
    x = (a * a - b * b + d * d) / (2 * d)
    h = math.sqrt(max(a * a - x * x, 0.0))
    side = pole - s
    side = (side - dir_t * side.dot(dir_t)).normalized()
    joint = s + dir_t * x + side * h
    aim(arm, upper, joint - s)
    aim(arm, lower, (s + dir_t * d) - pos(arm, lower))


def orient_like(arm, bone, rotation_world):
    """Give `bone` a world rotation (keeping its position and scale)."""
    m = world(arm, bone)
    loc, _rot, scale = m.decompose()
    set_world(arm, bone, Matrix.LocRotScale(loc, rotation_world, scale))


def _frame(a, b):
    a = a.normalized()
    b = (b - a * b.dot(a)).normalized()
    return Matrix((a, b, a.cross(b))).transposed()


def orient_rest_axes(arm, bone, rest_a, target_a, rest_b, target_b):
    """World rotation of `bone` mapping two rest-pose directions onto two targets.

    Maps `rest_a`/`rest_b` (world directions of its rest pose) onto `target_a` (exact) and
    `target_b` (as close as possible).
    """
    r = _frame(target_a, target_b) @ _frame(rest_a, rest_b).inverted()
    rest_rot = REST[bone].to_3x3().normalized()
    orient_like(arm, bone, (r @ rest_rot).to_quaternion())


def bow_upright(arm, forward):
    """Left fist turned so that the bow limbs stand vertical, the arrow along `forward`."""
    arm_dir = REST["forearm.L"]
    orient_rest_axes(
        arm, "Wrist.L", arm_dir, forward, Vector((0, -1, 0)), Vector((0, 0, 1))
    )


def hand_to_prop(arm, side, prop, along, pole):
    """IK the arm so that the fist sits on the prop at `along` metres on its axis."""
    target = prop @ Vector((0, along, 0))
    wrist_target = target - (prop.to_3x3() @ Vector((1, 0, 0))).normalized() * 0.075 * (
        1 if side == "R" else -1
    )
    ik2(
        arm, f"UpperArm.{side}", f"LowerArm.{side}", f"Wrist.{side}", wrist_target, pole
    )


def fist_on_prop(arm, prop):
    """Right wrist oriented as at rest relative to the prop (grip matches the model)."""
    rest_prop = REST["prop"]
    rest_wrist = REST["Wrist.R"]
    m = prop @ rest_prop.inverted() @ rest_wrist
    orient_like(arm, "Wrist.R", m.to_3x3().normalized().to_quaternion())


def prop_matrix(origin, axis, up):
    """World matrix of a prop: Y along `axis`, Z towards `up`, X = Y x Z."""
    y = axis.normalized()
    x = y.cross(up).normalized()
    z = x.cross(y).normalized()
    m = Matrix((x, y, z)).transposed().to_4x4()
    m.translation = origin
    return m


def aim_dir(elevation_deg, yaw_deg=0.0):
    """Forward (-Y) direction raised by `elevation_deg`, turned by `yaw_deg` (left +)."""
    e = math.radians(elevation_deg)
    y = math.radians(yaw_deg)
    return Vector((-math.sin(y) * math.cos(e), -math.cos(y) * math.cos(e), math.sin(e)))


# --- Pikes --------------------------------------------------------------------------------


def _pike_upright(arm):
    """Pike held upright, butt on the ground by the right foot."""
    foot = pos(arm, "Foot.R")
    butt = Vector((foot.x - 0.12, foot.y - 0.18, 0.0))
    prop = prop_matrix(
        butt + Vector((0, 0, 1.25)), Vector((0.03, -0.05, 1.0)), Vector((0, -1, 0))
    )
    STATE["prop"] = prop
    hand_to_prop(
        arm, "R", prop, 0.0, pos(arm, "UpperArm.R") + Vector((-0.4, 0.3, -0.4))
    )
    fist_on_prop(arm, prop)


def _pike_level(arm, thrust=0.0):
    """Pike levelled forwards at the waist, both hands, pushed forwards by `thrust` m."""
    chest = pos(arm, "Chest")
    grip = Vector(
        (chest.x - 0.16, chest.y + 0.12 - thrust, chest.z - 0.28 + 0.04 * thrust)
    )
    prop = prop_matrix(grip, Vector((0.08, -1.0, 0.06)), Vector((0, 0, 1)))
    STATE["prop"] = prop
    hand_to_prop(arm, "R", prop, 0.0, grip + Vector((-0.5, 0.4, -0.3)))
    fist_on_prop(arm, prop)
    hand_to_prop(arm, "L", prop, 0.55, grip + Vector((0.5, 0.1, -0.4)))


def pike_hold(arm, t):
    """Pose: pike held upright, butt on the ground."""
    _pike_upright(arm)


def pike_level(arm, t):
    """Pose: pike levelled forwards at the waist."""
    _pike_level(arm)


def pike_thrust(arm, t):
    """Pose: pike thrust forwards, torso pitching with the push."""
    push = smooth(0.15, 0.4, t) * (1 - smooth(0.55, 0.95, t))
    rotate_about(arm, "Torso", Vector((1, 0, 0)), 0.18 * push)
    _pike_level(arm, thrust=0.35 * push)


pike_thrust.frames = 28


# --- Longbow ------------------------------------------------------------------------------

BOW_ELEVATION = 32.0
DRAW_LENGTH = 0.70


def _bow_arm(arm, raise_t):
    """Bow arm from hanging (0) to aimed (1); torso turned side-on."""
    rotate_about(arm, "Torso", Vector((0, 0, 1)), math.radians(-38) * raise_t)
    shoulder = pos(arm, "UpperArm.L")
    aim_v = aim_dir(BOW_ELEVATION * raise_t + 0.0 * (1 - raise_t), -8.0)
    rest_target = shoulder + Vector((0.08, -0.38, -0.42))
    target = rest_target.lerp(shoulder + aim_v * 0.6, raise_t)
    ik2(
        arm,
        "UpperArm.L",
        "LowerArm.L",
        "Wrist.L",
        target,
        shoulder + Vector((0.4, 0.2, -0.6)),
    )
    # Bow vertical, arrow along the forearm (canted 10 degrees like English archers).
    fore = (pos(arm, "Wrist.L") - pos(arm, "LowerArm.L")).normalized()
    bow_upright(arm, fore)
    rotate_about(arm, "Wrist.L", fore, math.radians(-10))
    return aim_v


def _draw_hand(arm, draw, aim_v):
    grip = (
        pos(arm, "Wrist.L")
        + (pos(arm, "Wrist.L") - pos(arm, "LowerArm.L")).normalized() * 0.075
    )
    nock = grip - aim_v * (0.16 + (DRAW_LENGTH - 0.16) * draw)
    right = Vector((-1, 0, 0))
    hand = nock + right * 0.02
    ik2(
        arm,
        "UpperArm.R",
        "LowerArm.R",
        "Wrist.R",
        hand - aim_v * 0.02,
        pos(arm, "UpperArm.R") + Vector((-0.5, 0.5, 0.1)),
    )
    aim(arm, "Wrist.R", aim_v)
    return nock


def bow_rest(arm, t):
    """Longbow held low in front, string at rest."""
    _bow_arm(arm, 0.0)


def bow_shoot(arm, t):
    """Nock (0-0.2), raise and draw (0.2-0.55), hold, loose (0.62), recover to rest."""
    raise_t = smooth(0.12, 0.42, t) * (1 - smooth(0.72, 0.95, t))
    aim_v = _bow_arm(arm, raise_t)
    draw = smooth(0.25, 0.55, t) * (1 - smooth(0.615, 0.63, t))
    if t < 0.63:
        STATE["nock"] = _draw_hand(arm, draw, aim_v)
        STATE["arrow"] = t > 0.08
    else:
        # After the loose the right hand flies back, then drops.
        back = smooth(0.63, 0.7, t) * (1 - smooth(0.75, 0.95, t))
        hand = (
            pos(arm, "UpperArm.R")
            + Vector((-0.12, 0.25, -0.1)) * back
            + Vector((0.0, -0.1, -0.45)) * (1 - back)
        )
        ik2(
            arm,
            "UpperArm.R",
            "LowerArm.R",
            "Wrist.R",
            hand,
            pos(arm, "UpperArm.R") + Vector((-0.5, 0.5, -0.2)),
        )


bow_shoot.frames = 60  # release at 0.62 * 60 / 24 = 1.55 s


# --- Crossbow -----------------------------------------------------------------------------


def _crossbow_aim(arm, raise_t, elevation=6.0):
    """Crossbow shouldered (1) or held low in front (0)."""
    shoulder = pos(arm, "UpperArm.R")
    head = pos(arm, "Head")
    aim_v = aim_dir(elevation * raise_t - 25.0 * (1 - raise_t), -4.0)
    aimed = Vector((shoulder.x + 0.06, head.y - 0.22, head.z - 0.06))
    low = Vector((shoulder.x + 0.12, shoulder.y - 0.3, shoulder.z - 0.45))
    grip = low.lerp(aimed, raise_t)
    prop = prop_matrix(grip, aim_v, Vector((0, 0, 1)))
    STATE["prop"] = prop
    hand_to_prop(arm, "R", prop, 0.0, shoulder + Vector((-0.4, 0.3, -0.5)))
    fist_on_prop(arm, prop)
    hand_to_prop(
        arm, "L", prop, 0.32, pos(arm, "UpperArm.L") + Vector((0.4, 0.1, -0.6))
    )


def crossbow_rest(arm, t):
    """Pose: crossbow held low in front, at rest."""
    _crossbow_aim(arm, 0.0)


def crossbow_shoot(arm, t):
    """Aim, loose (0.06), lower nose to the ground, span with the belt hook, raise and aim."""
    lower = smooth(0.1, 0.22, t) * (1 - smooth(0.78, 0.92, t))
    if lower < 0.01:
        _crossbow_aim(arm, 1.0)
        return
    # Spanning: bend forwards, crossbow nose down in front of the feet, hands on the string.
    bend = smooth(0.18, 0.3, t) * (1 - smooth(0.62, 0.74, t))
    pull = smooth(0.32, 0.6, t)
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.75 * bend)
    feet = (pos(arm, "Foot.L") + pos(arm, "Foot.R")) / 2
    nose = Vector((feet.x, feet.y - 0.42, 0.05))
    up = Vector((0.0, 0.25, 1.0)).normalized()
    upright = nose + up * 0.7
    aimed_grip = Vector(
        (
            pos(arm, "UpperArm.R").x + 0.06,
            pos(arm, "Head").y - 0.22,
            pos(arm, "Head").z - 0.06,
        )
    )
    grip = upright.lerp(aimed_grip, 1 - lower)
    axis = (-up).lerp(aim_dir(6.0, -4.0), 1 - lower)
    prop = prop_matrix(grip, axis, Vector((0, -1, 0)))
    STATE["prop"] = prop
    along = lerp(0.3, 0.05, pull * bend) if bend > 0 else 0.0
    hand_to_prop(
        arm, "R", prop, along - 0.05, pos(arm, "UpperArm.R") + Vector((-0.5, -0.2, 0.0))
    )
    hand_to_prop(
        arm, "L", prop, along + 0.02, pos(arm, "UpperArm.L") + Vector((0.5, -0.2, 0.0))
    )


crossbow_shoot.frames = 120  # 5 s: release at 0.35 s, spanning, back on aim at 4.4 s


# --- Deaths and knock-downs ---------------------------------------------------------------


def climb(arm, t):
    """Scaling a ladder (SG1, loop): leaning in, hands and feet reaching up rung by rung."""
    phase = t * 2.0 * math.pi
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.22)
    translate(arm, "Body", Vector((0, 0, 0.05 * math.sin(2.0 * phase))))
    hips = pos(arm, "Body")
    for side, sx, shift in (("R", -1.0, 0.0), ("L", 1.0, math.pi)):
        reach = math.sin(phase + shift)
        shoulder = pos(arm, f"UpperArm.{side}")
        hand = shoulder + Vector((0.05 * sx, -0.34, 0.22 + 0.3 * reach))
        ik2(
            arm,
            f"UpperArm.{side}",
            f"LowerArm.{side}",
            f"Wrist.{side}",
            hand,
            shoulder + Vector((0.5 * sx, 0.3, -0.5)),
        )
        aim(arm, f"Wrist.{side}", Vector((0, -0.4, 1.0)))
        # Foot on the rung below the opposite hand: lifted while that hand is low.
        foot = pos(arm, f"Foot.{side}")
        lift = max(0.0, -reach)
        target = foot + Vector((0, -0.16 - 0.1 * lift, 0.34 * lift))
        ik2(
            arm,
            f"UpperLeg.{side}",
            f"LowerLeg.{side}",
            f"Foot.{side}",
            target,
            hips + Vector((0, -1.0, -0.3)),
        )
        translate(arm, f"Foot.{side}", target - foot)


climb.frames = 24


def death_knees(arm, t):
    """Knees give way, then the body pitches forwards onto the face."""
    sink = smooth(0.0, 0.35, t)
    fall = smooth(0.35, 0.8, t)
    hips = pos(arm, "Body")
    translate(arm, "Body", Vector((0, 0.05 * sink, -0.42 * sink)))
    for side in ("L", "R"):
        foot = pos(arm, f"Foot.{side}")
        ik2(
            arm,
            f"UpperLeg.{side}",
            f"LowerLeg.{side}",
            f"Foot.{side}",
            foot,
            hips + Vector((0, -1.0, -0.3)),
        )
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.4 * sink)
    arms_down = Vector((0, -0.2, -1.0))
    for side in ("L", "R"):
        aim(
            arm,
            f"UpperArm.{side}",
            arms_down + Vector((0.3 if side == "L" else -0.3, 0, 0)),
        )
    if fall > 0:
        knees = (pos(arm, "LowerLeg.L") + pos(arm, "LowerLeg.R")) / 2
        rotate_about(
            arm,
            "Root",
            Vector((1, 0, 0)),
            1.45 * fall,
            Vector((knees.x, knees.y, 0.08)),
        )


death_knees.frames = 30


def death_back(arm, t):
    """Thrown backwards (charge impact): lifted, flung back two metres, flat on the back."""
    fly = smooth(0.0, 0.55, t)
    hop = math.sin(min(t / 0.55, 1.0) * math.pi) * 0.35
    translate(arm, "Root", Vector((0, 1.8 * fly, hop)))
    rotate_about(
        arm, "Root", Vector((1, 0, 0)), -1.5 * smooth(0.05, 0.6, t), pos(arm, "Root")
    )
    for side, sx in (("L", 1), ("R", -1)):
        aim(arm, f"UpperArm.{side}", Vector((0.7 * sx, 0.3, 0.6)))


death_back.frames = 26


def knockdown(arm, t):
    """Knocked down (Death played forwards), a moment on the ground, then back on the feet."""


def _knockdown_frame(i, count, first, last):
    down = last - first
    if i <= down:
        return first + i
    if i <= down + 12:
        return last
    return max(first, last - (i - down - 12))


knockdown.frames = 26 + 12 + 26
knockdown.source_frame = _knockdown_frame


# --- Riders (the cavalry baker puts the rider on the saddle before these run) ------------

RIDE = {}


def _mount():
    return RIDE["mount"]


def _hv(v):
    """Horse-relative direction to world (follows the saddle's pitch and roll)."""
    return (_mount().delta().to_3x3().normalized() @ v).normalized()


def _hp(p):
    """Rest-pose world point carried by the saddle."""
    return _mount().delta() @ p


def _ride_legs(arm):
    """Thighs astride the barrel, feet in the stirrups."""
    m = _mount()
    for side, sx in (("L", 1), ("R", -1)):
        foot = _hp(m.stirrups[side])
        pole = pos(arm, f"UpperLeg.{side}") + _hv(Vector((0.5 * sx, -1.0, 0.1)))
        ik2(arm, f"UpperLeg.{side}", f"LowerLeg.{side}", f"Foot.{side}", foot, pole)
        # The foot bones hang from the root, not the shins: move them to the ankle.
        ankle = (
            pos(arm, f"LowerLeg.{side}")
            + limb_dir(arm, f"LowerLeg.{side}")
            * (foot - pos(arm, f"LowerLeg.{side}")).length
        )
        fm = world(arm, f"Foot.{side}")
        fm.translation = ankle
        set_world(arm, f"Foot.{side}", fm)


def _reins(arm, side="L"):
    m = _mount()
    target = _hp(m.pommel + Vector((0.06 if side == "L" else -0.06, -0.12, 0.12)))
    ik2(
        arm,
        f"UpperArm.{side}",
        f"LowerArm.{side}",
        f"Wrist.{side}",
        target,
        pos(arm, f"UpperArm.{side}")
        + _hv(Vector((0.6 if side == "L" else -0.6, 0.2, -0.6))),
    )


def _lean(arm, amount):
    rotate_about(arm, "Abdomen", _hv(Vector((1, 0, 0))), amount)


def _lance_at(arm, grip, axis, up=None):
    prop = prop_matrix(grip, axis, up if up is not None else _hv(Vector((0, 0, 1))))
    STATE["prop"] = prop
    hand_to_prop(
        arm, "R", prop, 0.0, pos(arm, "UpperArm.R") + _hv(Vector((-0.6, 0.3, -0.5)))
    )
    fist_on_prop(arm, prop)


def ride_lance_up(arm, t):
    """At a halt or walking: lance upright, butt by the right stirrup."""
    m = _mount()
    _ride_legs(arm)
    _reins(arm)
    butt = _hp(m.stirrups["R"] + Vector((-0.08, -0.05, 0.2)))
    axis = _hv(Vector((-0.05, -0.18, 1.0)))
    _lance_at(arm, butt + axis * 1.1, axis, _hv(Vector((0, -1, 0))))


def ride_lance_raised(arm, t):
    """Gallop: leaning forwards, lance slanted up and forwards."""
    _ride_legs(arm)
    _lean(arm, 0.2)
    _reins(arm)
    grip = pos(arm, "Chest") + _hv(Vector((-0.24, -0.12, -0.2)))
    _lance_at(arm, grip, _hv(Vector((-0.05, -0.7, 0.7))))


def ride_lance_couched(arm, t):
    """Charge: lance couched under the right arm, aimed forwards across the neck."""
    _ride_legs(arm)
    _lean(arm, 0.35)
    _reins(arm)
    grip = pos(arm, "Chest") + _hv(Vector((-0.2, -0.18, -0.2)))
    _lance_at(arm, grip, _hv(Vector((0.22, -1.0, -0.04))))


def ride_lance_thrust(arm, t):
    """Melee: short thrusts of the lance, held overhand, downwards and forwards."""
    _ride_legs(arm)
    push = smooth(0.1, 0.35, t) * (1 - smooth(0.5, 0.9, t))
    _lean(arm, 0.1 + 0.2 * push)
    _reins(arm)
    grip = pos(arm, "Chest") + _hv(
        Vector((-0.28, -0.05 - 0.4 * push, 0.05 - 0.1 * push))
    )
    _lance_at(arm, grip, _hv(Vector((0.05, -1.0, -0.35))))


ride_lance_thrust.frames = 28


def ride_bow_rest(arm, t):
    """Pose: mounted, longbow held low in front at rest."""
    _ride_legs(arm)
    _bow_arm(arm, 0.0)
    _reins(arm, "R")


def ride_bow_shoot(arm, t):
    """Pose: mounted, playing the longbow shoot pose while seated in the saddle."""
    _ride_legs(arm)
    bow_shoot(arm, t)


def _javelin_grip(arm, cock, aim_v=None):
    """Right-hand javelin held as a prop: `cock` 1 = drawn back by the ear, 0 = arm thrown
    fully forward (release). Left hand stays on the reins (`ride_javelin_*`)."""
    shoulder = pos(arm, "UpperArm.R")
    if aim_v is None:
        aim_v = aim_dir(10.0, -4.0)
    back = shoulder + Vector((-0.06, 0.16, 0.3))
    forward = shoulder + aim_v * 0.68
    grip = forward.lerp(back, cock)
    prop = prop_matrix(grip, aim_v, Vector((0, 0, 1)))
    STATE["prop"] = prop
    hand_to_prop(arm, "R", prop, 0.0, shoulder + Vector((-0.3, 0.3, 0.15)))
    fist_on_prop(arm, prop)
    return aim_v


def ride_javelin_rest(arm, t):
    """Pose: mounted jinete, a javelin held cocked back by the ear, ready to throw."""
    _ride_legs(arm)
    _javelin_grip(arm, 1.0)
    _reins(arm, "L")


def ride_javelin_throw(arm, t):
    """Cocked (0-0.35), snapped forward to release (0.35-0.55), arm recovers to cocked.

    No nock/draw like the bow: a jinete carries two or three loose javelins (azagayas) and
    throws them one at a time, so the clip loops back to the cocked pose rather than to rest.
    """
    _ride_legs(arm)
    cock = 1.0 - smooth(0.35, 0.55, t)
    _javelin_grip(arm, max(cock, 0.0))
    _reins(arm, "L")


ride_javelin_throw.frames = 30  # release at 0.55 * 30 / 24 = 0.69 s


def ride_death(arm, t):
    """Horse and rider down: the rider slides off to the left and falls on his back."""
    m = _mount()
    s = smooth(0.15, 0.7, t)
    if s < 1e-3:
        _ride_legs(arm)
        return
    ground = Matrix.Translation(Vector((0.95, 0.15, -0.6)))
    m.seat_rider(m.delta().lerp(ground, s))


def ride_fall(arm, t):
    """Unhorsed: the rider is thrown backwards over the croup (the horse stays up)."""
    m = _mount()
    s = smooth(0.0, 0.55, t)
    hop = math.sin(min(t / 0.55, 1.0) * math.pi) * 0.35
    ground = Matrix.Translation(Vector((0.15, 1.5, -0.6 + hop)))
    m.seat_rider(m.delta().lerp(ground, s))


# --- Lot EP5: standard bearers and musicians ----------------------------------------------

# Values kept between the frames of one clip (frames are baked in order, t = 0 first).
CLIP_CACHE = {}
STD_BELOW = 1.2  # pole length under the upper (right) hand; = weapons.STANDARD_BELOW
STD_MOUNTED_BELOW = 1.1  # = weapons.STANDARD_MOUNTED_BELOW
STD_LEFT_HAND = -0.45  # left fist along the pole, under the right one


def _std_grip(arm, forward=0.0, lift=0.0, side=0.0):
    """Upper-hand position of the standard: in front of the right side of the chest."""
    return pos(arm, "Chest") + Vector((-0.2 + side, -0.28 - forward, 0.02 + lift))


def _std_hands(arm, prop, left=True, release=0.0):
    """Both fists on the pole (right high, left low); `release` 0-1 lets go (death)."""
    wrist_r = pos(arm, "Wrist.R")
    wrist_l = pos(arm, "Wrist.L")
    if release < 1.0:
        target = prop @ Vector((0, 0, 0))
        target = target.lerp(wrist_r, release)
        grip_prop = prop.copy()
        grip_prop.translation = target
        hand_to_prop(
            arm, "R", grip_prop, 0.0, pos(arm, "UpperArm.R") + Vector((-0.5, 0.3, -0.4))
        )
        if release < 0.05:
            fist_on_prop(arm, prop)
    if left and release < 1.0:
        target = (prop @ Vector((0, STD_LEFT_HAND, 0))).lerp(wrist_l, release)
        grip_prop = prop.copy()
        grip_prop.translation = target
        hand_to_prop(
            arm, "L", grip_prop, 0.0, pos(arm, "UpperArm.L") + Vector((0.5, 0.2, -0.6))
        )


def _std_hold(arm, axis, forward=0.0, lift=0.0, side=0.0):
    grip = _std_grip(arm, forward, lift, side)
    prop = prop_matrix(grip, axis, Vector((0, -1, 0)))
    STATE["prop"] = prop
    _std_hands(arm, prop)
    return prop


def std_idle(arm, t):
    """Standard held upright in both hands, swaying gently (loop)."""
    s = math.sin(2 * math.pi * t)
    _std_hold(arm, Vector((0.03 * s, -0.05, 1.0)))


def std_walk(arm, t):
    """Walking with the standard upright (loop, cadence of `Walk`)."""
    s = math.sin(4 * math.pi * t)
    _std_hold(arm, Vector((0.025 * s, -0.08, 1.0)))


def std_run(arm, t):
    """Running / charging: the standard raised and slanted forwards (loop)."""
    s = math.sin(4 * math.pi * t)
    _std_hold(arm, Vector((0.03 * s, -0.32, 1.0)), forward=0.08, lift=0.08)


def std_wave(arm, t):
    """Melee: the standard brandished overhead, swept from side to side (loop)."""
    phase = 2 * math.pi * t
    s = math.sin(phase)
    rotate_about(arm, "Torso", Vector((0, 0, 1)), 0.18 * s)
    lift = 0.22 + 0.08 * abs(math.sin(phase))
    _std_hold(
        arm,
        Vector((0.55 * s, -0.12 - 0.1 * abs(s), 1.0)),
        forward=0.05,
        lift=lift,
        side=0.08 * s,
    )


std_wave.frames = 58  # 2.4 s


def std_death(arm, t):
    """Struck down (`Death`, falling backwards): the pole goes down with him, toppling
    backwards and a little to the right about its butt, and lies on the ground."""
    if t == 0.0 or "std_death" not in CLIP_CACHE:
        grip = _std_grip(arm)
        axis = Vector((0.0, -0.05, 1.0)).normalized()
        CLIP_CACHE["std_death"] = (grip - axis * STD_BELOW, axis)
    butt0, axis0 = CLIP_CACHE["std_death"]
    fall = smooth(0.05, 0.62, t)
    rot = Matrix.Rotation(-0.3 * fall, 3, "Y") @ Matrix.Rotation(-1.58 * fall, 3, "X")
    axis = (rot @ axis0).normalized()
    butt = butt0 + Vector((-0.1, 0.5, 0.0)) * fall
    butt.z = lerp(butt0.z, 0.03, fall)
    grip = butt + axis * STD_BELOW
    prop = prop_matrix(grip, axis, Vector((0, -1, 0)) if axis.z > 0.5 else Vector((0, 0, 1)))
    STATE["prop"] = prop
    _std_hands(arm, prop, release=smooth(0.35, 0.6, t))


# Mounted standard bearer (the cavalry baker seats the rider before these run).


def _ride_std(arm, tilt=0.0, sway=0.0, lift=0.0, both=False):
    m = _mount()
    butt = _hp(m.stirrups["R"] + Vector((-0.1, -0.06, 0.24 + lift)))
    axis = _hv(Vector((-0.04 + sway, -0.1 - tilt, 1.0)))
    grip = butt + axis * STD_MOUNTED_BELOW
    prop = prop_matrix(grip, axis, _hv(Vector((0, -1, 0))))
    STATE["prop"] = prop
    hand_to_prop(
        arm, "R", prop, 0.0, pos(arm, "UpperArm.R") + _hv(Vector((-0.6, 0.3, -0.5)))
    )
    fist_on_prop(arm, prop)
    if both:
        hand_to_prop(
            arm,
            "L",
            prop,
            STD_LEFT_HAND,
            pos(arm, "UpperArm.L") + _hv(Vector((0.5, 0.2, -0.6))),
        )
    else:
        _reins(arm)
    return prop


def ride_std_up(arm, t):
    """At a halt or walking: the standard upright, butt by the right stirrup."""
    _ride_legs(arm)
    _ride_std(arm, sway=0.03 * math.sin(2 * math.pi * t))


def ride_std_gallop(arm, t):
    """Gallop: leaning forwards, the standard slanted forwards."""
    _ride_legs(arm)
    _lean(arm, 0.15)
    _ride_std(arm, tilt=0.25, lift=0.05)


def ride_std_wave(arm, t):
    """Melee: the standard raised in both hands and swept from side to side."""
    _ride_legs(arm)
    s = math.sin(2 * math.pi * t)
    rotate_about(arm, "Torso", _hv(Vector((0, 0, 1))), 0.15 * s)
    _ride_std(arm, sway=0.45 * s, tilt=0.05, lift=0.2 + 0.08 * abs(s), both=True)


ride_std_wave.frames = 58


def ride_std_death(arm, t):
    """Horse and rider down: the standard falls to the right about its butt."""
    m = _mount()
    if t == 0.0 or "ride_std_death" not in CLIP_CACHE:
        _ride_legs(arm)
        prop = _ride_std(arm)
        CLIP_CACHE["ride_std_death"] = (
            prop @ Vector((0, -STD_MOUNTED_BELOW, 0)),
            (prop.to_3x3() @ Vector((0, 1, 0))).normalized(),
        )
    butt0, axis0 = CLIP_CACHE["ride_std_death"]
    ride_death(arm, t)
    fall = smooth(0.1, 0.7, t)
    # Falls forwards, ahead of the horse (the cloth lies clear of the bodies).
    rot = Matrix.Rotation(1.53 * fall, 3, Vector((1.0, 0.15, 0.0)).normalized())
    axis = (rot @ axis0).normalized()
    butt = butt0 + Vector((0.2, -0.2, 0.0)) * fall
    butt.z = lerp(butt0.z, 0.03, fall)
    grip = butt + axis * STD_MOUNTED_BELOW
    STATE["prop"] = prop_matrix(grip, axis, Vector((0, -1, 0)) if axis.z > 0.5 else Vector((0, 0, 1)))
    _ = m


# Drummer: tabor at the waist (see `battle_skinned_weapons.tabor`), two sticks.


def _drum_top(arm):
    import battle_skinned_weapons as weapons

    hips = world(arm, "Hips")
    rest = REST.get("Hips")
    centre = pos(arm, "Hips") + weapons.DRUM_OFFSET
    if rest is not None:
        # Follow the rotation of the hips (the drum is bound to them).
        delta = hips.to_3x3().normalized() @ rest.to_3x3().normalized().inverted()
        centre = pos(arm, "Hips") + delta @ weapons.DRUM_OFFSET
        axis = (delta @ weapons.DRUM_AXIS).normalized()
    else:
        axis = weapons.DRUM_AXIS
    return centre + axis * weapons.DRUM_DEPTH * 0.5, axis


def _drum_hands(arm, beats, t):
    """Alternate strokes of the two sticks, `beats` strokes per hand over the clip."""
    top, axis = _drum_top(arm)
    for side, sx, shift in (("R", -1.0, 0.0), ("L", 1.0, math.pi)):
        lift = 0.5 + 0.5 * math.cos(2 * math.pi * beats * t + shift)
        lift = lift**1.6  # quick stroke, longer hold up
        hit = top + Vector((0.055 * sx, 0.0, 0.0))
        stick = Vector((0.0, -0.45, -0.89)).normalized()
        wrist = hit - stick * 0.3 + Vector((0.05 * sx, 0.0, 0.0)) + Vector((0, 0.03, 0.13)) * lift
        shoulder = pos(arm, f"UpperArm.{side}")
        ik2(
            arm,
            f"UpperArm.{side}",
            f"LowerArm.{side}",
            f"Wrist.{side}",
            wrist,
            shoulder + Vector((0.5 * sx, 0.3, -0.4)),
        )
        fore = (pos(arm, f"Wrist.{side}") - pos(arm, f"LowerArm.{side}")).normalized()
        tip_target = hit + Vector((0, 0, 0.12)) * lift
        direction = (tip_target - pos(arm, f"Wrist.{side}")).normalized()
        orient_rest_axes(
            arm,
            f"Wrist.{side}",
            Vector((0, -1, 0)),
            direction,
            REST["forearm." + side],
            fore,
        )
    _ = axis


def drum_idle(arm, t):
    """Standing, a slow beat (two strokes per hand in two seconds, loop)."""
    _drum_hands(arm, 2, t)


drum_idle.frames = 48


def drum_march(arm, t):
    """Walking and beating the march: one stroke per step (loop, cadence of `Walk`)."""
    _drum_hands(arm, 1, t)


def drum_beat(arm, t):
    """Standing, a quick roll (charge, melee; loop)."""
    _drum_hands(arm, 3, t)


drum_beat.frames = 24


# Busine player: the trumpet on `Prop` (mouthpiece `HORN_BACK` m behind the fist).


def _horn(arm, raise_t):
    import battle_skinned_weapons as weapons

    shoulder = pos(arm, "UpperArm.R")
    low_grip = shoulder + Vector((-0.1, -0.12, -0.5))
    low_axis = Vector((-0.08, -0.4, -1.0)).normalized()
    if raise_t > 0.0:
        rotate_about(arm, "Torso", Vector((1, 0, 0)), -0.1 * raise_t)
    head = pos(arm, "Head")
    mouth = head + Vector((-0.01, -0.13, 0.06))
    blow_axis = aim_dir(22.0, -4.0)
    blow_grip = mouth + blow_axis * weapons.HORN_BACK
    grip = low_grip.lerp(blow_grip, raise_t)
    axis = low_axis.lerp(blow_axis, raise_t).normalized()
    up = Vector((0, -1, 0)).lerp(Vector((0, 0, 1)), raise_t).normalized()
    prop = prop_matrix(grip, axis, up)
    STATE["prop"] = prop
    hand_to_prop(arm, "R", prop, 0.0, shoulder + Vector((-0.5, 0.2, -0.4)))
    fist_on_prop(arm, prop)
    if raise_t > 0.0:
        wrist_l = pos(arm, "Wrist.L")
        target = (prop @ Vector((0, 0.38, 0))).lerp(wrist_l, 1.0 - raise_t)
        support = prop.copy()
        support.translation = target
        hand_to_prop(
            arm, "L", support, 0.0, pos(arm, "UpperArm.L") + Vector((0.5, 0.1, -0.6))
        )


def horn_idle(arm, t):
    """Standing, the busine held low at the right side (loop)."""
    _horn(arm, 0.0)


def horn_walk(arm, t):
    """Walking, the busine held low (loop, cadence of `Walk`)."""
    _horn(arm, 0.0)


def horn_blow(arm, t):
    """Raise the busine to the lips, sound a call (about 2 s), lower it (loop, 3.5 s)."""
    _horn(arm, smooth(0.0, 0.2, t) * (1 - smooth(0.8, 1.0, t)))


horn_blow.frames = 84
