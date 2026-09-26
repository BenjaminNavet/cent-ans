"""Lot FG1: fine battle figures (MakeHuman body) for the skinned battle rendering.

Run from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- <step> [options]

Steps (after ``--``):
    rigs                    bake ``human`` and ``cavalry`` bone textures with the realistic
                            proportions of ``battle_fine_rig`` (same bones, same clips)
    figures [--only a,b]    build the fine figures of ``battle_fine_figures`` (3 LODs each)
    check <dir>             render the computed poses (bow, crossbow, pike, riders) on the
                            fitted body, to check hands on weapons, arrow at the cheek, feet
                            in the stirrups
    all                     rigs then figures

Output: ``game/assets/models/battle_fine/`` (``CAM1`` meshes, ``CAB1`` bone textures and a
``manifest.json`` of the same shape as ``battle_skinned/manifest.json``), read by
``BattleSkinned`` when the game runs with ``--fine-figures`` after ``--``. The Quaternius
pipeline (``battle_skinned.py``, ``battle_skinned/``) is untouched.
"""

import json
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_fine_rig as fine_rig  # noqa: E402
import battle_skinned as bs  # noqa: E402

FINE_DIR = os.path.join(bs.ROOT, "game", "assets", "models", "battle_fine")


# --- Rigs ---------------------------------------------------------------------------------


def load_fine_human(keep_meshes=False):
    """Fresh scene with the Quaternius ``human`` armature at bind pose, fine proportions."""
    bs.reset_scene()
    arm, meshes, roots = bs.import_glb(os.path.join(bs.CHARS, "adventurer.glb"))
    for r in roots:
        r.scale = (bs.HUMAN_SCALE,) * 3
    bs.rest_pose(arm)
    if not keep_meshes:
        for m in meshes:
            bpy.data.objects.remove(m)
        meshes = []
    bpy.context.view_layer.update()
    fine_rig.apply_proportions(arm)
    bs.rest_pose(arm)
    return arm, meshes


def fine_mount_class():
    """``battle_skinned_cavalry.Mount`` whose rider has the fine proportions."""
    import battle_skinned_cavalry as cav

    base = getattr(cav, "_QuaterniusMount", cav.Mount)

    class FineMount(base):
        """Mount with the rider's rest joints moved by ``battle_fine_rig``."""

        def __init__(self):
            """Build the Quaternius mount, then move the rider's joints."""
            super().__init__()
            fine_rig.apply_proportions(self.rarm)
            bs.rest_pose(self.rarm)

    cav._QuaterniusMount = base
    return FineMount


def use_fine_mount():
    """Make ``battle_skinned_cavalry`` build fine-proportioned riders from now on."""
    import battle_skinned_cavalry as cav

    cav.Mount = fine_mount_class()


def bake_human_rig():
    """``human`` bone texture on the fine rig (same clips as ``battle_skinned``)."""
    arm, _meshes = load_fine_human()
    print("PROPORTIONS arm", fine_rig.segment_lengths(arm, ARM_CHAIN))
    rig = bs.Rig("human")
    for b in bs.HUMAN_BONES:
        rig.add(arm, b, b)
    bs.add_human_virtuals(rig, arm)
    rig.capture_rest()
    for clip, source, loop, overrides, mirror in bs.human_clip_specs():
        start = rig.begin_clip()
        act = bs.find_action(source, "CharacterArmature")
        frames = getattr(overrides, "frames", None) if overrides is not None else None
        bs.sample_action(
            rig, arm, act, overrides=overrides, mirror=mirror, frames=frames
        )
        rig.end_clip(clip, start, loop)
    rig.write()
    return rig


def bake_cavalry_rig():
    """``cavalry`` bone texture: Quaternius horse, fine-proportioned rider."""
    import battle_skinned_cavalry as cav

    use_fine_mount()
    return cav.bake_cavalry_rig()


ARM_CHAIN = ["UpperArm.L", "LowerArm.L", "Wrist.L"]


def step_rigs(manifest):
    """Bake both bone textures into ``FINE_DIR`` and record them in `manifest`."""
    rigs = {"human": bake_human_rig(), "cavalry": bake_cavalry_rig()}
    for name, rig in rigs.items():
        manifest["rigs"][name] = rig.manifest()
    return rigs


# --- Main ---------------------------------------------------------------------------------


def load_manifest():
    """Current fine manifest (empty skeleton when absent)."""
    path = os.path.join(FINE_DIR, "manifest.json")
    if os.path.exists(path):
        with open(path) as f:
            return json.load(f)
    return {"rigs": {}, "figures": {}}


def save_manifest(manifest):
    """Write the fine manifest."""
    manifest["source"] = (
        "tools/blender_scripts/battle_fine.py (MakeHuman CC0 + Quaternius CC0, see SOURCE.md)"
    )
    with open(os.path.join(FINE_DIR, "manifest.json"), "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)


def main():
    """Parse the step and run it."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    step = args[0] if args else "all"
    os.makedirs(FINE_DIR, exist_ok=True)
    bs.OUT_DIR = FINE_DIR  # Rig.write() reads the module global
    manifest = load_manifest()
    only = None
    if "--only" in args:
        only = set(args[args.index("--only") + 1].split(","))
    if step in ("rigs", "all"):
        step_rigs(manifest)
        save_manifest(manifest)
    if step in ("figures", "all"):
        import battle_fine_figures as ff

        ff.build_all(manifest, only)
        save_manifest(manifest)
    if step == "check":
        import battle_fine_figures as ff

        ff.check_poses(args[1] if len(args) > 1 else "/tmp/fg1_check")
    print("OK")


if __name__ == "__main__":
    main()
