"""GA3-L3: generated battle figures (fal TRELLIS 2) on the fine ``human`` rig, game format.

Chain of the S2 probe (``ga3_figure_probe``) generalised: the TRELLIS 2 mesh of an A-pose,
empty-handed reference (``tools/experiments/ga3_fal_figure.py``) is cleaned and scaled to the
rig. TRELLIS 2 surfaces are open, doubled sheets (bone heat fails on them, collapse
decimation stalls): the figure is thickened (solidify 12 mm), voxel-remeshed (8 mm, outer
shell kept) and collapsed to the LOD0 budget of the fine foot soldiers, smart-UV unwrapped
and baked from the source (Cycles, selected to active: graded albedo and tint classes); it
is then auto-weighted on the rig posed onto the mesh and brought back to the bind pose.
LOD1 / LOD2 are decimated copies (weights and UVs kept, same texture). The held weapon is the procedural one of the fine figure it replaces
(``battle_fine_weapons.longbow``: limbs on ``Wrist.L``, string on ``Nock``, arrow on
``Arrow``). Exported as ``CAM1`` (``battle_skinned.export_mesh``) with per-face material
codes, plus one albedo texture shared by the three LODs:

- RGB: the TRELLIS albedo, graded (auto-levels), the key-coloured zones turned to grey
  luminance (no green / blue fringe in the mipmaps);
- A: tint mask (1 = the livery garment or the hose, painted in key colours in the reference:
  saturated green / blue; 0 = kept as generated). The face code tells which: ``C_LIVERY``
  (livery, arms and badge of the game) or ``C_CLOTH`` (per-soldier natural colour);
  ``C_PLATE`` where TRELLIS says metallic (helmet), ``C_EXACT`` elsewhere. The shader
  (``GA3_TEX``) multiplies the game colour by the texture luminance over its mean
  (``ga3_lum``: livery, cloth), so folds, quilting and dirt stay.

The equipment keeps its UVs shifted to negative u (the shader leaves it untextured).

Run from the repository root::

    blender -b --factory-startup --python tools/blender_scripts/ga3_figures.py -- \
        <unit> [--raw ~/dev/cent-ans-raw/ga3/l3] [--renders]
    python3 tools/blender_scripts/ga3_figures.py sheet <unit> <jpg> [--raw DIR] \
        [--tags multi_front_back,trellis,trellis2]
    python3 tools/blender_scripts/ga3_figures.py board <jpg> [--units a,b] [--raw DIR]

Output: ``game/assets/models/battle_ga3/`` (``<figure>_lod{0,1,2}.mesh.bin``,
``<figure>_albedo.png``, ``manifest.json`` whose entries override the fine figures in
``BattleSkinned``).
"""

import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

ROOT = os.path.dirname(os.path.dirname(HERE))
OUT_DIR = os.environ.get("GA3_OUT_DIR") or os.path.join(
    ROOT, "game", "assets", "models", "battle_ga3"
)  # GA3_OUT_DIR: rebake elsewhere
FINE_DIR = os.path.join(ROOT, "game", "assets", "models", "battle_fine")
RAW = os.path.expanduser("~/dev/cent-ans-raw/ga3")
L4_RAW = "l4"  # renders, .blend and the L4 generations, under RAW

# Per unit: fine figure replaced, generated mesh and reference (under RAW), helmet top (m),
# held equipment (item names of the fine recipe: builder, variant mask and kwargs come from
# ``battle_skinned_figures.FIGURES`` with the SR3b overrides), steel height (``metal_z``: faces
# classed as steel below it keep the texture code; 0 = the whole harness is steel), and the
# clips of the renders (walk, attack; the sheet shows them beside the current figure).
UNITS = {
    "longbowman": {
        "figure": "archer_0",
        "glb": "l3/longbowman/multi_front_back.glb",
        "reference": "l3/longbowman/sheet.png",
        "faces": ["l4/longbowman_v2"],
        "top": 1.80,
        "equipment": ["longbow"],
        "metal_z": 1.5,
        "clips": {"idle": "bow_idle", "walk": "bow_walk", "attack": "bow_shoot"},
    },
    # L3b. Others sharing the figure type: infantry_7, infantry_8 (harness / sword).
    "man_at_arms": {
        "figure": "infantry_0",
        "glb": "l3/man_at_arms/multi_front_back.glb",
        "reference": "l3/man_at_arms/sheet.png",
        "faces": ["l4/man_at_arms_v2"],
        "top": 1.86,
        "equipment": ["sword", "heater_shield"],
        "metal_z": 0.0,
        "clips": {"idle": "idle", "walk": "walk", "attack": "slash"},
    },
    # Others: archer_1, archer_4 (crossbow).
    "crossbowman": {
        "figure": "archer_2",
        "glb": "l3/crossbowman/multi_front_back.glb",
        "reference": "l3/crossbowman/sheet.png",
        "faces": ["l4/crossbowman_v2"],
        "top": 1.80,
        "equipment": ["crossbow", "quiver", "pavise"],
        "metal_z": 1.5,
        "clips": {"idle": "xbow_idle", "walk": "walk", "attack": "xbow_shoot"},
    },
    # Others: infantry_4 (pike).
    "sergeant": {
        "figure": "infantry_1",
        "glb": "l3/sergeant/multi_front_back.glb",
        "reference": "l3/sergeant/sheet.png",
        "faces": ["l4/sergeant_v2"],
        "top": 1.84,
        "equipment": ["pike"],
        "metal_z": 1.5,
        "clips": {"idle": "pike_idle", "walk": "pike_walk", "attack": "pike_thrust"},
    },
    # Others: infantry_2, infantry_3, infantry_6 (staff weapons of the militia).
    "militia": {
        "figure": "infantry_5",
        "glb": "l3/militia/multi_front_back.glb",
        "reference": "l3/militia/sheet.png",
        "faces": ["l4/militia_v2"],
        "top": 1.80,
        "equipment": ["goedendag"],
        "metal_z": 1.5,
        "clips": {"idle": "pike_idle", "walk": "pike_walk", "attack": "pike_thrust"},
    },
    # L3c: the knight's rider on the fine FG4 horse of cavalry_0 (``mounted``: built on the
    # human rig, moved onto the ``cavalry`` rider bones, merged with the fine horse of the
    # exported figure, see ``build_mounted``). Others sharing the unit type: cavalry_3
    # (gendarmes, white harness), standard_1 (mounted standard bearer).
    "knight": {
        "figure": "cavalry_0",
        "glb": "l3/knight/multi_front_back.glb",
        "reference": "l3/knight/sheet.png",
        "faces": ["l4/knight_v2"],
        "top": 1.84,
        "equipment": ["heater_shield", "lance"],
        "scabbard": True,
        "metal_z": 0.0,
        "mounted": True,
        "clips": {"idle": "c_idle", "walk": "c_charge", "attack": "c_thrust"},
    },
}


# L4: the remaining recipes, generated as edits of an L3 sheet (``parent``: same pose and
# scale; the helmet height comes from the ratio of the figure heights on the two sheets).
def _derived(figure, parent, equipment, clips, metal_z=1.5, **extra):
    name = extra.pop("name")
    return {
        "figure": figure,
        "parent": parent,
        "glb": f"l4/{name}/multi_front_back.glb",
        "reference": f"l4/{name}/sheet.png",
        "faces": extra.pop("faces", [f"l4/{name}_v2"]),
        "equipment": equipment,
        "metal_z": metal_z,
        "clips": dict(zip(("idle", "walk", "attack"), clips, strict=True)),
        **extra,
    }


POLE = ("pike_idle", "pike_walk", "pike_thrust")
XBOW = ("xbow_idle", "walk", "xbow_shoot")
UNITS.update(
    {
        "retinue": _derived(
            "infantry_7", "man_at_arms", ["pollaxe"], POLE, 0.0, name="retinue"
        ),
        "routier": _derived(
            "infantry_8",
            "man_at_arms",
            ["sword", "round_shield"],
            ("idle", "walk", "slash"),
            name="routier",
        ),
        "urban_militia": _derived(
            "infantry_2", "militia", ["spear", "bill"], POLE, name="urban_militia"
        ),
        "welsh_spearman": _derived(
            "infantry_3",
            "militia",
            ["spear", "round_shield"],
            POLE,
            name="welsh_spearman",
        ),
        "schiltron": _derived(
            "infantry_4", "sergeant", ["pike", "round_shield"], POLE, name="schiltron"
        ),
        "coutilier": _derived(
            "infantry_6", "militia", ["coustille"], POLE, name="coutilier"
        ),
        "plain_crossbowman": _derived(
            "archer_1",
            "crossbowman",
            ["crossbow", "quiver"],
            XBOW,
            name="plain_crossbowman",
        ),
        "gascon_crossbowman": _derived(
            "archer_4",
            "crossbowman",
            ["crossbow", "quiver", "pavise"],
            XBOW,
            name="gascon_crossbowman",
        ),
        "gendarme": _derived(
            "cavalry_3",
            "knight",
            ["lance"],
            ("c_idle", "c_charge", "c_thrust"),
            0.0,
            name="gendarme",
            mounted=True,
        ),
        "standard_bearer": _derived(
            "standard_1",
            "knight",
            ["standard_pole"],
            ("c_idle", "c_charge", "c_idle"),
            0.0,
            name="standard_bearer",
            mounted=True,
            # Budget: the one recipe without a face variant (seen from afar, one per army).
            faces=[],
        ),
    }
)
KEY_LIVERY = (75.0, 170.0)  # saturated green
KEY_CLOTH = (190.0, 265.0)  # saturated blue
KEY_SAT = 0.28
CLOSE_STEPS = (
    8  # px at the 2048 source: dirt spots inside a key colour zone are tinted too
)
METAL_Z = 1.5  # m, default ``metal_z``: steel code on the helmet only (dagger, buckles)
TRI_CAP = (11900, 1350, 260)  # battle_fine_figures.TRI_CAP (foot soldiers)
RIDER_CAP = (9000, 1000, 180)  # battle_fine_figures.RIDER_CAP (the horse is FG4's)
TEX = 1024
AUTO_LEVELS = 0.5  # percent clipped at each end (ga3_cleanup, L1)
EQUIP_UV_SHIFT = -4.0
SOLIDIFY = 0.012  # m, thickens the open TRELLIS sheets before the voxel remesh
VOXEL = 0.008  # m
LIVERY_MIN_Z = 0.5  # m: livery faces reaching below (shins) keep the generated colour
ARM_MIN_Z = 0.6  # m: no arm bone weights below (see ``strip_arm_leaks``)
# Arm and hand bones of the fine rig (fingers are aliased to ``Wrist`` at export).
ARM_BONE_KEYS = (
    "Shoulder",
    "Arm",
    "Wrist",
    "Index",
    "Middle",
    "Ring",
    "Pinky",
    "Thumb",
)
KEEP_ISLAND = 0.02  # share of the voxel vertices under which an island is dropped
# Material codes (battle_skinned_equipment / battle_soldier_skinned.gdshader).
C_LIVERY, C_PLATE, C_CLOTH, C_EXACT, C_ARMS = 0, 2, 4, 5, 6


# --- Albedo and masks ---------------------------------------------------------------------


def _pixels(image):
    """(H, W, 4) float32 array of a Blender image (rows bottom-up)."""
    import numpy as np

    w, h = image.size
    px = np.empty(w * h * 4, dtype=np.float32)
    image.pixels.foreach_get(px)
    return px.reshape(h, w, 4)


def _hue_sat(rgb):
    """Hue (degrees) and HSV saturation of an (..., 3) array."""
    import numpy as np

    mx = rgb.max(axis=-1)
    mn = rgb.min(axis=-1)
    d = mx - mn
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    safe = np.where(d > 1e-6, d, 1.0)
    hue = np.where(
        mx == r,
        ((g - b) / safe) % 6.0,
        np.where(mx == g, (b - r) / safe + 2.0, (r - g) / safe + 4.0),
    )
    hue = np.where(d > 1e-6, hue * 60.0, -1.0)
    sat = np.where(mx > 1e-6, d / np.where(mx > 1e-6, mx, 1.0), 0.0)
    return hue, sat, mx


def _srgb_to_linear(c):
    import numpy as np

    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def _grow(mask, steps, value):
    """Binary dilation (`value` True) or erosion (False) by `steps` 3x3 passes."""
    import numpy as np

    out = mask.copy()
    for _ in range(steps):
        pad = np.pad(out, 1, constant_values=not value)
        acc = out.copy()
        for dy in (0, 1, 2):
            for dx in (0, 1, 2):
                win = pad[dy : dy + out.shape[0], dx : dx + out.shape[1]]
                acc = (acc | win) if value else (acc & win)
        out = acc
    return out


def close_mask(mask, steps=CLOSE_STEPS):
    """Morphological closing: dirt spots inside a key-coloured garment join the zone."""
    return _grow(_grow(mask, steps, True), steps, False)


def texture_images(obj):
    """(base colour image, metallic-roughness image or None) of the TRELLIS material."""
    base, orm = None, None
    for slot in obj.material_slots:
        nt = slot.material.node_tree if slot.material else None
        if nt is None:
            continue
        for n in nt.nodes:
            if n.type != "TEX_IMAGE":
                continue
            targets = [lk.to_socket.name for lk in n.outputs[0].links]
            if "Base Color" in targets:
                base = n.image
            else:
                orm = n.image
    return base, orm


def source_maps(base, orm):
    """Graded source albedo and tint-class map, at the TRELLIS texture size.

    Returns (albedo rgb sRGB, classes rgb (R livery, G cloth, B metal), bottom-up float
    arrays) and the linear mean luminance of the livery and cloth texels.
    """
    import numpy as np

    src = _pixels(base)[..., :3]
    h, w = src.shape[:2]
    covered = src.max(axis=-1) > 0.004
    hue, sat, val = _hue_sat(src)
    live = (
        (hue >= KEY_LIVERY[0]) & (hue <= KEY_LIVERY[1]) & (sat > KEY_SAT) & (val > 0.05)
    )
    cloth = (
        (hue >= KEY_CLOTH[0]) & (hue <= KEY_CLOTH[1]) & (sat > KEY_SAT) & (val > 0.05)
    )
    live = close_mask(live)
    cloth = close_mask(cloth) & ~live
    # Steel: the metallic map of TRELLIS 2, else grey bright texels; ``code_faces`` keeps it
    # on the helmet only (above ``METAL_Z``).
    metal = (sat < 0.15) & (val > 0.3) & ~live & ~cloth
    if orm is not None:
        mr = _pixels(orm)
        if mr.shape[:2] == (h, w):
            metal = (mr[..., 2] > 0.5) & ~live & ~cloth
    values = src[covered & ~live & ~cloth]
    low = float(np.percentile(values, AUTO_LEVELS))
    high = float(np.percentile(values, 100.0 - AUTO_LEVELS))
    rgb = np.clip((src - low) / max(1e-3, high - low), 0.0, 1.0)
    lum = rgb @ np.array([0.3, 0.59, 0.11], dtype=np.float32)
    tint = live | cloth
    rgb[tint] = lum[tint, None]
    classes = np.stack([live, cloth, metal], axis=-1).astype(np.float32)
    lin = _srgb_to_linear(rgb) @ np.array([0.3, 0.59, 0.11], dtype=np.float32)
    lum_live = float(lin[live].mean()) if live.any() else 0.2
    lum_cloth = float(lin[cloth].mean()) if cloth.any() else 0.2
    share = {
        "livery": float(live[covered].mean()),
        "cloth": float(cloth[covered].mean()),
        "metal": float(metal[covered].mean()),
    }
    print(
        f"GA3 albedo low={low:.3f} high={high:.3f} lum=({lum_live:.3f}, {lum_cloth:.3f}) "
        f"share={ {k: round(v, 3) for k, v in share.items()} }"
    )
    return rgb.astype(np.float32), classes, (lum_live, lum_cloth)


def image_from(array, name, data=False):
    """Blender image from a bottom-up (H, W, 3|4) float array."""
    import bpy
    import numpy as np

    h, w = array.shape[:2]
    if array.shape[2] == 3:
        array = np.concatenate([array, np.ones((h, w, 1), dtype=np.float32)], axis=-1)
    img = bpy.data.images.new(name, w, h, alpha=True)
    if data:
        img.colorspace_settings.name = "Non-Color"
    img.pixels.foreach_set(array.astype(np.float32).reshape(-1))
    return img


def save_png(rgba, path, name):
    """Write a bottom-up RGBA float array as an 8-bit sRGB PNG (straight alpha)."""
    import bpy

    h, w = rgba.shape[:2]
    img = bpy.data.images.new(name, w, h, alpha=True)
    img.alpha_mode = "STRAIGHT"
    img.pixels.foreach_set(rgba.reshape(-1))
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    return img


def preview_rgba(
    rgba, classes, lum, livery=(0.62, 0.08, 0.06), hose=(0.27, 0.16, 0.08)
):
    """Albedo as the game tints it (linear): livery red, hose brown (renders only)."""
    import numpy as np

    rgb = _srgb_to_linear(rgba[..., :3])
    a = rgba[..., 3:4]
    l_lin = rgb @ np.array([0.3, 0.59, 0.11], dtype=np.float32)
    live = (
        np.asarray(livery, dtype=np.float32)
        * np.clip(l_lin / lum[0], 0.2, 2.0)[..., None]
    )
    cloth = (
        np.asarray(hose, dtype=np.float32)
        * np.clip(l_lin / lum[1], 0.2, 2.0)[..., None]
    )
    tinted = np.where(classes[..., 1:2] > 0.5, cloth, live)
    return rgb * (1 - a) + tinted * a


# --- Mesh ---------------------------------------------------------------------------------


def face_classes(obj, classes):
    """Class of every polygon: majority over its corners and centroid (UV lookups)."""
    import numpy as np

    me = obj.data
    uv = me.uv_layers.active.data
    n = classes.shape[0]
    label = np.zeros(classes.shape[:2], dtype=np.uint8)
    label[classes[..., 2] > 0.5] = 3
    label[classes[..., 1] > 0.5] = 2
    label[classes[..., 0] > 0.5] = 1
    out = []
    for poly in me.polygons:
        pts = [uv[li].uv for li in poly.loop_indices]
        cx = sum(p[0] for p in pts) / len(pts)
        cy = sum(p[1] for p in pts) / len(pts)
        votes = [0, 0, 0, 0]
        for u, v in [(cx, cy), (cx, cy)] + [(p[0], p[1]) for p in pts]:
            x = min(max(int(u * n), 0), n - 1)
            y = min(max(int(v * n), 0), n - 1)
            votes[int(label[y, x])] += 1
        out.append(int(np.argmax(votes)))
    return out


def coded_materials(image):
    """One material per class: game code and white base colour, albedo image on the UV.

    The code and colour are custom properties read by ``export_mesh``; the image serves the
    renders only.
    """
    import bpy

    mats = []
    for code, name, metal in (
        (C_EXACT, "fixed", 0.0),
        (C_LIVERY, "livery", 0.0),
        (C_CLOTH, "cloth", 0.0),
        (C_PLATE, "metal", 0.7),
    ):
        mat = bpy.data.materials.new(f"ga3_{name}")
        mat["code"] = code
        mat["rgb"] = [1.0, 1.0, 1.0]
        mat.use_nodes = True
        nt = mat.node_tree
        bsdf = nt.nodes["Principled BSDF"]
        tex = nt.nodes.new("ShaderNodeTexImage")
        tex.image = image
        nt.links.new(tex.outputs[0], bsdf.inputs["Base Color"])
        bsdf.inputs["Metallic"].default_value = metal
        bsdf.inputs["Roughness"].default_value = 0.45 if metal else 0.85
        mats.append(mat)
    return mats


def code_faces(obj, classes, mats, metal_z=METAL_Z):
    """Replace the TRELLIS material by the coded ones, per face class."""
    import numpy as np

    fc = face_classes(obj, classes)
    verts = obj.data.vertices
    mw = obj.matrix_world
    for i, poly in enumerate(obj.data.polygons):
        if fc[i] == 3 and (mw @ poly.center).z < metal_z:
            fc[i] = 0
        # No livery garment reaches the shins: stray key-coloured texels there (grime on
        # greaves) would flutter with the cloth motion (AN1a) once decimated.
        if (
            fc[i] == 1
            and min((mw @ verts[v].co).z for v in poly.vertices) < LIVERY_MIN_Z
        ):
            fc[i] = 0
    obj.data.materials.clear()
    for m in mats:
        obj.data.materials.append(m)
    obj.data.polygons.foreach_set("material_index", np.array(fc, dtype=np.int32))
    counts = [fc.count(k) for k in range(4)]
    print(f"GA3 faces fixed/livery/cloth/metal = {counts}")
    return counts


def heat(obj, wrig):
    """Bone heat of `obj` on `wrig` directly; returns the weighted vertex share."""
    import bpy

    for o in bpy.context.selected_objects:
        o.select_set(False)
    obj.select_set(True)
    wrig.select_set(True)
    bpy.context.view_layer.objects.active = wrig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    for m in [m for m in obj.modifiers if m.type == "ARMATURE"]:
        obj.modifiers.remove(m)
    mw = obj.matrix_world.copy()
    obj.parent = None
    obj.matrix_world = mw
    share = sum(1 for v in obj.data.vertices if v.groups) / max(
        1, len(obj.data.vertices)
    )
    print(f"HEAT direct weighted={share:.3f} arm leaks removed={strip_arm_leaks(obj)}")
    return share


def strip_arm_leaks(obj):
    """Drop arm weights below ``ARM_MIN_Z`` (heat leaks from the A-pose hands to the legs).

    The hands hang at about 0.8 m: an arm bone moving a shin (sergeant: hose weighted to
    ``Wrist.R``) is always a leak. The remaining weights are renormalised.
    """
    arm_groups = {
        g.index for g in obj.vertex_groups if any(k in g.name for k in ARM_BONE_KEYS)
    }
    mw = obj.matrix_world
    fixed = 0
    for v in obj.data.vertices:
        if (mw @ v.co).z >= ARM_MIN_Z:
            continue
        # Indices first: removing a group invalidates the ``v.groups`` elements.
        leak = [g.group for g in v.groups if g.group in arm_groups]
        rest = sum(g.weight for g in v.groups if g.group not in arm_groups)
        if not leak or rest <= 1e-4:
            continue
        for index in leak:
            obj.vertex_groups[index].remove([v.index])
        for g in v.groups:
            g.weight /= rest
        fixed += 1
    return fixed


def skin_to_bind(obj, arm):
    """``ga3_figure_probe.skin_to_bind`` (fit, heat on a segment rig, back to the bind pose)."""
    import battle_fine_proto as fp
    import bpy
    import ga3_figure_probe as probe

    fit = arm.copy()
    fit.data = arm.data.copy()
    fit.name = "ga3_fit"
    bpy.context.scene.collection.objects.link(fit)
    fit.animation_data_clear()
    for pb in fit.pose.bones:
        for c in list(pb.constraints):
            pb.constraints.remove(c)
    limbs = probe.mesh_limbs(obj)
    probe.fit_rig_to_mesh(fit, limbs)
    for o in bpy.context.selected_objects:
        o.select_set(False)
    bpy.context.view_layer.objects.active = fit
    fit.select_set(True)
    bpy.ops.object.mode_set(mode="POSE")
    bpy.ops.pose.armature_apply(selected=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    fit.select_set(False)
    wrig = probe.weight_rig(fit)
    share = heat(obj, wrig)
    bpy.data.objects.remove(wrig)
    mod = obj.modifiers.new("fit", "ARMATURE")
    mod.object = fit
    for b in probe.bs_hierarchy(fit):
        fit.pose.bones[b.name].matrix = arm.data.bones[b.name].matrix_local.copy()
        bpy.context.view_layer.update()
    with bpy.context.temp_override(object=obj, active_object=obj):
        bpy.ops.object.modifier_apply(modifier=mod.name)
    obj.data.transform(obj.matrix_world)
    obj.matrix_world.identity()
    bpy.data.objects.remove(fit)
    fp.attach(obj, arm)
    return share > 0.99, limbs


def closed_low(hi, target, name):
    """Closed, collapsible copy of `hi` at about `target` triangles (see module doc)."""
    import battle_skinned as bs
    import bpy
    import ga3_figure_probe as probe

    low = hi.copy()
    low.data = hi.data.copy()
    low.name = name
    bpy.context.scene.collection.objects.link(low)
    low.modifiers.clear()
    for kind, key, value in (
        ("SOLIDIFY", "thickness", SOLIDIFY),
        ("REMESH", "voxel_size", VOXEL),
    ):
        mod = low.modifiers.new(kind.lower(), kind)
        setattr(mod, key, value)
        if kind == "SOLIDIFY":
            mod.offset = 0.0
        else:
            mod.mode = "VOXEL"
        with bpy.context.temp_override(object=low, active_object=low):
            bpy.ops.object.modifier_apply(modifier=mod.name)
    # Legs under a tabard may come out as islands of their own (≈ 6 % each, crossbowman):
    # only the voxel crumbs go.
    islands, dropped = probe.drop_islands(low, keep_ratio=KEEP_ISLAND)
    dense = len(low.data.polygons)
    for _ in range(4):
        if len(low.data.polygons) <= target * 1.02:
            break
        bs.weld_and_decimate(low, target)
    print(
        f"LOW voxel={dense} islands={islands} dropped={dropped} -> {len(low.data.polygons)}"
    )
    return low


def bake_low(hi, low, albedo, classes, size=TEX):
    """Smart-UV `low`, bake `hi`'s albedo and tint classes into two TEX² images."""
    import bpy

    for layer in list(low.data.uv_layers):
        low.data.uv_layers.remove(layer)
    low.data.uv_layers.new(name="UVMap")
    for o in bpy.context.selected_objects:
        o.select_set(False)
    low.select_set(True)
    bpy.context.view_layer.objects.active = low
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(66.0), island_margin=0.006)
    bpy.ops.object.mode_set(mode="OBJECT")
    src = bpy.data.materials.new("ga3_source")
    src.use_nodes = True
    s_bsdf = src.node_tree.nodes["Principled BSDF"]
    s_bsdf.inputs["Metallic"].default_value = 0.0
    s_tex = src.node_tree.nodes.new("ShaderNodeTexImage")
    src.node_tree.links.new(s_tex.outputs[0], s_bsdf.inputs["Base Color"])
    hi.data.materials.clear()
    hi.data.materials.append(src)
    dst = bpy.data.materials.new("ga3_bake")
    dst.use_nodes = True
    d_tex = dst.node_tree.nodes.new("ShaderNodeTexImage")
    low.data.materials.clear()
    low.data.materials.append(dst)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 4
    bake = scene.render.bake
    bake.use_selected_to_active = True
    bake.cage_extrusion = 0.03
    bake.max_ray_distance = 0.08
    bake.margin = 8
    bake.use_pass_direct = False
    bake.use_pass_indirect = False
    bake.use_pass_color = True
    out = []
    for name, image, data in (("albedo", albedo, False), ("classes", classes, True)):
        s_tex.image = image
        target = bpy.data.images.new(f"ga3_low_{name}", size, size, alpha=False)
        if data:
            target.colorspace_settings.name = "Non-Color"
        d_tex.image = target
        dst.node_tree.nodes.active = d_tex
        for o in bpy.context.selected_objects:
            o.select_set(False)
        hi.select_set(True)
        low.select_set(True)
        bpy.context.view_layer.objects.active = low
        bpy.ops.object.bake(type="DIFFUSE", pass_filter={"COLOR"})
        out.append(_pixels(target))
    print("BAKE albedo + classes", size)
    return out


def lod_copy(obj, target, name):
    """Decimated copy of the weighted body (groups, UVs, materials kept)."""
    import battle_skinned as bs
    import bpy

    dup = obj.copy()
    dup.data = obj.data.copy()
    dup.name = name
    bpy.context.scene.collection.objects.link(dup)
    for m in [m for m in dup.modifiers if m.type == "ARMATURE"]:
        dup.modifiers.remove(m)
    before = len(dup.data.polygons)
    after = bs.weld_and_decimate(dup, target)
    print(
        f"LOD {name} {before} -> {after} (target {target}) mods={[m.type for m in dup.modifiers]}"
    )
    import battle_fine_proto as fp

    fp.attach(dup, obj.parent)
    return dup


def recipe_items(unit):
    """(name, variant mask, kwargs) of the unit's held items, from the fine recipe."""
    import battle_fine_sr as sr
    import battle_skinned_figures as figures

    fig = unit["figure"]
    recipe = sr.fine_recipe(fig, figures.FIGURES[fig])
    items = {}
    for item in recipe.get("equipment", []):
        items.setdefault(item[0], (item[1], item[2] if len(item) > 2 else {}))
    out = []
    for name in unit["equipment"]:
        mask, kwargs = items.get(name, (0, {}))
        out.append((name, mask, kwargs))
    return out


def equipment(unit, arm, level):
    """Held items of the unit at `level` (procedural, bound to the rig's bones).

    Same builders as the fine figure (``battle_fine_figures.build_figure``): the fine
    ``battle_fine_gear`` / ``battle_fine_weapons`` registry, else the V2 weapons and
    equipment. UVs move to negative u (untextured by ``GA3_TEX``) except on the painted
    faces (``C_ARMS``: shields, pavise), which keep the heraldry UV.
    """
    import battle_fine_gear as gear
    import battle_skinned as bs
    import battle_skinned_cavalry as cav
    import battle_skinned_equipment as eq
    import battle_skinned_weapons as weapons

    mounted = unit.get("mounted", False)
    ctx = eq.Context(arm, level, bs.material, bs.bone_world)
    out = []
    for name, mask, kwargs in recipe_items(unit):
        objs = gear.build(
            name, kwargs, gear.Gear(ctx, None, [], unit["figure"], mounted)
        )
        if objs is None:
            builder = (
                (getattr(cav, name, None) if mounted else None)
                or getattr(weapons, name, None)
                or getattr(eq, name)
            )
            objs = builder(ctx, **kwargs)
            for obj in objs:
                gear.unwrap(obj)
        for obj in objs:
            shift_equipment_uv(obj)
            # Riders take the recipe mask as is (``battle_fine_figures.build_figure``).
            bs.set_face_mask(obj, mask if mounted else bs.held_mask(name, mask, kwargs))
            out.append(obj)
    if unit.get("scabbard"):
        out.extend(scabbard(arm))
    return out


def scabbard(arm):
    """Sword in its scabbard at the left hip (``battle_fine_equipment.scabbard``, FINE_KIT).

    The fine builder only reads two landmarks of the fitted body: the left hip joint and
    the waist height (``Landmarks``), taken here from the rig.
    """
    from types import SimpleNamespace

    import battle_fine_equipment as fe
    import battle_fine_gear as gear
    import battle_skinned as bs

    bone = {b.name: arm.matrix_world @ b.head_local for b in arm.data.bones}
    lm = SimpleNamespace(bone=bone, waist_z=bone["Abdomen"].z + 0.02)
    objs = fe.scabbard(lm)
    for obj in objs:
        gear.unwrap(obj)
        shift_equipment_uv(obj)
        bs.set_face_mask(obj, 0)
    return objs


def shift_equipment_uv(obj):
    """Move the UVs of `obj` to negative u, except on its ``C_ARMS`` faces."""
    uv = obj.data.uv_layers.active
    if uv is None:
        return
    mats = obj.data.materials
    for poly in obj.data.polygons:
        mat = mats[poly.material_index] if poly.material_index < len(mats) else None
        if mat is not None and int(mat.get("code", -1)) == C_ARMS:
            continue
        for li in poly.loop_indices:
            d = uv.data[li]
            d.uv = (d.uv[0] + EQUIP_UV_SHIFT, d.uv[1])


def tris(objs):
    """Triangle count of objects."""
    total = 0
    for o in objs:
        o.data.calc_loop_triangles()
        total += len(o.data.loop_triangles)
    return total


# --- L4: face variants (grafted heads) and fine hands --------------------------------------

HEAD_TRIS = (
    1300,
    110,
)  # triangles of a grafted head variant at LOD0, LOD1 (LOD2: head A)
HEAD_OVERLAP = (
    0.04  # m: a variant head reaches this far below the cut, inside the collar
)
FACE_TEX = TEX // 2  # texels of a variant head's square in the strip under the atlas
GRAFT_MAX_OFFSET = 0.02  # m, horizontal shift of a variant head onto the neck
HAND_MIN = 0.5  # fine hand: triangles whose vertices all weigh this much on a wrist
MITTEN_MIN = (
    0.6  # generated mitten: body triangles whose vertices all weigh this on a wrist
)
HAND_RADIUS = (
    0.16  # m around the mitten's centroid: the fine hand (not a shield or a hilt)
)


def figure_box(path):
    """(top, feet) rows over the height of the front figure of a cut sheet (alpha)."""
    import bpy

    img = bpy.data.images.load(path)
    alpha = _pixels(img)[::-1, :, 3] > 32.0 / 255.0
    bpy.data.images.remove(img)
    cols = alpha.sum(axis=0) > 2
    x0 = x1 = None
    for x, on in enumerate(list(cols) + [False]):
        if on and x0 is None:
            x0 = x
        elif not on and x0 is not None:
            if x - x0 > 0.03 * len(cols):
                x1 = x
                break
            x0 = None
    rows = alpha[:, x0:x1].any(axis=1).nonzero()[0]
    h = alpha.shape[0]
    return float(rows[0]) / h, float(rows[-1] + 1) / h


def figure_height(raw, rel_dir):
    """Height of the front figure on the cut sheet of `rel_dir`, over the sheet height."""
    top, feet = figure_box(os.path.join(raw, rel_dir, "sheet_cut.png"))
    return feet - top


def unit_top(unit_name, raw):
    """Helmet height (m): given for the L3 units, else scaled from the parent's sheet."""
    unit = UNITS[unit_name]
    if "top" in unit:
        return unit["top"]
    parent = UNITS[unit["parent"]]
    ratio = figure_height(raw, os.path.dirname(unit["glb"])) / figure_height(
        raw, os.path.dirname(parent["glb"])
    )
    return parent["top"] * ratio


def neck_cut(arm):
    """Height (m, bind pose) of the cut between the shared body and the variant heads."""
    head = {b.name: arm.matrix_world @ b.head_local for b in arm.data.bones}
    return 0.5 * (head["Neck"].z + head["Head"].z)


def delete_faces(obj, keep):
    """Remove the faces of `obj` for which ``keep(world centre, world vertex zs)`` is false."""
    import bmesh

    mw = obj.matrix_world
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    gone = [
        f
        for f in bm.faces
        if not keep(mw @ f.calc_center_median(), [(mw @ v.co).z for v in f.verts])
    ]
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    bm.to_mesh(obj.data)
    bm.free()


def cap_hole(obj, z_min):
    """Close the open neck of the shared body (boundary edges above `z_min`).

    The cap is hidden inside head A; under a narrower variant head it shows as the collar
    (UV of each corner taken from the vertex's other faces). Returns the faces added.
    """
    import bmesh

    mw = obj.matrix_world
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    edges = [
        e
        for e in bm.edges
        if e.is_boundary and min((mw @ v.co).z for v in e.verts) > z_min
    ]
    before = set(bm.faces)
    bmesh.ops.holes_fill(bm, edges=edges, sides=0)
    new = [f for f in bm.faces if f not in before]
    uv = bm.loops.layers.uv.active
    for f in new:
        near = next((g for e in f.edges for g in e.link_faces if g not in new), None)
        if near is not None:
            f.material_index = near.material_index
        for loop in f.loops:
            other = next(
                (lp for lp in loop.vert.link_loops if lp.face not in new), None
            )
            if other is not None and uv is not None:
                loop[uv].uv = other[uv].uv
    res = bmesh.ops.triangulate(bm, faces=new)
    bm.to_mesh(obj.data)
    bm.free()
    return len(res["faces"])


def split_head(obj, z_cut, name):
    """Split `obj` at `z_cut`: the head above becomes `name`, the body is capped."""
    import bpy

    head = obj.copy()
    head.data = obj.data.copy()
    head.name = name
    bpy.context.scene.collection.objects.link(head)
    delete_faces(head, lambda c, _zs: c.z > z_cut)
    delete_faces(obj, lambda c, _zs: c.z <= z_cut)
    capped = cap_hole(obj, z_cut - 0.06)
    print(f"SPLIT {name} head={len(head.data.polygons)} cap={capped}")
    return head


def face_variant(raw, rel, top, arm, z_cut, target, name):
    """Head of a face variant (A-pose frame): hi-res source, closed low head, maps.

    The sheet is a head-only edit of the unit's sheet: same body, so the same normalisation
    (helmet height `top`, chest centred) puts the head where head A is.
    """
    import ga3_figure_probe as probe

    obj = probe.import_figure(os.path.join(raw, rel, "multi_front_back.glb"))
    probe.drop_islands(obj)
    probe.HELMET_TOP = top
    probe.normalise(obj, arm)
    base, orm = texture_images(obj)
    rgb, cls, lum = source_maps(base, orm)
    floor = z_cut - HEAD_OVERLAP
    delete_faces(obj, lambda _c, zs: max(zs) > floor - 0.03)
    low = closed_low(obj, target, name)
    delete_faces(low, lambda _c, zs: min(zs) >= floor)
    return obj, low, rgb, cls, lum


def match_tint(rgb, cls, lum, lum_ref):
    """Scale the key-coloured texels of a head so that their mean matches the unit's."""
    import numpy as np

    out = rgb.copy()
    for channel, (own, ref) in enumerate(zip(lum, lum_ref, strict=True)):
        zone = cls[..., channel] > 0.5
        if zone.any() and own > 1e-4:
            lin = _srgb_to_linear(out[zone]) * (ref / own)
            out[zone] = np.where(
                lin <= 0.0031308,
                lin * 12.92,
                1.055 * np.clip(lin, 0.0, 1.0) ** (1 / 2.4) - 0.055,
            )
    return out


def graft_weights(head, body, z_cut):
    """Put `head` on the neck of `body` (bind pose) and copy the body's bone weights.

    Horizontal offset: neck band centroids. Weights: nearest face of the body, interpolated.
    """
    import bpy
    from mathutils import Matrix, Vector

    def band(o):
        pts = [
            o.matrix_world @ v.co
            for v in o.data.vertices
            if z_cut - 0.05 < (o.matrix_world @ v.co).z < z_cut + 0.03
        ]
        return sum(pts, Vector()) / max(1, len(pts))

    d = band(body) - band(head)
    d.z = 0.0
    # Same normalisation (chest centred): a large offset means a different collar (visor,
    # bevor), not a misplaced head; it is clamped.
    if d.length > GRAFT_MAX_OFFSET:
        d *= GRAFT_MAX_OFFSET / d.length
    head.data.transform(Matrix.Translation(d))
    for o in bpy.context.selected_objects:
        o.select_set(False)
    head.select_set(True)
    body.select_set(True)
    bpy.context.view_layer.objects.active = body
    bpy.ops.object.data_transfer(
        data_type="VGROUP_WEIGHTS",
        vert_mapping="POLYINTERP_NEAREST",
        layers_select_src="ALL",
        layers_select_dst="NAME",
        use_create=True,
    )
    weighted = sum(1 for v in head.data.vertices if v.groups) / max(
        1, len(head.data.vertices)
    )
    print(f"GRAFT {head.name} offset=({d.x:.3f}, {d.y:.3f}) weighted={weighted:.3f}")
    return d


def compose_atlas(main, heads):
    """Unit atlas: `main` (TEX²) above a strip of FACE_TEX² head squares (bottom-up rows)."""
    import numpy as np

    if not heads:
        return main
    strip = np.zeros((FACE_TEX, TEX, main.shape[2]), dtype=np.float32)
    for k, h in enumerate(heads):
        strip[:, k * FACE_TEX : (k + 1) * FACE_TEX] = h
    return np.concatenate([strip, main], axis=0)


def remap_uv(obj, k, faces):
    """UVs of `obj` into the composed atlas: k = -1 main (head A, body), else head square k."""
    uv = obj.data.uv_layers.active
    height = TEX + FACE_TEX * (1 if faces else 0)
    for d in uv.data:
        u, v = d.uv
        if k < 0:
            d.uv = (u, (TEX * v + height - TEX) / height)
        else:
            d.uv = ((k + u) * FACE_TEX / TEX, v * FACE_TEX / height)


def graft_hands(path, fine_path, bones, prefix=""):
    """LOD0: the generated mittens replaced by the closed hands of the fine figure (CAM level).

    Same rig and bind pose: the fine fists (gloves, gauntlets of the recipe) sit on the rig's
    wrists and close around the grips of the held items. Mitten = body triangles (u >= 0)
    all weighted on a wrist; fine hand = its triangles all weighted on that wrist, near the
    mitten, no variant or held piece. Returns (removed, added) triangles.
    """
    import math as m

    wrists = {
        i: side
        for i, b in enumerate(bones)
        for side in ("L", "R")
        if b == f"{prefix}Wrist.{side}"
    }
    ga3 = read_cam(path)
    fine = read_cam(fine_path)

    def on_wrist(cam, i):
        acc = {"L": 0.0, "R": 0.0}
        for b, w in zip(cam["bone"][i], cam["weight"][i], strict=True):
            side = wrists.get(int(b + 0.5))
            if side:
                acc[side] += w
        return acc

    def hand_tris(cam, threshold, keep):
        out = {"L": [], "R": []}
        for t in range(0, len(cam["idx"]), 3):
            tri = cam["idx"][t : t + 3]
            if not all(keep(i) for i in tri):
                continue
            for side in ("L", "R"):
                if all(on_wrist(cam, i)[side] >= threshold for i in tri):
                    out[side].append(t)
        return out

    mitten = hand_tris(ga3, MITTEN_MIN, lambda i: ga3["uv"][i][0] >= 0.0)
    fine_hand = hand_tris(
        fine, HAND_MIN, lambda i: int(fine["mask"][i][0] + 0.5) & 127 == 0
    )
    removed, added = set(), []
    for side in ("L", "R"):
        pts = [ga3["pos"][i] for t in mitten[side] for i in ga3["idx"][t : t + 3]]
        if not pts:
            continue
        c = [sum(p[k] for p in pts) / len(pts) for k in range(3)]
        near = [
            t
            for t in fine_hand[side]
            if all(
                m.dist(fine["pos"][i], c) < HAND_RADIUS for i in fine["idx"][t : t + 3]
            )
        ]
        if not near:
            print(f"HANDS {side}: no fine hand near the mitten, kept")
            continue
        removed.update(mitten[side])
        added.extend(near)
    out = {k: [] for k in ga3 if k != "idx"}
    out["idx"] = []
    remap = {}
    for t in range(0, len(ga3["idx"]), 3):
        if t in removed:
            continue
        for i in ga3["idx"][t : t + 3]:
            if i not in remap:
                remap[i] = len(out["pos"])
                for k in out:
                    if k != "idx" and ga3[k] is not None:
                        out[k].append(ga3[k][i])
            out["idx"].append(remap[i])
    if ga3["atlas"] is None:
        out["atlas"] = None
    kept_body = [
        out["pos"][i] for i in range(len(out["pos"])) if out["uv"][i][0] >= 0.0
    ]
    fine_remap = {}
    hand_pts = []
    for t in added:
        for i in fine["idx"][t : t + 3]:
            if i not in fine_remap:
                fine_remap[i] = len(out["pos"])
                if max(on_wrist(fine, i).values()) < 0.8:
                    hand_pts.append(fine["pos"][i])
                for k in out:
                    if k == "idx":
                        continue
                    if k == "uv":
                        out[k].append([EQUIP_UV_SHIFT * 2.0, 0.0])
                    elif k == "mask":
                        out[k].append([0.0])
                    elif k == "atlas":
                        if out["atlas"] is not None:
                            out[k].append([0.0])
                    else:
                        out[k].append(fine[k][i])
            out["idx"].append(fine_remap[i])
    write_cam(path, out)
    # Seam: the wrist end of the fine hand (vertices partly on the forearm) to the nearest
    # kept body vertex.
    gaps = [
        min(m.dist(p, q) for q in kept_body)
        for p in hand_pts[:: max(1, len(hand_pts) // 100)]
    ]
    print(
        f"HANDS removed={len(removed)} added={len(added)} "
        f"gap median={sorted(gaps)[len(gaps) // 2] if gaps else 0:.3f} "
        f"max={max(gaps) if gaps else 0:.3f}"
    )
    return len(removed), len(added)


def fine_hand_tris(fig, bones, prefix=""):
    """Triangles of the fine figure's hands at LOD0 (budget of the graft)."""
    fine = read_cam(os.path.join(FINE_DIR, f"{fig}_lod0.mesh.bin"))
    wrist = {
        i for i, b in enumerate(bones) if b in (f"{prefix}Wrist.L", f"{prefix}Wrist.R")
    }
    count = 0
    for t in range(0, len(fine["idx"]), 3):
        tri = fine["idx"][t : t + 3]
        if all(
            int(fine["mask"][i][0] + 0.5) & 127 == 0
            and sum(
                w
                for b, w in zip(fine["bone"][i], fine["weight"][i], strict=True)
                if int(b + 0.5) in wrist
            )
            >= HAND_MIN
            for i in tri
        ):
            count += 1
    return count


# --- Build --------------------------------------------------------------------------------


def build(unit_name, raw, renders, glb=None):
    """Whole chain for one unit; writes the meshes, the albedo and the manifest entry.

    L4: the face variants of ``faces`` (head-only edits of the unit's sheet) are grafted on
    the shared body as heads with variant masks (head A bit 0, variant k bit k), baked in a
    strip under the atlas; the LOD0 mittens are replaced by the fine figure's hands.
    """
    import battle_fine as bf
    import battle_fine_proto as fp
    import battle_skinned as bs
    import bpy
    import ga3_figure_probe as probe
    from mathutils import Vector

    unit = UNITS[unit_name]
    fig = unit["figure"]
    mounted = unit.get("mounted", False)
    cap = RIDER_CAP if mounted else TRI_CAP
    faces = unit.get("faces", [])
    with open(os.path.join(FINE_DIR, "manifest.json")) as f:
        fine = json.load(f)
    if 1 + len(faces) > int(fine["figures"][fig].get("variants", 1)):
        raise SystemExit(f"GA3: more heads than fine variants for {fig}")
    if len(faces) > TEX // FACE_TEX:
        raise SystemExit("GA3: at most two variant heads (atlas strip)")
    os.makedirs(OUT_DIR, exist_ok=True)
    seat = None
    if mounted:
        import battle_skinned_cavalry as cav

        # Rest of the fine rider on the saddle (``cavalry`` rig): the whole chain runs on
        # the human rig, the rider is moved there at export (same armature data).
        bf.use_fine_mount()
        r_rest = cav.Mount().r_rest.copy()
    arm, _ = bf.load_fine_human()
    if mounted:
        seat = r_rest @ arm.matrix_world.inverted()
    fp.prime_virtuals(arm)
    top = unit_top(unit_name, raw)
    z_cut = neck_cut(arm)
    rig_name = "cavalry" if mounted else "human"
    bones = fine["rigs"][rig_name]["bones"]
    prefix = "R:" if mounted else ""
    hands = fine_hand_tris(fig, bones, prefix)
    print(f"UNIT {unit_name} top={top:.3f} neck cut={z_cut:.3f} fine hands={hands}")
    # Variant heads first (their triangles come out of the body budget).
    heads = []
    for k, rel in enumerate(faces):
        own_top = (
            top
            * figure_height(raw, rel)
            / figure_height(raw, os.path.dirname(unit["glb"]))
        )
        hi, low, rgb, cls, lum = face_variant(
            raw, rel, own_top, arm, z_cut, HEAD_TRIS[0], f"{fig}_ga3_head{k + 1}"
        )
        heads.append({"hi": hi, "low": low, "rgb": rgb, "cls": cls, "lum": lum})
        print(f"HEAD {k + 1} {rel} top={own_top:.3f} tris={probe.tris(low)}")
    glb = glb or unit["glb"]
    probe.HELMET_TOP = top
    obj = probe.import_figure(os.path.join(raw, glb))
    raw_tris = probe.tris(obj)
    islands, dropped = probe.drop_islands(obj)
    scale = probe.normalise(obj, arm)
    print(f"RAW tris={raw_tris} islands={islands} dropped={dropped} scale={scale:.3f}")
    base, orm = texture_images(obj)
    src_rgb, src_cls, lum = source_maps(base, orm)
    src_albedo = image_from(src_rgb, "ga3_src_albedo")
    src_classes = image_from(src_cls, "ga3_src_classes", data=True)
    for k, h in enumerate(heads):
        h_rgb = match_tint(h["rgb"], h["cls"], h["lum"], lum)
        h["baked"], h["classes"] = bake_low(
            h["hi"],
            h["low"],
            image_from(h_rgb, f"ga3_src_albedo_h{k}"),
            image_from(h["cls"], f"ga3_src_classes_h{k}", data=True),
            FACE_TEX,
        )
        bpy.data.objects.remove(h["hi"])
    # LOD0 budget: the cap minus the level-0 equipment (measured on a throwaway build), the
    # fine hands and the variant heads.
    probe_gear = equipment(unit, arm, 0)
    eq_tris = tris(probe_gear)
    for o in probe_gear:
        bpy.data.objects.remove(o)
    head_tris = sum(tris([h["low"]]) for h in heads)
    print(f"EQUIP level-0 tris={eq_tris} heads={head_tris}")
    low = closed_low(
        obj, cap[0] - eq_tris - hands // 3 - head_tris - 100, f"{fig}_ga3_body"
    )
    baked, classes = bake_low(obj, low, src_albedo, src_classes)
    bpy.data.objects.remove(obj)
    obj = low
    ok, limbs = skin_to_bind(obj, arm)
    print(f"ARM leaks in the bind pose: {strip_arm_leaks(obj)}")
    print("HEAT_OK", ok, {k: tuple(round(x, 3) for x in v) for k, v in limbs.items()})
    if not ok:
        raise SystemExit("GA3: bone heat failed")
    for h in heads:
        graft_weights(h["low"], obj, z_cut)
        fp.attach(h["low"], arm)
    tint = (classes[..., 0] > 0.5) | (classes[..., 1] > 0.5)
    rgba = baked.copy()
    rgba[..., 3] = tint.astype("float32")
    head_rgba = []
    for h in heads:
        a = h["baked"].copy()
        a[..., 3] = (
            (h["classes"][..., 0] > 0.5) | (h["classes"][..., 1] > 0.5)
        ).astype("float32")
        head_rgba.append(a)
    atlas = compose_atlas(rgba, head_rgba)
    atlas_classes = compose_atlas(classes, [h["classes"] for h in heads])
    albedo_name = f"{fig}_albedo.png"
    albedo = save_png(atlas, os.path.join(OUT_DIR, albedo_name), albedo_name)
    mats = coded_materials(albedo)
    code_faces(obj, classes[..., :3], mats, unit.get("metal_z", METAL_Z))
    bs.set_face_mask(obj, 0)
    obj.name = f"{fig}_ga3_body"
    for k, h in enumerate(heads):
        code_faces(h["low"], h["classes"][..., :3], mats, unit.get("metal_z", METAL_Z))
        bs.set_face_mask(h["low"], 1 << (k + 1))
        remap_uv(h["low"], k, faces)
    remap_uv(obj, -1, faces)
    rig = bs.rig_stub(rig_name, bones)
    for pb in arm.pose.bones:
        pb.matrix_basis.identity()
    bpy.context.view_layer.update()
    # Level bodies (from the whole figure A, before its head is split off), then the heads.
    levels = {0: obj}
    for level in (1, 2):
        equipment_probe = equipment(unit, arm, level)
        g_tris = tris(equipment_probe)
        for o in equipment_probe:
            bpy.data.objects.remove(o)
        # Level 1: the variant heads and the neck cap (split below).
        extra = len(heads) * HEAD_TRIS[1] + 40 if level == 1 else 0
        levels[level] = lod_copy(
            obj, cap[level] - g_tris - extra, f"{fig}_ga3_lod{level}"
        )
    pieces = {0: [], 1: [], 2: []}
    if heads:
        for level in (0, 1):
            head_a = split_head(levels[level], z_cut, f"{fig}_ga3_headA_lod{level}")
            bs.set_face_mask(head_a, 1)
            pieces[level].append(head_a)
            for k, h in enumerate(heads):
                piece = (
                    h["low"]
                    if level == 0
                    else lod_copy(h["low"], HEAD_TRIS[1], f"{fig}_ga3_head{k + 1}_lod1")
                )
                pieces[level].append(piece)
    files, counts = [], []
    kept = {}
    for level in range(3):
        gear_objs = equipment(unit, arm, level)
        body_obj = levels[level]
        name = f"{fig}_lod{level}.mesh.bin"
        objs = [body_obj, *pieces[level], *gear_objs]
        path = os.path.join(OUT_DIR, name)
        if mounted:
            counts.append(export_mounted(objs, seat, rig, bones, level, path, fig))
        else:
            bs.export_mesh(
                objs, rig, bs.human_bone_alias, path, influences=bs.INFLUENCES[level]
            )
            if level == 0:
                graft_hands(path, os.path.join(FINE_DIR, name), bones)
            counts.append(len(read_cam(path)["idx"]) // 3)
            if counts[-1] > cap[level]:
                raise SystemExit(f"GA3: LOD{level} {counts[-1]} > cap {cap[level]}")
        files.append(name)
        kept[level] = (body_obj, [*pieces[level], *gear_objs])
    head_y = (seat @ Vector((0.0, 0.0, z_cut))).z if mounted else z_cut
    entry = {
        "lods": files,
        "tris": counts,
        "variants": 1 + len(heads),
        "head_y": round(head_y, 4),
        "ga3_albedo": albedo_name,
        "ga3_lum": [round(lum[0], 4), round(lum[1], 4)],
        "unit": unit_name,
        **({"fine_horse": True} if mounted else {}),
        "source": f"fal-ai/nano-banana-2/edit + {os.path.basename(glb)} (tools/experiments/ga3_fal_figure.py)",
        "faces": [os.path.basename(rel) for rel in faces],
    }
    path = os.path.join(OUT_DIR, "manifest.json")
    manifest = {"figures": {}}
    if os.path.exists(path):
        with open(path) as f:
            manifest = json.load(f)
    manifest["figures"][fig] = entry
    manifest["source"] = "tools/blender_scripts/ga3_figures.py (GA3-L3/L4, ADR 0140)"
    with open(path, "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
    print("ENTRY", json.dumps(entry))
    out_dir = os.path.join(raw, L4_RAW, unit_name)
    os.makedirs(out_dir, exist_ok=True)
    if renders:
        tag = os.path.splitext(os.path.basename(glb))[0]
        render_views(
            unit_name,
            tag,
            os.path.join(raw, L4_RAW),
            arm,
            kept,
            atlas,
            atlas_classes,
            lum,
        )
    bpy.ops.wm.save_as_mainfile(
        filepath=os.path.join(out_dir, f"ga3_{unit_name}.blend")
    )
    print("OK")


# --- Mounted figures (L3c) ----------------------------------------------------------------


def export_mounted(objs, seat, rig, bones, level, path, fig=None):
    """Rider `objs` moved onto the saddle (`seat`), merged with the fine horse; triangles.

    The rider pieces are copied into the ``cavalry`` rest frame with their groups renamed
    ``R:`` (``battle_skinned_cavalry._rename_groups``) and exported alone; the horse, its
    harness and bards come from the exported fine figure (``merge_horse``).
    """
    import battle_skinned as bs
    import battle_skinned_cavalry as cav
    import bpy

    copies = []
    for o in objs:
        me = o.data.copy()
        me.transform(seat @ o.matrix_world)
        c = bpy.data.objects.new(o.name + "_rider", me)
        bpy.context.scene.collection.objects.link(c)
        # Mesh copies carry the group names (Blender >= 3): add them only if missing.
        if len(c.vertex_groups) == 0:
            for g in o.vertex_groups:
                c.vertex_groups.new(name=g.name)
        cav._rename_groups(c, "R:")
        copies.append(c)
    tmp = path + ".rider"
    rider = bs.export_mesh(
        copies, rig, cav.cavalry_alias, tmp, influences=bs.INFLUENCES[level]
    )
    for c in copies:
        bpy.data.objects.remove(c)
    fine = os.path.join(FINE_DIR, os.path.basename(path))
    if level == 0:
        # L4: the fine rider's closed hands (same fine figure file, ``R:`` wrists).
        graft_hands(tmp, fine, bones, "R:")
        rider = len(read_cam(tmp)["idx"]) // 3
    horse = merge_horse(fine, tmp, path, bones)
    os.remove(tmp)
    print(f"MOUNTED lod{level} rider={rider} horse={horse}")
    return rider + horse


def read_cam(path):
    """Arrays of a ``CAM1`` / ``CAM2`` file (``battle_skinned.export_mesh``)."""
    import struct
    import zlib

    with open(path, "rb") as f:
        data = f.read()
    n, m, _size = struct.unpack("<III", data[4:16])
    raw = zlib.decompress(data[16:])
    atlas = data[:4] == b"CAM2"
    per = 22 if atlas else 21
    floats = struct.unpack(f"<{n * per}f", raw[: n * per * 4])
    idx = list(struct.unpack(f"<{m}I", raw[n * per * 4 : n * per * 4 + m * 4]))
    out, offset = {}, 0
    for key, width in (
        ("pos", 3),
        ("nor", 3),
        ("col", 4),
        ("uv", 2),
        ("bone", 4),
        ("weight", 4),
        ("mask", 1),
        ("atlas", 1),
    ):
        if key == "atlas" and not atlas:
            out[key] = None
            break
        out[key] = [
            list(floats[offset + i * width : offset + (i + 1) * width])
            for i in range(n)
        ]
        offset += n * width
    out["idx"] = idx
    return out


def write_cam(path, cam):
    """Write `cam` (``read_cam`` layout) as ``CAM2`` when it carries atlas UVs."""
    import struct
    import zlib

    n = len(cam["pos"])
    raw = bytearray()
    keys = ["pos", "nor", "col", "uv", "bone", "weight", "mask"]
    if cam["atlas"] is not None:
        keys.append("atlas")
    for key in keys:
        for item in cam[key]:
            raw += struct.pack(f"<{len(item)}f", *item)
    raw += struct.pack(f"<{len(cam['idx'])}I", *cam["idx"])
    with open(path, "wb") as f:
        f.write(b"CAM2" if cam["atlas"] is not None else b"CAM1")
        f.write(struct.pack("<III", n, len(cam["idx"]), len(raw)))
        f.write(zlib.compress(bytes(raw), 9))


def merge_horse(fine_path, rider_path, out_path, bones):
    """Fine horse of `fine_path` (every triangle on horse bones only) plus the rider file.

    The horse keeps its FG3 atlas UVs (coat, caparison: ``atlas_layer`` of the fine figure)
    and its UVs move to negative u (untextured by ``GA3_TEX``) except on the ``C_ARMS``
    faces (caparison arms). The rider's vertices carry no atlas UV (source 0: no FG3 read).
    Returns the horse triangle count.
    """
    horse_bones = {i for i, b in enumerate(bones) if not b.startswith("R:")}
    fine = read_cam(fine_path)
    rider = read_cam(rider_path)
    on_horse = [
        all(int(b + 0.5) in horse_bones for b, w in zip(bs_, ws, strict=True) if w > 0)
        for bs_, ws in zip(fine["bone"], fine["weight"], strict=True)
    ]
    remap, out = {}, {k: [] for k in fine if k != "idx"}
    out["idx"] = []
    kept = 0
    for t in range(0, len(fine["idx"]), 3):
        tri = fine["idx"][t : t + 3]
        if not all(on_horse[i] for i in tri):
            continue
        kept += 1
        for i in tri:
            if i not in remap:
                remap[i] = len(out["pos"])
                for k in out:
                    if k == "idx" or fine[k] is None:
                        continue
                    item = list(fine[k][i])
                    if k == "uv" and int(fine["col"][i][3] + 0.5) != C_ARMS:
                        item[0] += EQUIP_UV_SHIFT
                    out[k].append(item)
            out["idx"].append(remap[i])
    if fine["atlas"] is None:
        out["atlas"] = None
    base = len(out["pos"])
    for k in out:
        if k == "idx":
            continue
        if k == "atlas":
            if out["atlas"] is not None:
                out["atlas"].extend([[0.0]] * len(rider["pos"]))
            continue
        out[k].extend(rider[k])
    out["idx"].extend(i + base for i in rider["idx"])
    write_cam(out_path, out)
    return kept


# --- Renders and sheet --------------------------------------------------------------------

# (file label, clip role of the unit's ``clips``, fraction, eye, target). L3a names kept.
SHOTS = [
    ("bind", None, 0.0, (0.9, -3.3, 1.2), (0.45, 0, 0.95)),
    ("walk", "walk", 0.25, (0.9, -3.3, 1.2), (0.45, 0, 0.95)),
    ("shoot", "attack", 0.5, (1.7, -2.9, 1.5), (0.45, 0, 1.1)),
    ("back", "walk", 0.6, (0.2, 3.4, 1.3), (0.45, 0, 0.95)),
]
# L3c: rider on the horse (rest, charge, lance thrust, back), the current figure at +X.
MOUNTED_SHOTS = [
    ("bind", None, 0.0, (3.2, -5.2, 2.4), (0.9, 0, 1.5)),
    ("walk", "walk", 0.3, (3.2, -5.2, 2.4), (0.9, 0, 1.5)),
    ("shoot", "attack", 0.5, (4.2, -3.8, 2.6), (0.9, 0, 1.6)),
    ("back", "walk", 0.6, (1.4, 5.6, 2.4), (0.9, 0, 1.5)),
]
MOUNTED_OFFSET = 1.8  # m along +X between the generated and the current mounted figure


def cam_uvs(path):
    """Per-vertex UVs of an exported ``CAM1`` / ``CAM2`` file (Blender convention)."""
    import struct
    import zlib

    with open(path, "rb") as f:
        data = f.read()
    n = struct.unpack("<I", data[4:8])[0]
    raw = zlib.decompress(data[16:])
    floats = struct.unpack(f"<{n * 2}f", raw[n * 10 * 4 : n * 12 * 4])
    return [floats[2 * i : 2 * i + 2] for i in range(n)]


def game_posed(fig, clip, frac, idle, image, rig_name="human", variant=0):
    """The exported GA3 LOD0 of `fig` skinned on the CPU with the game's bone texture.

    Same frames as the current figure beside it (``add_current_archer``): the held items
    follow the virtual bones (``Prop``) exactly as in the game. Body faces carry the preview
    albedo, equipment faces (u < 0, ``C_ARMS``) their vertex colour.
    """
    import battle_fine_check as fc
    import bpy
    import ga3_figure_probe as probe

    with open(os.path.join(FINE_DIR, "manifest.json")) as f:
        rig = json.load(f)["rigs"][rig_name]
    frames = fc.load_bones(os.path.join(FINE_DIR, rig["texture"]))
    path = os.path.join(OUT_DIR, f"{fig}_lod0.mesh.bin")
    *mesh, _atlas = probe.load_cam(path)
    uvs = cam_uvs(path)
    c = rig["clips"][clip or idle]
    fr = c["start"] + int(round((frac if clip else 0.0) * (c["frames"] - 1)))
    obj = fc.skinned_object(f"{fig}_ga3_game{variant}", mesh, frames[fr], variant)
    me = obj.data
    layer = me.uv_layers.new(name="UVMap")
    colours = mesh[1]
    body = bpy.data.materials.new("ga3_game_body")
    body.use_nodes = True
    tex = body.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = image
    body.node_tree.links.new(
        tex.outputs[0], body.node_tree.nodes["Principled BSDF"].inputs["Base Color"]
    )
    body.node_tree.nodes["Principled BSDF"].inputs["Roughness"].default_value = 0.8
    me.materials.append(body)
    me.materials.append(probe._vertex_colour_material())
    for poly in me.polygons:
        gear_face = False
        for li in poly.loop_indices:
            vi = me.loops[li].vertex_index
            layer.data[li].uv = uvs[vi]
            code = int(colours[vi][3] + 0.5)  # CAM colour alpha = raw code
            gear_face = gear_face or uvs[vi][0] < 0.0 or code == C_ARMS
        poly.material_index = 1 if gear_face else 0
    return obj


def render_views(unit_name, tag, raw, arm, kept, rgba, classes, lum):
    """Exported LOD0 beside the current fine figure, both in the game's poses (L3b).

    Game-like tint of the albedo; the Blender-posed objects stay hidden (their virtual
    bones do not follow the clips' overrides).
    """
    import battle_fine_proto as fp
    import bpy
    import ga3_figure_probe as probe
    import numpy as np

    out = os.path.join(raw, unit_name, "renders_" + tag)
    os.makedirs(out, exist_ok=True)
    for level in (1, 2):
        body_obj, gear_objs = kept[level]
        for o in [body_obj, *gear_objs]:
            o.hide_render = True
    lin = preview_rgba(rgba, classes, lum)
    srgb = np.where(
        lin <= 0.0031308, lin * 12.92, 1.055 * np.clip(lin, 0, 1) ** (1 / 2.4) - 0.055
    )
    prev = np.concatenate([np.clip(srgb, 0, 1), np.ones_like(srgb[..., :1])], axis=-1)
    img = bpy.data.images.new("ga3_preview", prev.shape[1], prev.shape[0], alpha=True)
    img.pixels.foreach_set(prev.astype(np.float32).reshape(-1))
    for mat in bpy.data.materials:
        if mat.name.startswith("ga3_") and mat.node_tree:
            for n in mat.node_tree.nodes:
                if n.type == "TEX_IMAGE":
                    n.image = img
    scene = probe.setup_eevee()
    cam = fp.camera()
    for body_obj, gear_objs in kept.values():
        for o in [body_obj, *gear_objs]:
            o.hide_render = True
    unit = UNITS[unit_name]
    fig = unit["figure"]
    clips = unit["clips"]
    mounted = unit.get("mounted", False)
    rig_name = "cavalry" if mounted else "human"
    for label, role, frac, eye, target in MOUNTED_SHOTS if mounted else SHOTS:
        clip = clips[role] if role else None
        for o in [
            o
            for o in bpy.data.objects
            if o.name.startswith(fig) and ("ga3" not in o.name or "_ga3_game" in o.name)
        ]:
            bpy.data.objects.remove(o)
        for pb in arm.pose.bones:
            pb.matrix_basis.identity()
        if clip and not mounted:
            fp.pose_clip(arm, clip, frac)
        bpy.context.view_layer.update()
        probe.add_current_archer(
            clip,
            frac,
            fig,
            clips["idle"],
            rig_name,
            MOUNTED_OFFSET if mounted else None,
        )
        game_posed(fig, clip, frac, clips["idle"], img, rig_name)
        fp.look_at(cam, eye, target, 40)
        scene.render.resolution_x, scene.render.resolution_y = (
            (760, 620) if mounted else (560, 620)
        )
        fp.render(os.path.join(out, f"{label}.png"))
    # L4: the heads of every variant side by side (rest pose, close-up, LOD0 as in game).
    for o in [o for o in bpy.data.objects if o.name.startswith(fig)]:
        if "ga3" not in o.name or "_ga3_game" in o.name:
            bpy.data.objects.remove(o)
    count = 1 + len(unit.get("faces", []))
    neutral = "c_idle" if mounted else "idle"

    def body_points(o):
        # Generated body faces only (material 0): not the weapon raised above the head.
        me = o.data
        return [
            me.vertices[i].co
            for p in me.polygons
            if p.material_index == 0
            for i in p.vertices
        ]

    posed = []
    for v in range(count):
        o = game_posed(fig, neutral, 0.0, neutral, img, rig_name, v)
        o.location.x += 0.6 * v
        posed.append(o)
    pts = body_points(posed[0])
    head_z = max(p.z for p in pts) - 0.17
    xs = [p.x for p in pts if p.z > head_z]
    x0 = 0.5 * (min(xs) + max(xs))
    cx = x0 + 0.3 * (count - 1)
    fp.look_at(cam, (cx + 0.25, -2.4, head_z + 0.12), (cx, 0.0, head_z - 0.08), 50)
    scene.render.resolution_x, scene.render.resolution_y = (560, 420)
    fp.render(os.path.join(out, "faces.png"))
    # Hands of variant 0 (fine fists grafted at LOD0) in the weapon's rest clip.
    for o in posed:
        bpy.data.objects.remove(o)
    game_posed(fig, None, 0.0, clips["idle"], img, rig_name, 0)
    hand_z = head_z - 0.5
    fp.look_at(cam, (x0 + 0.35, -1.8, hand_z + 0.2), (x0, 0.0, hand_z), 35)
    fp.render(os.path.join(out, "hands.png"))
    _ = math


def sheet(unit_name, jpg, raw, tags):
    """Planche: reference A-pose | rigged ``tags[0]`` beside the current figure.

    Rows: reference and bind pose; walk, shoot and back; the walk of every generator in
    ``tags``.
    """
    from PIL import Image, ImageDraw, ImageFont

    unit = UNITS[unit_name]
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 15)
    except OSError:
        font = ImageFont.load_default()
    h = 420

    def tile(tag, name, label):
        path = os.path.join(raw, unit_name, "renders_" + tag, name + ".png")
        im = Image.open(path).convert("RGB")
        return label, im.resize((int(im.width * h / im.height), h))

    ref = Image.open(os.path.join(raw, unit["reference"])).convert("RGB")
    main_tag = tags[0]
    rows = [
        [
            (
                "Référence A-pose (NB2)",
                ref.resize((int(ref.width * h / ref.height), h)),
            ),
            tile(main_tag, "bind", f"{main_tag} liaison | {unit['figure']} actuel"),
        ],
        [
            tile(main_tag, "walk", unit["clips"]["walk"]),
            tile(main_tag, "shoot", unit["clips"]["attack"]),
            tile(main_tag, "back", f"dos ({unit['clips']['walk']})"),
        ],
    ]
    if len(tags) > 1:
        rows.append([tile(t, "walk", t) for t in tags])
    width = max(sum(t.width for _, t in r) for r in rows)
    canvas = Image.new("RGB", (width, len(rows) * (h + 26)), (30, 30, 32))
    y = 0
    for r in rows:
        x = 0
        for label, t in r:
            canvas.paste(t, (x, y + 26))
            ImageDraw.Draw(canvas).text(
                (x + 6, y + 5), label, fill=(235, 230, 215), font=font
            )
            x += t.width
        y += h + 26
    if width > 1600:
        canvas = canvas.resize((1600, int(canvas.height * 1600 / width)))
    os.makedirs(os.path.dirname(jpg), exist_ok=True)
    canvas.save(jpg, quality=86)
    print("SHEET", jpg, canvas.size)


def board(jpg, raw, names, tag="multi_front_back", per_row=8):
    """Planche of several units (L4): walk (generated | current) above the variant heads.

    `per_row` units per band; renders under ``raw/l4/<unit>/``.
    """
    from PIL import Image, ImageDraw, ImageFont

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 14)
    except OSError:
        font = ImageFont.load_default()
    w = 300
    tiles = []
    for name in names:
        unit = UNITS[name]
        folder = os.path.join(raw, L4_RAW, name, "renders_" + tag)
        walk = Image.open(os.path.join(folder, "walk.png")).convert("RGB")
        faces = Image.open(os.path.join(folder, "faces.png")).convert("RGB")
        hands = Image.open(os.path.join(folder, "hands.png")).convert("RGB")
        hands = hands.resize((w, int(hands.height * w / hands.width)))
        walk = walk.resize((w, int(walk.height * w / walk.width)))
        faces = faces.resize((w, int(faces.height * w / faces.width)))
        tile = Image.new(
            "RGB", (w, 22 + walk.height + faces.height + hands.height), (30, 30, 32)
        )
        tile.paste(walk, (0, 22))
        tile.paste(faces, (0, 22 + walk.height))
        tile.paste(hands, (0, 22 + walk.height + faces.height))
        ImageDraw.Draw(tile).text(
            (5, 4), f"{unit['figure']} ({name})", fill=(235, 230, 215), font=font
        )
        tiles.append(tile)
    bands = [tiles[k : k + per_row] for k in range(0, len(tiles), per_row)]
    band_h = [max(t.height for t in b) for b in bands]
    canvas = Image.new("RGB", (w * min(per_row, len(tiles)), sum(band_h)), (30, 30, 32))
    y = 0
    for b, bh in zip(bands, band_h, strict=True):
        for k, t in enumerate(b):
            canvas.paste(t, (k * w, y))
        y += bh
    if canvas.width > 1800:
        canvas = canvas.resize((1800, int(canvas.height * 1800 / canvas.width)))
    os.makedirs(os.path.dirname(jpg), exist_ok=True)
    canvas.save(jpg, quality=84)
    print("BOARD", jpg, canvas.size)


def main():
    """Parse the Blender, sheet or board command line."""
    if len(sys.argv) > 1 and sys.argv[1] == "board":
        args = sys.argv[2:]
        raw = args[args.index("--raw") + 1] if "--raw" in args else RAW
        units = (
            args[args.index("--units") + 1].split(",")
            if "--units" in args
            else list(UNITS)
        )
        board(args[0], os.path.expanduser(raw), units)
        return
    if len(sys.argv) > 1 and sys.argv[1] == "sheet":
        args = sys.argv[2:]
        raw = args[args.index("--raw") + 1] if "--raw" in args else RAW
        tags = (
            args[args.index("--tags") + 1].split(",")
            if "--tags" in args
            else ["multi_front_back"]
        )
        sheet(args[0], args[1], os.path.expanduser(raw), tags)
        return
    args = sys.argv[sys.argv.index("--") + 1 :]
    raw = args[args.index("--raw") + 1] if "--raw" in args else RAW
    glb = args[args.index("--glb") + 1] if "--glb" in args else None
    build(args[0], os.path.expanduser(raw), "--renders" in args, glb)


if __name__ == "__main__":
    main()
