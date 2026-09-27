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
    """Right-hand javelin held as a prop.

    `cock` 1 = drawn back by the ear, 0 = arm thrown fully forward (release). Left hand stays
    on the reins (`ride_javelin_*`).
    """
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
    """Struck down (`Death`, falling backwards): the pole goes down with him.

    It topples backwards and a little to the right about its butt, and lies on the ground.
    """
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
    prop = prop_matrix(
        grip, axis, Vector((0, -1, 0)) if axis.z > 0.5 else Vector((0, 0, 1))
    )
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
    STATE["prop"] = prop_matrix(
        grip, axis, Vector((0, -1, 0)) if axis.z > 0.5 else Vector((0, 0, 1))
    )
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
        wrist = (
            hit
            - stick * 0.3
            + Vector((0.05 * sx, 0.0, 0.0))
            + Vector((0, 0.03, 0.13)) * lift
        )
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


# --- Siege engine crews (SG3) -------------------------------------------------------------


def _both_hands(arm, right, left):
    """IK both arms onto two world points, elbows out and down."""
    ik2(
        arm,
        "UpperArm.R",
        "LowerArm.R",
        "Wrist.R",
        right,
        pos(arm, "UpperArm.R") + Vector((-0.45, 0.3, -0.45)),
    )
    ik2(
        arm,
        "UpperArm.L",
        "LowerArm.L",
        "Wrist.L",
        left,
        pos(arm, "UpperArm.L") + Vector((0.45, 0.3, -0.45)),
    )


def _crouch(arm, depth):
    """Knees bent by lowering the hips `depth` metres, feet kept on the ground."""
    if depth <= 1e-4:
        return
    hips = pos(arm, "Body")
    feet = {side: pos(arm, f"Foot.{side}") for side in ("L", "R")}
    translate(arm, "Body", Vector((0, 0.06 * depth, -depth)))
    for side in ("L", "R"):
        ik2(
            arm,
            f"UpperLeg.{side}",
            f"LowerLeg.{side}",
            f"Foot.{side}",
            feet[side],
            hips + Vector((0, -1.0, -0.3)),
        )
        translate(arm, f"Foot.{side}", feet[side] - pos(arm, f"Foot.{side}"))


def crank(arm, t):
    """Winding a windlass (loop): both fists on the handle, turning before the belly."""
    phase = t * 2.0 * math.pi
    _crouch(arm, 0.08 + 0.04 * math.sin(phase))
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.32 + 0.08 * math.sin(phase))
    chest = pos(arm, "Chest")
    centre = Vector((chest.x, chest.y - 0.48, chest.z - 0.42))
    handle = centre + Vector((0, -0.2 * math.cos(phase), 0.2 * math.sin(phase)))
    _both_hands(arm, handle + Vector((-0.1, 0, 0)), handle + Vector((0.1, 0, 0)))


crank.frames = 32


def haul(arm, t):
    """Hauling on a rope hand over hand (loop), leaning back, the rope running forwards."""
    phase = t * 2.0 * math.pi
    _crouch(arm, 0.1)
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), -0.22 + 0.06 * math.sin(phase))
    chest = pos(arm, "Chest")
    reach_r = 0.5 + 0.5 * math.sin(phase)
    reach_l = 0.5 - 0.5 * math.sin(phase)
    right = chest + Vector((-0.04, -0.2 - 0.38 * reach_r, -0.22 + 0.1 * reach_r))
    left = chest + Vector((0.04, -0.2 - 0.38 * reach_l, -0.22 + 0.1 * reach_l))
    _both_hands(arm, right, left)


haul.frames = 36


def load(arm, t):
    """Loading (loop): stoops for the stone or the ball, lifts it to the chest.

    Then heaves it forwards into the sling or the muzzle and straightens up.
    """
    stoop = smooth(0.0, 0.18, t) * (1.0 - smooth(0.32, 0.55, t))
    lift = smooth(0.32, 0.55, t)
    heave = smooth(0.62, 0.76, t) * (1.0 - smooth(0.86, 1.0, t))
    _crouch(arm, 0.32 * stoop)
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.1 + 0.75 * stoop + 0.2 * heave)
    chest = pos(arm, "Chest")
    low = chest + Vector((0, -0.42, -0.6))
    held = chest + Vector((0, -0.3, -0.12))
    out = chest + Vector((0, -0.62, 0.12))
    hands = low.lerp(held, lift).lerp(out, heave)
    _both_hands(arm, hands + Vector((-0.12, 0, 0)), hands + Vector((0.12, 0, 0)))


load.frames = 48


def swab(arm, t):
    """Swabbing and ramming a bombard (loop): rammer levelled, pushed in, drawn back."""
    thrust = 0.5 - 0.5 * math.cos(t * 2.0 * math.pi)
    _crouch(arm, 0.06 + 0.06 * thrust)
    rotate_about(arm, "Torso", Vector((1, 0, 0)), 0.1 + 0.16 * thrust)
    _pike_level(arm, thrust=0.45 * thrust)


swab.frames = 40


def push(arm, t):
    """Pushing a ram or a siege tower (on the walk): leaning in, both palms on the beam."""
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.42)
    chest = pos(arm, "Chest")
    _both_hands(
        arm,
        chest + Vector((-0.2, -0.46, 0.14)),
        chest + Vector((0.2, -0.46, 0.14)),
    )


# --- Lot EP12: wounded on the ground and routers running without their arms --------------

CRAWL_STEPS = 5.0  # hand-over-hand pulls before the crawler gives up
CRAWL_STEP = 0.5  # metres gained per pull
CRAWL_REACH = 0.42  # hand planted this far ahead of the shoulder


def _ease_out(a, b, t):
    """Progress from 0 to 1 over [a, b], fast at first then slowing to a stop."""
    u = min(max((t - a) / (b - a), 0.0), 1.0)
    return 1.0 - (1.0 - u) * (1.0 - u)


def crawl(arm, t):
    """Wounded crawler (EP12, not looped): pitches onto his face, then drags himself.

    He pulls himself a few metres on his forearms, slower and slower, lifts his head a last
    time and lies still. The figure moves along its facing (-Y), so the renderer turns
    crawlers away from the fight.
    """
    fall = smooth(0.0, 0.14, t)
    steps = CRAWL_STEPS * _ease_out(0.16, 0.86, t)
    feet = (pos(arm, "Foot.L") + pos(arm, "Foot.R")) / 2
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.35 * math.sin(math.pi * fall))
    rotate_about(
        arm, "Root", Vector((1, 0, 0)), 1.5 * fall, Vector((feet.x, feet.y, 0.14))
    )
    translate(arm, "Root", Vector((0, -CRAWL_STEP * steps, 0)))
    lift_head = 0.55 * fall * (1.0 - smooth(0.86, 0.97, t))
    rotate_about(arm, "Neck", Vector((1, 0, 0)), -lift_head)
    for side, sx, offset in (("R", -1.0, 0.0), ("L", 1.0, 0.5)):
        phase = (steps + offset) % 1.0
        if phase < 0.6:
            rel, lift = -CRAWL_REACH + CRAWL_STEP * phase, 0.0
        else:
            u = (phase - 0.6) / 0.4
            rel = lerp(-CRAWL_REACH + 0.6 * CRAWL_STEP, -CRAWL_REACH, smooth(0, 1, u))
            lift = 0.12 * math.sin(math.pi * u)
        shoulder = pos(arm, f"UpperArm.{side}")
        hand = Vector((shoulder.x + 0.1 * sx, shoulder.y + rel, 0.06 + lift))
        hand = pos(arm, f"Wrist.{side}").lerp(hand, fall)
        ik2(
            arm,
            f"UpperArm.{side}",
            f"LowerArm.{side}",
            f"Wrist.{side}",
            hand,
            shoulder + Vector((0.5 * sx, 0.0, 0.6)),
        )
        # Knee drawn up now and then, the heel lifting off the ground.
        bend = 0.35 * max(0.0, math.sin(2.0 * math.pi * (steps + offset))) * fall
        rotate_about(arm, f"LowerLeg.{side}", Vector((1, 0, 0)), bend)


crawl.frames = 168  # 7 s


def wounded_sit(arm, t):
    """Wounded sitting (EP12, not looped): sinks onto his backside, one leg bent.

    He clutches his belly and rocks, propped on his other hand, then slumps onto his back
    and lies still.
    """
    down = smooth(0.0, 0.18, t)
    slump = smooth(0.8, 0.95, t)
    hips = pos(arm, "Body")
    feet = {side: pos(arm, f"Foot.{side}") for side in ("L", "R")}
    translate(arm, "Body", Vector((0, 0.25 * down, -(hips.z - 0.2) * down)))
    seat = pos(arm, "Body")
    for side, ahead, knee_up in (("L", 0.62, 0.0), ("R", 0.4, 0.2)):
        target = Vector((feet[side].x, seat.y - ahead, 0.06))
        target = feet[side].lerp(target, down)
        ik2(
            arm,
            f"UpperLeg.{side}",
            f"LowerLeg.{side}",
            f"Foot.{side}",
            target,
            seat + Vector((0, -1.0, 0.6 + knee_up)),
        )
        translate(arm, f"Foot.{side}", target - pos(arm, f"Foot.{side}"))
    rock = 0.08 * math.sin(2.0 * math.pi * 3.0 * t) * smooth(0.18, 0.3, t) * (1 - slump)
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), (0.35 + rock) * down - 1.5 * slump)
    rotate_about(arm, "Neck", Vector((1, 0, 0)), 0.3 * down)
    belly = pos(arm, "Abdomen") + Vector((0.02, -0.17, 0.06))
    right = pos(arm, "Wrist.R").lerp(belly, down * (1 - slump))
    ik2(
        arm,
        "UpperArm.R",
        "LowerArm.R",
        "Wrist.R",
        right,
        pos(arm, "UpperArm.R") + Vector((-0.5, 0.2, -0.4)),
    )
    prop_l = Vector((seat.x + 0.28, seat.y + 0.28, 0.06))
    left = pos(arm, "Wrist.L").lerp(prop_l, down * (1 - slump))
    ik2(
        arm,
        "UpperArm.L",
        "LowerArm.L",
        "Wrist.L",
        left,
        pos(arm, "UpperArm.L") + Vector((0.5, 0.4, 0.2)),
    )


wounded_sit.frames = 144  # 6 s


def wounded_kneel(arm, t):
    """Wounded on his knees (EP12, not looped): hunched, both hands on the wound, swaying.

    He then keels over onto his side and lies still.
    """
    sink = smooth(0.0, 0.15, t)
    topple = smooth(0.78, 0.92, t)
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
    sway = 0.06 * math.sin(2.0 * math.pi * 2.5 * t) * smooth(0.15, 0.25, t)
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.55 * sink + sway)
    rotate_about(arm, "Neck", Vector((1, 0, 0)), 0.35 * sink)
    belly = pos(arm, "Abdomen") + Vector((0, -0.18, 0.06))
    for side, sx in (("R", -1.0), ("L", 1.0)):
        hand = pos(arm, f"Wrist.{side}").lerp(
            belly + Vector((0.07 * sx, 0, 0)), sink * (1 - topple)
        )
        ik2(
            arm,
            f"UpperArm.{side}",
            f"LowerArm.{side}",
            f"Wrist.{side}",
            hand,
            pos(arm, f"UpperArm.{side}") + Vector((0.5 * sx, 0.2, -0.4)),
        )
    if topple > 0:
        knees = (pos(arm, "LowerLeg.L") + pos(arm, "LowerLeg.R")) / 2
        rotate_about(
            arm,
            "Root",
            Vector((0, 1, 0)),
            1.4 * topple,
            Vector((knees.x, knees.y, 0.1)),
        )


wounded_kneel.frames = 144  # 6 s


def flee(arm, t):
    """Router running for his life without weapon or shield (EP12, loop on `Run`).

    Bent forwards, elbows pumping high, he throws a glance over his shoulder.
    """
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), 0.45)
    glance = smooth(0.45, 0.6, t) * (1.0 - smooth(0.75, 0.9, t))
    rotate_about(arm, "Neck", Vector((0, 0, 1)), 1.1 * glance)
    rotate_about(arm, "Neck", Vector((1, 0, 0)), -0.35)
    for side, sx in (("L", 1.0), ("R", -1.0)):
        # Empty hands thrown wide, elbows bent: nothing held any more.
        rotate_about(arm, f"UpperArm.{side}", Vector((0, 1, 0)), -0.45 * sx)
        rotate_about(arm, f"LowerArm.{side}", Vector((1, 0, 0)), -0.6)


flee.frames = 60  # three `Run` strides (2.5 s): the glance comes once per loop


# --- Lot AN1b: victory, idle variants, parry, overhead cut, impacts, horse rear/stumble ---
#
# Loops on `Idle` (41 source frames) last 41 or 82 frames so that the source loops with the
# clip; their overrides are periodic in `t` (same value at t = 0 and t = 1). Non-looped combat
# clips fade their overrides out before the end, back to the guard.

X_AXIS = Vector((1, 0, 0))
Z_AXIS = Vector((0, 0, 1))


def _env(t, a, b, c, d):
    """Rises over [a, b], falls over [c, d] (smoothstep both ways)."""
    return smooth(a, b, t) * (1.0 - smooth(c, d, t))


def _arm_to(arm, side, target, pole_offset):
    """Two-bone IK of an arm onto `target`, elbow towards shoulder + `pole_offset`."""
    ik2(
        arm,
        f"UpperArm.{side}",
        f"LowerArm.{side}",
        f"Wrist.{side}",
        target,
        pos(arm, f"UpperArm.{side}") + pole_offset,
    )


def _held_prop(arm):
    """Right-hand prop where the wrist carries it (no override)."""
    return world(arm, "Wrist.R") @ REST["Wrist.R"].inverted() @ REST["prop"]


def _sword_at(arm, grip, axis, up, weight=1.0, left=None):
    """Right fist (and the left one `left` m towards the pommel) on a prop at `grip`.

    `weight` blends from the prop carried by the wrist (0) to the given one (1).
    """
    target = prop_matrix(grip, axis, up)
    prop = _held_prop(arm).lerp(target, weight) if weight < 0.999 else target
    STATE["prop"] = prop
    hand_to_prop(
        arm, "R", prop, 0.0, pos(arm, "UpperArm.R") + Vector((-0.5, 0.3, -0.4))
    )
    fist_on_prop(arm, prop)
    if left is not None:
        wrist_l = pos(arm, "Wrist.L")
        on_grip = prop @ Vector((0, left, 0))
        on_grip += (prop.to_3x3() @ Vector((1, 0, 0))).normalized() * 0.075
        _arm_to(arm, "L", wrist_l.lerp(on_grip, weight), Vector((0.5, 0.2, -0.5)))
    return prop


def _pump(t, beats):
    """0 -> 1 -> 0 `beats` times over the clip (periodic)."""
    return 0.5 - 0.5 * math.cos(2.0 * math.pi * beats * t)


def _cheer_left(arm, pump):
    """Left fist punched up into the air (the shield or bow goes up with it)."""
    shoulder = pos(arm, "UpperArm.L")
    _arm_to(
        arm,
        "L",
        shoulder + Vector((0.22, -0.12, 0.42 + 0.1 * pump)),
        Vector((0.7, 0.3, -0.1)),
    )


def victory(arm, t):
    """Victory (loop): weapon thrust up at arm's length twice, left fist raised, cheering."""
    pump = _pump(t, 2)
    _crouch(arm, 0.02 + 0.04 * pump)
    rotate_about(arm, "Abdomen", X_AXIS, -0.1 - 0.04 * pump)
    rotate_about(arm, "Neck", X_AXIS, -0.3)
    shoulder = pos(arm, "UpperArm.R")
    grip = shoulder + Vector((-0.1, -0.1, 0.5 + 0.14 * pump))
    _sword_at(arm, grip, Vector((-0.08, -0.25, 1.0)), Vector((0, -1, 0)))
    _cheer_left(arm, 1.0 - pump)


victory.frames = 41  # 1.7 s, one `Idle` loop


def victory_b(arm, t):
    """Victory (loop): weapon brandished overhead from side to side, left fist on the hip."""
    s = math.sin(2.0 * math.pi * 2.0 * t)
    rotate_about(arm, "Torso", Z_AXIS, 0.16 * s)
    rotate_about(arm, "Neck", X_AXIS, -0.22)
    rotate_about(arm, "Neck", Z_AXIS, 0.2 * s)
    head = pos(arm, "Head")
    grip = head + Vector((-0.18 + 0.12 * s, -0.12, 0.34 + 0.05 * abs(s)))
    _sword_at(arm, grip, Vector((0.6 * s, -0.2, 1.0)), Vector((0, -1, 0)))
    hips = pos(arm, "Body")
    _arm_to(arm, "L", hips + Vector((0.24, 0.02, 0.08)), Vector((0.6, 0.4, 0.0)))


victory_b.frames = 82  # 3.4 s, two `Idle` loops


def victory_pike(arm, t):
    """Victory with a polearm (loop): the pike raised and grounded again, left fist up."""
    pump = _pump(t, 2)
    rotate_about(arm, "Neck", X_AXIS, -0.28)
    foot = pos(arm, "Foot.R")
    butt = Vector((foot.x - 0.12, foot.y - 0.18, 0.35 * pump))
    prop = prop_matrix(
        butt + Vector((0, 0, 1.45)), Vector((0.03, -0.08, 1.0)), Vector((0, -1, 0))
    )
    STATE["prop"] = prop
    hand_to_prop(
        arm, "R", prop, 0.0, pos(arm, "UpperArm.R") + Vector((-0.4, 0.3, -0.4))
    )
    fist_on_prop(arm, prop)
    _cheer_left(arm, 1.0 - pump)


victory_pike.frames = 41


def _look_around(arm, t):
    """Head (and a little of the torso) turned left, then right, then back."""
    yaw = 0.75 * _env(t, 0.08, 0.22, 0.38, 0.52) - 0.65 * _env(t, 0.56, 0.7, 0.84, 0.97)
    rotate_about(arm, "Torso", Z_AXIS, 0.25 * yaw)
    rotate_about(arm, "Neck", Z_AXIS, 0.75 * yaw)
    rotate_about(arm, "Neck", X_AXIS, -0.08 * abs(yaw))


def idle_look(arm, t):
    """Idle variant (loop): looks around, left then right."""
    _look_around(arm, t)


idle_look.frames = 82


def pike_look(arm, t):
    """Idle variant with a pike upright (loop): looks around."""
    _look_around(arm, t)
    _pike_upright(arm)


pike_look.frames = 82


def bow_look(arm, t):
    """Idle variant with the longbow held low (loop): looks around."""
    _look_around(arm, t)
    _bow_arm(arm, 0.0)


bow_look.frames = 82


def xbow_look(arm, t):
    """Idle variant with the crossbow held low (loop): looks around."""
    _look_around(arm, t)
    _crossbow_aim(arm, 0.0)


xbow_look.frames = 82


def idle_lean(arm, t):
    """Idle variant (loop): leaning on the weapon, point on the ground, hands on the pommel."""
    sway = math.sin(2.0 * math.pi * t)
    translate(arm, "Body", Vector((0.025 * sway, 0.0, 0.0)))
    rotate_about(arm, "Abdomen", X_AXIS, 0.14)
    rotate_about(arm, "Neck", X_AXIS, 0.12 + 0.04 * sway)
    hips = pos(arm, "Body")
    grip = Vector((hips.x - 0.02, hips.y - 0.34, 0.9))
    _sword_at(arm, grip, Vector((0.0, -0.06, -1.0)), Vector((0, -1, 0)), left=-0.08)


idle_lean.frames = 82


def idle_helm(arm, t):
    """Idle variant (loop): the left hand goes up to the helmet and sets it straight."""
    up = _env(t, 0.12, 0.3, 0.62, 0.8)
    wiggle = 0.03 * math.sin(2.0 * math.pi * 5.0 * t) * _env(t, 0.3, 0.36, 0.56, 0.62)
    rotate_about(arm, "Neck", Z_AXIS, 0.15 * up)
    rotate_about(arm, "Neck", X_AXIS, 0.1 * up)
    head = pos(arm, "Head")
    helm = head + Vector((0.13, -0.03 + wiggle, 0.12 + wiggle))
    _arm_to(arm, "L", pos(arm, "Wrist.L").lerp(helm, up), Vector((0.6, 0.1, -0.2)))


idle_helm.frames = 82


def parry(arm, t):
    """Parry (melee): shield (left arm) thrown up before the face, crouched, then guard."""
    w = _env(t, 0.0, 0.2, 0.55, 0.95)
    _crouch(arm, 0.07 * w)
    rotate_about(arm, "Torso", Z_AXIS, -0.3 * w)
    rotate_about(arm, "Abdomen", X_AXIS, 0.1 * w)
    rotate_about(arm, "Neck", X_AXIS, 0.15 * w)
    chest = pos(arm, "Chest")
    block = chest + Vector((0.05, -0.36, 0.26))
    _arm_to(arm, "L", pos(arm, "Wrist.L").lerp(block, w), Vector((0.6, -0.1, -0.5)))


parry.frames = 24  # 1 s (melee cycle 1.3 s)


def overhead(arm, t):
    """Two-handed overhead cut: wound up behind the head, brought down, back to the guard."""
    raise_t = smooth(0.0, 0.34, t)
    cut = smooth(0.38, 0.56, t)
    w = raise_t * (1.0 - smooth(0.66, 1.0, t))
    rotate_about(arm, "Abdomen", X_AXIS, -0.12 * raise_t * (1 - cut) + 0.32 * cut * w)
    _crouch(arm, 0.1 * cut * w)
    head = pos(arm, "Head")
    chest = pos(arm, "Chest")
    wound = head + Vector((-0.1, 0.1, 0.22))
    struck = chest + Vector((-0.02, -0.5, -0.36))
    grip = wound.lerp(struck, cut) + Vector((0, -0.28, 0.12)) * math.sin(math.pi * cut)
    axis = Vector((0.08, 0.55, 0.8)).lerp(Vector((0.04, -0.85, -0.5)), cut)
    up = Vector((0.0, -axis.z, axis.y))
    _sword_at(arm, grip, axis.normalized(), up.normalized(), weight=w, left=-0.1)


overhead.frames = 28  # 1.17 s


def hit_stagger(arm, t):
    """Impact variant: rocked back a step, head snapped back, arms flung, then guard."""
    w = _env(t, 0.0, 0.14, 0.45, 1.0)
    translate(arm, "Root", Vector((0.0, 0.16 * w, 0.0)))
    rotate_about(arm, "Abdomen", X_AXIS, -0.3 * w)
    rotate_about(arm, "Torso", Z_AXIS, 0.22 * w)
    rotate_about(arm, "Neck", X_AXIS, -0.35 * w)
    for side, sx in (("L", 1.0), ("R", -1.0)):
        rotate_about(arm, f"UpperArm.{side}", Vector((0, 1, 0)), -0.35 * sx * w)


hit_stagger.frames = 20


# Horse: the cavalry baker runs `pose.horse(harm, t)` before seating the rider. The hooves
# hang from the root (IK bones of the Quaternius horse): once the legs have moved they are
# carried along with their lower leg.

HORSE_LEGS = {
    "FL": (("FrontUpperLeg.L", "FrontLowerLeg.L"), "IKFrontLeg.L"),
    "FR": (("FrontUpperLeg.R", "FrontLowerLeg.R"), "IKFrontLeg.R"),
    "BL": (("BackLeg.L", "BackUpperLeg.L", "BackLowerLeg.L"), "IKBackLeg.L"),
    "BR": (("BackLeg.R", "BackUpperLeg.R", "BackLowerLeg.R"), "IKBackLeg.R"),
}


def _horse_capture(harm):
    """World matrices of each lower leg and hoof before the overrides."""
    return {
        k: (world(harm, chain[-1]).copy(), world(harm, hoof).copy())
        for k, (chain, hoof) in HORSE_LEGS.items()
    }


def _hoof_at(harm, key, before):
    """Where the hoof of leg `key` is carried by its lower leg now."""
    lower_b, hoof_b = before[key]
    chain = HORSE_LEGS[key][0]
    return world(harm, chain[-1]) @ lower_b.inverted() @ hoof_b


def _horse_feet(harm, before):
    """Hooves moved with their lower legs."""
    for key, (_chain, hoof) in HORSE_LEGS.items():
        set_world(harm, hoof, _hoof_at(harm, key, before))


def _plant(harm, key, before, target, iterations=10):
    """Leg `key` bent (sagittal CCD) so that its hoof reaches `target`."""
    chain = HORSE_LEGS[key][0]
    for _ in range(iterations):
        for bone in reversed(chain):
            end = _hoof_at(harm, key, before).to_translation()
            pivot = pos(harm, bone)
            a = end - pivot
            b = target - pivot
            angle = math.atan2(b.z, b.y) - math.atan2(a.z, a.y)
            angle = (angle + math.pi) % (2.0 * math.pi) - math.pi
            rotate_about(harm, bone, X_AXIS, max(-0.4, min(0.4, angle)), pivot)


def _rear_env(t):
    return _env(t, 0.0, 0.34, 0.64, 0.97)


def horse_rear(harm, t):
    """Horse rearing (pikes in its face): up on the hind legs, forelegs pawing the air."""
    up = _rear_env(t)
    if up < 1e-3:
        return
    before = _horse_capture(harm)
    planted = {k: before[k][1].to_translation() for k in ("BL", "BR")}
    hip = (pos(harm, "BackLeg.L") + pos(harm, "BackLeg.R")) / 2
    rotate_about(harm, "Body", X_AXIS, -0.82 * up, hip)
    translate(harm, "Body", Vector((0.0, 0.06, -0.16)) * up)
    for key in ("BL", "BR"):
        _plant(harm, key, before, planted[key])
    for side, phase in (("L", 0.0), ("R", 0.5)):
        paw = math.sin(2.0 * math.pi * (2.5 * t + phase))
        rotate_about(harm, f"FrontUpperLeg.{side}", X_AXIS, (-0.7 - 0.25 * paw) * up)
        rotate_about(harm, f"FrontLowerLeg.{side}", X_AXIS, (1.4 + 0.3 * paw) * up)
    rotate_about(harm, "Neck1", X_AXIS, 0.4 * up)
    rotate_about(harm, "Head", X_AXIS, 0.25 * up)
    _horse_feet(harm, before)


def _stumble_env(t):
    return _env(t, 0.34, 0.42, 0.5, 0.66)


def horse_stumble(harm, t):
    """Horse stumbling at the gallop: forelegs buckle, nose down, then it recovers."""
    s = _stumble_env(t)
    if s < 1e-3:
        return
    before = _horse_capture(harm)
    hip = (pos(harm, "BackLeg.L") + pos(harm, "BackLeg.R")) / 2
    rotate_about(harm, "Body", X_AXIS, 0.22 * s, hip)
    translate(harm, "Body", Vector((0.0, -0.08, -0.12)) * s)
    for side, lag in (("L", 0.0), ("R", 0.25)):
        k = s * (1.0 - lag * (1.0 - s))
        rotate_about(harm, f"FrontUpperLeg.{side}", X_AXIS, -0.45 * k)
        rotate_about(harm, f"FrontLowerLeg.{side}", X_AXIS, 1.7 * k)
    rotate_about(harm, "Neck1", X_AXIS, 0.55 * s)
    _horse_feet(harm, before)


def ride_rear(arm, t):
    """Rider of a rearing horse: thrown forwards onto the neck, reins short, lance up."""
    up = _rear_env(t)
    m = _mount()
    _ride_legs(arm)
    _lean(arm, 0.75 * up)
    _reins(arm)
    butt = _hp(m.stirrups["R"] + Vector((-0.08, -0.05, 0.2)))
    axis = _hv(Vector((-0.05, -0.18, 1.0))).lerp(
        Vector((-0.05, -0.35, 1.0)).normalized(), up
    )
    axis.normalize()
    _lance_at(arm, butt + axis * 1.1, axis, _hv(Vector((0, -1, 0))))


ride_rear.horse = horse_rear
ride_rear.frames = 32  # 1.33 s: fits the cavalry melee cycle (1.4 s)


def ride_stumble(arm, t):
    """Rider of a stumbling horse: lance couched, pitched forwards, back in the saddle."""
    s = _stumble_env(t)
    _ride_legs(arm)
    _lean(arm, 0.35 + 0.35 * s)
    _reins(arm)
    grip = pos(arm, "Chest") + _hv(Vector((-0.2, -0.18, -0.2 - 0.1 * s)))
    _lance_at(arm, grip, _hv(Vector((0.22, -1.0, -0.04 - 0.3 * s))))


ride_stumble.horse = horse_stumble
ride_stumble.frames = 90  # six `Gallop` strides (3.75 s)


def ride_victory(arm, t):
    """Mounted victory (loop): lance or javelin raised high and shaken, reins in the left."""
    pump = _pump(t, 3)
    _ride_legs(arm)
    _lean(arm, -0.06)
    _reins(arm)
    rotate_about(arm, "Neck", _hv(X_AXIS), -0.25)
    grip = pos(arm, "UpperArm.R") + _hv(Vector((-0.1, -0.08, 0.42 + 0.14 * pump)))
    _lance_at(arm, grip, _hv(Vector((-0.1, -0.3, 1.0))), _hv(Vector((0, -1, 0))))
