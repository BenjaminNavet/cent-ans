"""Lot AS8b probe: horse geometry and action lengths (blender -b --factory-startup --python ...)."""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned as bs  # noqa: E402
import battle_skinned_cavalry as cav  # noqa: E402
import battle_skinned_poses as poses  # noqa: E402
import bpy  # noqa: E402

mount = cav.Mount()
harm = mount.harm
for name in ("Walk", "Gallop", "Idle"):
    act = bs.find_action(name, "AnimalArmature")
    print("ACTION", name, act.frame_range[:], bpy.context.scene.render.fps)
for key, (chain, hoof) in poses.HORSE_LEGS.items():
    print(
        "LEG",
        key,
        "root",
        tuple(round(v, 3) for v in poses.pos(harm, chain[0])),
        "hoof",
        tuple(round(v, 3) for v in poses.pos(harm, hoof)),
        "lower",
        tuple(round(v, 3) for v in poses.pos(harm, chain[-1])),
    )
print("BODY", tuple(round(v, 3) for v in poses.pos(harm, "Body")))
print("PARENT", harm.pose.bones["FrontUpperLeg.L"].parent.name, harm.pose.bones["BackLeg.L"].parent.name)
