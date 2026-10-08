"""Lot FG1: realistic proportions of the Quaternius ``human`` rig for the fine figures.

The Quaternius men are stylised: short arms (18 cm humerus), a short trunk under a big
head. The fine figures (MakeHuman body, lot FG0) need joints where a real man has them, so
the rig's *rest* joints are moved (same bones, same names, same clips): each bone's head is
placed at ``parent head + (old offset from the parent) * factor + extra``, children follow.

Bone orientations are kept, so the Quaternius actions (rotation keys only on these bones;
translation keys exist only on ``Body``, the feet and the IK pole targets, which are left
alone) play unchanged on the new rest pose. The legs are not touched: the feet are children
of ``Root`` with keyed translations and must stay where the legs reach.

One table serves the foot soldiers (``human`` rig) and the riders (``R:`` bones of the
``cavalry`` rig); the horse bones are not touched (lot FG4).
"""

import bpy
from mathutils import Vector

# bone -> (factor on the offset from its parent's head, extra offset in metres, world
# axes of the bind pose, left side; the right side mirrors X). Unlisted bones follow their
# parent. Measured on the rig at bind pose (HUMAN_SCALE 0.97) against the MakeHuman body at
# the trunk scale of the fit (see docs/archive/chantiers.md, « Proportions »).
PROPORTIONS = {
    # Trunk: hips-to-neck 0.60 -> 0.63 m, so that the MakeHuman trunk is fitted at ~0.95
    # (1.70 m tall) instead of 0.90 (1.63 m, head disproportionate to the arms).
    "Abdomen": (1.05, (0.0, 0.0, 0.0)),
    "Torso": (1.05, (0.0, 0.0, 0.0)),
    "Chest": (1.05, (0.0, 0.0, 0.0)),
    "Neck": (1.05, (0.0, 0.0, 0.0)),
    # Head pivot at the base of the skull (MakeHuman neck 0.10 m, Quaternius 0.074 m).
    "Head": (1.3, (0.0, 0.0, 0.0)),
    # Shoulder joints wider and lower (the Quaternius shoulders hunch under the head).
    "UpperArm.L": (1.0, (0.03, 0.0, -0.03)),
    # Humerus 0.176 -> 0.25 m, forearm 0.228 -> 0.255 m (elbow to wrist joint).
    "LowerArm.L": (1.42, (0.0, 0.0, 0.0)),
    "Wrist.L": (1.12, (0.0, 0.0, 0.0)),
}


def _entry(name):
    """Table entry of a bone (right side mirrored), or None."""
    if name in PROPORTIONS:
        return PROPORTIONS[name]
    if name.endswith(".R"):
        left = PROPORTIONS.get(name[:-2] + ".L")
        if left is not None:
            factor, (x, y, z) = left
            return factor, (-x, y, z)
    return None


def target_heads(arm):
    """World positions of every bone head of `arm` at rest after the proportion edits."""
    mw = arm.matrix_world
    old = {b.name: mw @ b.head_local for b in arm.data.bones}
    new = {}
    for bone in _hierarchy(arm):
        if bone.parent is None:
            new[bone.name] = old[bone.name]
            continue
        p = bone.parent.name
        offset = old[bone.name] - old[p]
        entry = _entry(bone.name)
        if entry is not None:
            factor, extra = entry
            offset = offset * factor + Vector(extra)
        new[bone.name] = new[p] + offset
    return new


def _hierarchy(arm):
    """Bones of `arm`, parents before children."""
    out = []

    def visit(b):
        out.append(b)
        for c in b.children:
            visit(c)

    for b in arm.data.bones:
        if b.parent is None:
            visit(b)
    return out


def apply_proportions(arm):
    """Move the rest joints of `arm` (edit mode); orientations and lengths are kept."""
    heads = target_heads(arm)
    inv = arm.matrix_world.inverted()
    view_layer = bpy.context.view_layer
    previous = view_layer.objects.active
    view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    for eb in arm.data.edit_bones:
        delta = inv @ heads[eb.name] - eb.head
        eb.head += delta
        eb.tail += delta
    bpy.ops.object.mode_set(mode="OBJECT")
    view_layer.objects.active = previous
    view_layer.update()


# --- Computed poses on the fine rig -------------------------------------------------------

# Longbow anchor: corner of the mouth on the right side, from the head joint (base of the
# skull), in the figure's frame before the torso turns side-on (figures face -Y, right = -X).
BOW_ANCHOR = Vector((-0.045, -0.09, 0.0))
# Draw length from the bow hand to the anchor (a real war bow draws ~0.72-0.76 m; the
# rig's bow arm reaches ~0.58 m from the shoulder, which caps it).
BOW_DRAW = 0.68
BOW_TURN = -75.0  # torso turned side-on at full draw (degrees about Z; FG1: -60)
CLAVICLE_REACH = 20.0  # FG2: bow shoulder swung towards the aim at full draw (degrees)


def fine_bow_arm(arm, raise_t):
    """Bow arm from hanging (0) to aimed (1), the arrow line through the cheek anchor.

    Replaces ``battle_skinned_poses._bow_arm`` for the fine rig: the Quaternius version
    aims the bow hand from the left shoulder, which puts the nock at the chest; with real
    arm lengths the hand is placed on the line anchor + aim * draw instead, so that the full
    draw brings the nock (``_draw_hand``) and the right hand to the cheek.
    """
    import math

    import battle_skinned_poses as poses
    from mathutils import Matrix

    turn = math.radians(BOW_TURN) * raise_t
    poses.rotate_about(arm, "Torso", Vector((0, 0, 1)), turn)
    aim_v = poses.aim_dir(poses.BOW_ELEVATION * raise_t, -8.0)
    if CLAVICLE_REACH and raise_t > 0.0:
        # FG2: the bow shoulder pushed towards the target (clavicle swung along the aim),
        # as archers do to lengthen the draw.
        root = poses.pos(arm, "Shoulder.L")
        tip = poses.pos(arm, "UpperArm.L")
        axis = (tip - root).cross(aim_v)
        if axis.length > 1e-6:
            angle = math.radians(CLAVICLE_REACH) * raise_t
            poses.rotate_about(arm, "Shoulder.L", axis.normalized(), angle, root)
    shoulder = poses.pos(arm, "UpperArm.L")
    rest_target = shoulder + Vector((0.08, -0.38, -0.42))
    anchor = poses.pos(arm, "Head") + Matrix.Rotation(turn, 3, "Z") @ BOW_ANCHOR
    grip = anchor + aim_v * BOW_DRAW
    target = rest_target.lerp(grip - aim_v * 0.075, raise_t)
    poses.ik2(
        arm,
        "UpperArm.L",
        "LowerArm.L",
        "Wrist.L",
        target,
        shoulder + Vector((0.4, 0.2, -0.6)),
    )
    fore = (poses.pos(arm, "Wrist.L") - poses.pos(arm, "LowerArm.L")).normalized()
    poses.bow_upright(arm, fore)
    poses.rotate_about(arm, "Wrist.L", fore, math.radians(-10))
    if raise_t > 0.0:
        # Where the bow hand actually got to (the arm may fall short of the full draw):
        # the draw ends at the anchor, whatever the reach.
        reached = poses.pos(arm, "Wrist.L") + fore * 0.075
        poses.DRAW_LENGTH = max(0.4, (reached - anchor).dot(aim_v))
    return aim_v


def use_fine_poses():
    """Swap the pose helpers that depend on the rig's proportions (fine bakes only)."""
    import battle_skinned_poses as poses

    poses._bow_arm = fine_bow_arm
    poses.DRAW_LENGTH = BOW_DRAW


def segment_lengths(arm, bones):
    """Joint-to-joint lengths (metres, rest) of consecutive `bones` pairs, for the logs."""
    mw = arm.matrix_world
    heads = [mw @ arm.data.bones[b].head_local for b in bones]
    return [round((b - a).length, 3) for a, b in zip(heads, heads[1:], strict=False)]
