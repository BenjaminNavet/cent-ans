"""Lot FG0: generate the MakeHuman base body used by the finer battle figures.

Requires the MPFB 2 extension (https://extensions.blender.org/add-ons/mpfb/, GPL-3.0 code,
**not** redistributed here) installed in Blender's user extensions, e.g.:

    blender --command extension install-file -r user_default -e add-on-mpfb-v2.0.17.zip

Run from the repository root (without --factory-startup, so the extension loads):

    blender --background --python tools/blender_scripts/fg_makehuman_base.py

Output: ``game/assets/third_party/characters/makehuman_base/fg_base_male.blend`` holding the
body mesh (targets baked, helpers removed except the eyeballs) and MPFB's ``game_engine``
armature with its weights. The MakeHuman base mesh, targets and rig weights are CC0 1.0
(explicit release by the MakeHuman team, 2020); the generated output is CC0 too. The battle
pipeline (``battle_fine_proto.py``) only reads this .blend: MPFB is needed only to
regenerate it.
"""

import os

import addon_utils
import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
OUT_DIR = os.path.join(
    ROOT, "game", "assets", "third_party", "characters", "makehuman_base"
)
OUT = os.path.join(OUT_DIR, "fg_base_male.blend")

# A weathered young adult man-at-arms: male, ~25 years, muscular, slightly heavy, tall.
MACRO = {
    "gender": 1.0,
    "age": 0.5,
    "muscle": 0.68,
    "weight": 0.55,
    "proportions": 0.65,
    "height": 0.55,
    "cupsize": 0.5,
    "firmness": 0.5,
    "race": {"asian": 0.0, "caucasian": 0.9, "african": 0.1},
}

# Face and body details (target file name without extension -> weight), kept mild.
DETAILS = {
    "nose-scale-vert-incr": 0.25,
    "nose-hump-incr": 0.35,
    "chin-prominent-incr": 0.3,
    "chin-width-incr": 0.25,
    "l-cheek-bones-incr": 0.3,
    "r-cheek-bones-incr": 0.3,
    "forehead-temple-decr": 0.2,
    "neck-scale-horiz-incr": 0.35,
}

KEEP_HELPERS = ("helper-l-eye", "helper-r-eye")

# Lot FG1: face variants, stored as shape keys ``face_<k>`` on the body (full body shape;
# the battle pipeline only takes the head from them). Each face = (age macro, detail
# targets added to ``DETAILS``). Age 0.5 = 25 years, 0.6875 = 45, 0.75 = 55 (MakeHuman).
FACES = [
    # 0: the FG0 man-at-arms (base), 25 years.
    (0.5, {}),
    # 1: veteran, ~45, heavy brow, broken hooked nose, drawn cheeks.
    (
        0.69,
        {
            "nose-hump-incr": 0.6,
            "nose-point-down": 0.3,
            "l-cheek-volume-decr": 0.4,
            "r-cheek-volume-decr": 0.4,
            "mouth-angles-down": 0.4,
            "eyebrows-trans-down": 0.4,
            "head-age-incr": 0.4,
        },
    ),
    # 2: youth, ~18, round face, short upturned nose.
    (
        0.43,
        {
            "head-round": 0.4,
            "nose-point-up": 0.35,
            "nose-scale-vert-decr": 0.3,
            "chin-prominent-decr": 0.3,
            "l-cheek-volume-incr": 0.3,
            "r-cheek-volume-incr": 0.3,
        },
    ),
    # 3: broad square jaw, wide nose.
    (
        0.55,
        {
            "head-square": 0.6,
            "chin-width-incr": 0.6,
            "chin-bones-incr": 0.5,
            "nose-width2-incr": 0.4,
            "nose-flaring-incr": 0.3,
            "neck-scale-horiz-incr": 0.3,
        },
    ),
    # 4: long gaunt face, long straight nose.
    (
        0.58,
        {
            "head-oval": 0.6,
            "head-scale-vert-incr": 0.25,
            "l-cheek-volume-decr": 0.6,
            "r-cheek-volume-decr": 0.6,
            "nose-scale-vert-incr": 0.4,
            "nose-greek-incr": 0.4,
            "chin-height-incr": 0.3,
        },
    ),
    # 5: brawler, flattened broad nose, low brow.
    (
        0.6,
        {
            "nose-compression-compress": 0.5,
            "nose-width1-incr": 0.5,
            "nose-width2-incr": 0.5,
            "nose-curve-concave": 0.4,
            "eyebrows-trans-down": 0.5,
            "head-scale-horiz-incr": 0.2,
            "mouth-scale-horiz-incr": 0.3,
        },
    ),
    # 6: aquiline, jutting chin, narrow head.
    (
        0.52,
        {
            "nose-hump-incr": 0.9,
            "nose-point-down": 0.5,
            "nose-scale-depth-incr": 0.4,
            "chin-prognathism-incr": 0.4,
            "head-triangular": 0.4,
            "head-scale-horiz-decr": 0.2,
        },
    ),
    # 7: old soldier, ~55, fleshy nose, heavy jowls.
    (
        0.76,
        {
            "nose-volume-incr": 0.5,
            "head-fat-incr": 0.4,
            "mouth-laugh-lines-in": 0.5,
            "mouth-angles-down": 0.3,
            "head-age-incr": 0.6,
        },
    ),
]


def mpfb_module():
    """Import the MPFB 2 extension package (enabled in the user preferences)."""
    for mod in addon_utils.modules():
        if mod.__name__.endswith(".mpfb") or mod.__name__ == "mpfb":
            addon_utils.enable(mod.__name__, default_set=True)
            return __import__(mod.__name__, fromlist=["services"])
    raise RuntimeError("MPFB 2 extension not installed (see module docstring)")


def apply_details(basemesh, target_service):
    """Load the detail targets at their weights."""
    for name, weight in DETAILS.items():
        path = target_service.target_full_path(name)
        if path is None:
            print(f"WARN target not found: {name}")
            continue
        target_service.load_target(basemesh, path, weight=weight, name=name)


def strip_helpers(basemesh):
    """Delete helper and joint geometry (keep the body and the eyeballs)."""
    me = basemesh.data
    keep = set()
    for vg_name in ("body", *KEEP_HELPERS):
        vg = basemesh.vertex_groups.get(vg_name)
        if vg is None:
            continue
        for v in me.vertices:
            for g in v.groups:
                if g.group == vg.index and g.weight > 0.5:
                    keep.add(v.index)
    import bmesh

    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    doomed = [v for v in bm.verts if v.index not in keep]
    bmesh.ops.delete(bm, geom=doomed, context="VERTS")
    bm.to_mesh(me)
    bm.free()
    for vg in list(basemesh.vertex_groups):
        helper = vg.name.startswith("joint-") or vg.name.startswith("helper-")
        if helper and vg.name not in KEEP_HELPERS:
            basemesh.vertex_groups.remove(vg)


def face_coords(human_service, target_service, age, details):
    """Vertex coordinates of the body with face `details` at `age` (helpers stripped)."""
    basemesh = human_service.create_human(
        mask_helpers=False,
        detailed_helpers=True,
        extra_vertex_groups=True,
        feet_on_ground=True,
        scale=0.1,
        macro_detail_dict={**MACRO, "age": age},
    )
    apply_details(basemesh, target_service)
    for name, weight in details.items():
        path = target_service.target_full_path(name)
        if path is None:
            print(f"WARN target not found: {name}")
            continue
        target_service.load_target(basemesh, path, weight=weight, name=name)
    target_service.bake_targets(basemesh)
    strip_helpers(basemesh)
    coords = [basemesh.matrix_world @ v.co for v in basemesh.data.vertices]
    bpy.data.objects.remove(basemesh)
    return coords


def add_faces(basemesh, human_service, target_service):
    """Shape keys ``face_<k>`` (lot FG1) on the finished body, one per `FACES` entry."""
    inv = basemesh.matrix_world.inverted()
    if basemesh.data.shape_keys is None:
        basemesh.shape_key_add(name="Basis", from_mix=False)
    for k, (age, details) in enumerate(FACES):
        coords = face_coords(human_service, target_service, age, details)
        if len(coords) != len(basemesh.data.vertices):
            raise RuntimeError(f"face {k}: topology differs ({len(coords)} verts)")
        key = basemesh.shape_key_add(name=f"face_{k}", from_mix=False)
        for i, co in enumerate(coords):
            key.data[i].co = inv @ co
        print(f"FACE {k} age={age} targets={len(details)}")


def main():
    """Create the human, bake the targets, rig it and save the .blend."""
    # No factory reset: it would unload the extension. Empty the startup scene instead.
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj)
    mpfb = mpfb_module()
    from importlib import import_module

    base = mpfb.__name__
    human_service = import_module(base + ".services.humanservice").HumanService
    target_service = import_module(base + ".services.targetservice").TargetService

    basemesh = human_service.create_human(
        mask_helpers=False,
        detailed_helpers=True,
        extra_vertex_groups=True,
        feet_on_ground=True,
        scale=0.1,
        macro_detail_dict=MACRO,
    )
    apply_details(basemesh, target_service)
    rig = human_service.add_builtin_rig(basemesh, "game_engine", import_weights=True)
    target_service.bake_targets(basemesh)
    for mod in list(basemesh.modifiers):
        if mod.type != "ARMATURE":
            basemesh.modifiers.remove(mod)
    strip_helpers(basemesh)
    add_faces(basemesh, human_service, target_service)
    basemesh.name = basemesh.data.name = "MH_Body"
    rig.name = rig.data.name = "MH_Rig"
    zs = [(basemesh.matrix_world @ v.co).z for v in basemesh.data.vertices]
    print(
        f"MH body verts={len(basemesh.data.vertices)} faces={len(basemesh.data.polygons)} "
        f"height={max(zs) - min(zs):.3f} bones={len(rig.data.bones)}"
    )
    os.makedirs(OUT_DIR, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=OUT, compress=True)
    print("SAVED", OUT)


if __name__ == "__main__":
    main()
