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
    STATE["prop"] = None
    STATE["nock"] = None
    STATE["arrow"] = False


def smooth(a, b, t):
    x = min(max((t - a) / (b - a), 0.0), 1.0) if b != a else float(t >= a)
    return x * x * (3 - 2 * x)


def lerp(a, b, t):
    return a + (b - a) * t


# --- World-space bone helpers -----------------------------------------------------------


def world(arm, bone):
    return arm.matrix_world @ arm.pose.bones[bone].matrix


def set_world(arm, bone, m):
    arm.pose.bones[bone].matrix = arm.matrix_world.inverted() @ m
    bpy.context.view_layer.update()


def pos(arm, bone):
    return world(arm, bone).to_translation()


# Child whose head gives the direction of a limb (the imported bones' own axes are not
# reliably along the limbs).
LIMB_CHILD = {
    "UpperArm.L": "LowerArm.L", "LowerArm.L": "Wrist.L",
    "UpperArm.R": "LowerArm.R", "LowerArm.R": "Wrist.R",
    "UpperLeg.L": "LowerLeg.L", "UpperLeg.R": "LowerLeg.R",
}


def limb_dir(arm, bone):
    """Current direction of a limb: towards its child's head, else the forearm's for a
    wrist (the hand extends the forearm at rest), else the bone's own axis."""
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
    """World rotation of `bone` mapping two world directions of its rest pose (`rest_a`,
    `rest_b`) onto `target_a` (exact) and `target_b` (as close as possible)."""
    r = _frame(target_a, target_b) @ _frame(rest_a, rest_b).inverted()
    rest_rot = REST[bone].to_3x3().normalized()
    orient_like(arm, bone, (r @ rest_rot).to_quaternion())


def bow_upright(arm, forward):
    """Left fist turned so that the bow limbs stand vertical, the arrow along `forward`."""
    arm_dir = REST["forearm.L"]
    orient_rest_axes(arm, "Wrist.L", arm_dir, forward, Vector((0, -1, 0)), Vector((0, 0, 1)))


def hand_to_prop(arm, side, prop, along, pole):
    """IK the arm so that the fist sits on the prop at `along` metres on its axis."""
    target = prop @ Vector((0, along, 0))
    wrist_target = target - (prop.to_3x3() @ Vector((1, 0, 0))).normalized() * 0.075 * (1 if side == "R" else -1)
    ik2(arm, f"UpperArm.{side}", f"LowerArm.{side}", f"Wrist.{side}", wrist_target, pole)


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
    prop = prop_matrix(butt + Vector((0, 0, 1.25)), Vector((0.03, -0.05, 1.0)), Vector((0, -1, 0)))
    STATE["prop"] = prop
    hand_to_prop(arm, "R", prop, 0.0, pos(arm, "UpperArm.R") + Vector((-0.4, 0.3, -0.4)))
    fist_on_prop(arm, prop)


def _pike_level(arm, thrust=0.0):
    """Pike levelled forwards at the waist, both hands, pushed forwards by `thrust` m."""
    chest = pos(arm, "Chest")
    grip = Vector((chest.x - 0.16, chest.y + 0.12 - thrust, chest.z - 0.28 + 0.04 * thrust))
    prop = prop_matrix(grip, Vector((0.08, -1.0, 0.06)), Vector((0, 0, 1)))
    STATE["prop"] = prop
    hand_to_prop(arm, "R", prop, 0.0, grip + Vector((-0.5, 0.4, -0.3)))
    fist_on_prop(arm, prop)
    hand_to_prop(arm, "L", prop, 0.55, grip + Vector((0.5, 0.1, -0.4)))


def pike_hold(arm, t):
    _pike_upright(arm)


def pike_level(arm, t):
    _pike_level(arm)


def pike_thrust(arm, t):
    push = smooth(0.15, 0.4, t) * (1 - smooth(0.55, 0.95, t))
    rotate_about(arm, "Torso", Vector((1, 0, 0)), -0.18 * push)
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
    ik2(arm, "UpperArm.L", "LowerArm.L", "Wrist.L", target, shoulder + Vector((0.4, 0.2, -0.6)))
    # Bow vertical, arrow along the forearm (canted 10 degrees like English archers).
    fore = (pos(arm, "Wrist.L") - pos(arm, "LowerArm.L")).normalized()
    bow_upright(arm, fore)
    rotate_about(arm, "Wrist.L", fore, math.radians(-10))
    return aim_v


def _draw_hand(arm, draw, aim_v):
    grip = pos(arm, "Wrist.L") + (pos(arm, "Wrist.L") - pos(arm, "LowerArm.L")).normalized() * 0.075
    nock = grip - aim_v * (0.16 + (DRAW_LENGTH - 0.16) * draw)
    right = Vector((-1, 0, 0))
    hand = nock + right * 0.02
    ik2(arm, "UpperArm.R", "LowerArm.R", "Wrist.R", hand - aim_v * 0.02, pos(arm, "UpperArm.R") + Vector((-0.5, 0.5, 0.1)))
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
        hand = pos(arm, "UpperArm.R") + Vector((-0.12, 0.25, -0.1)) * back + Vector((0.0, -0.1, -0.45)) * (1 - back)
        ik2(arm, "UpperArm.R", "LowerArm.R", "Wrist.R", hand, pos(arm, "UpperArm.R") + Vector((-0.5, 0.5, -0.2)))


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
    hand_to_prop(arm, "L", prop, 0.32, pos(arm, "UpperArm.L") + Vector((0.4, 0.1, -0.6)))


def crossbow_rest(arm, t):
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
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), -0.75 * bend)
    feet = (pos(arm, "Foot.L") + pos(arm, "Foot.R")) / 2
    nose = Vector((feet.x, feet.y - 0.42, 0.05))
    up = Vector((0.0, 0.25, 1.0)).normalized()
    upright = nose + up * 0.7
    aimed_grip = Vector((pos(arm, "UpperArm.R").x + 0.06, pos(arm, "Head").y - 0.22, pos(arm, "Head").z - 0.06))
    grip = upright.lerp(aimed_grip, 1 - lower)
    axis = (-up).lerp(aim_dir(6.0, -4.0), 1 - lower)
    prop = prop_matrix(grip, axis, Vector((0, -1, 0)))
    STATE["prop"] = prop
    along = lerp(0.3, 0.05, pull * bend) if bend > 0 else 0.0
    hand_to_prop(arm, "R", prop, along - 0.05, pos(arm, "UpperArm.R") + Vector((-0.5, -0.2, 0.0)))
    hand_to_prop(arm, "L", prop, along + 0.02, pos(arm, "UpperArm.L") + Vector((0.5, -0.2, 0.0)))


crossbow_shoot.frames = 120  # 5 s: release at 0.35 s, spanning, back on aim at 4.4 s


# --- Deaths and knock-downs ---------------------------------------------------------------


def death_knees(arm, t):
    """Knees give way, then the body pitches forwards onto the face."""
    sink = smooth(0.0, 0.35, t)
    fall = smooth(0.35, 0.8, t)
    hips = pos(arm, "Body")
    translate(arm, "Body", Vector((0, 0.05 * sink, -0.42 * sink)))
    for side in ("L", "R"):
        foot = pos(arm, f"Foot.{side}")
        ik2(arm, f"UpperLeg.{side}", f"LowerLeg.{side}", f"Foot.{side}", foot, hips + Vector((0, -1.0, -0.3)))
    rotate_about(arm, "Abdomen", Vector((1, 0, 0)), -0.4 * sink)
    arms_down = Vector((0, -0.2, -1.0))
    for side in ("L", "R"):
        aim(arm, f"UpperArm.{side}", arms_down + Vector((0.3 if side == "L" else -0.3, 0, 0)))
    if fall > 0:
        knees = (pos(arm, "LowerLeg.L") + pos(arm, "LowerLeg.R")) / 2
        rotate_about(arm, "Root", Vector((1, 0, 0)), -1.45 * fall, Vector((knees.x, knees.y, 0.08)))


death_knees.frames = 30


def death_back(arm, t):
    """Thrown backwards (charge impact): lifted, flung back two metres, flat on the back."""
    fly = smooth(0.0, 0.55, t)
    hop = math.sin(min(t / 0.55, 1.0) * math.pi) * 0.35
    translate(arm, "Root", Vector((0, 1.8 * fly, hop)))
    rotate_about(arm, "Root", Vector((1, 0, 0)), 1.5 * smooth(0.05, 0.6, t), pos(arm, "Root"))
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
