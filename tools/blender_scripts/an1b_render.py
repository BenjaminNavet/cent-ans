"""Lot AN1b: check renders of the new clips on the fine figures (run inside Blender).

    blender -b --factory-startup --python tools/blender_scripts/an1b_render.py -- human DIR
    blender -b --factory-startup --python tools/blender_scripts/an1b_render.py -- cavalry DIR
    uv run --project tools python tools/blender_scripts/an1b_planche.py DIR

Poses are computed exactly as the bake does (``battle_skinned.human_clip_specs``,
``battle_skinned_cavalry.clip_specs``), without baking the bone textures.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

# (figure, clip, fractions)
HUMAN = [
    ("infantry_0", "victory", (0.0, 0.25)),
    ("infantry_0", "victory_b", (0.12, 0.37)),
    ("infantry_1", "victory_pike", (0.0, 0.25)),
    ("infantry_0", "idle_look", (0.3, 0.77)),
    ("infantry_0", "idle_lean", (0.0, 0.5)),
    ("infantry_0", "idle_helm", (0.46,)),
    ("infantry_1", "pike_look", (0.3,)),
    ("archer_0", "bow_look", (0.77,)),
    ("infantry_0", "parry", (0.35,)),
    ("infantry_0", "overhead", (0.3, 0.45, 0.6)),
    ("infantry_0", "hit_b", (0.3, 0.6)),
    ("infantry_0", "hit_c", (0.25,)),
]
FRONT = ((1.6, -3.0, 1.4), (0, 0, 1.05))
CAVALRY = [
    ("c_rear", (0.0, 0.3, 0.5, 0.75)),
    ("c_stumble", (0.38, 0.46, 0.55)),
    ("c_victory", (0.0, 0.17)),
]


def human(out):
    """Workbench renders of the human clips (``battle_fine`` check step)."""
    import battle_fine as bf
    import battle_fine_figures as ff

    ff.CHECKS = [(fig, clip, fracs, *FRONT) for fig, clip, fracs in HUMAN]
    sys.argv = [sys.argv[0], "--", "check", out]
    bf.main()


def cavalry(out):
    """Eevee side views of the cavalry clips on a fine rouncey (legs in full view)."""
    import battle_fine_proto as fp
    import fg4_render as r

    os.makedirs(out, exist_ok=True)
    mount, _rider, _horse, _extra = r.build(r.LEGS_FIGURE, 0)
    r._look("alezan")
    for clip, fracs in CAVALRY:
        for k, fr in enumerate(fracs):
            fp.pose_cavalry(mount, clip, fr)
            r._shot(out, f"clip_{r.LEGS_FIGURE}_{clip}_{k}.png", r.SIDE, (640, 480))


def main():
    """Parse ``human|cavalry DIR`` after ``--``."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    what = args[0] if args else "human"
    out = args[1] if len(args) > 1 else "/tmp/an1b"
    {"human": human, "cavalry": cavalry}[what](out)
    print("OK")


if __name__ == "__main__":
    main()
