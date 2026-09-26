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
# the trunk scale of the fit (see docs/wip/fg1-corps.md, « Proportions »).
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


def segment_lengths(arm, bones):
    """Joint-to-joint lengths (metres, rest) of consecutive `bones` pairs, for the logs."""
    mw = arm.matrix_world
    heads = [mw @ arm.data.bones[b].head_local for b in bones]
    return [round((b - a).length, 3) for a, b in zip(heads, heads[1:], strict=False)]
