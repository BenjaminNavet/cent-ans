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
OUT_DIR = os.path.join(ROOT, "game", "assets", "models", "battle_ga3")
FINE_DIR = os.path.join(ROOT, "game", "assets", "models", "battle_fine")
RAW = os.path.expanduser("~/dev/cent-ans-raw/ga3/l3")

# Per unit: figure replaced, source folder, helmet top (m), held equipment builders
# (``battle_fine_weapons``), hue bands (degrees) of the key colours.
UNITS = {
    "longbowman": {
        "figure": "archer_0",
        "glb": "longbowman/multi_front_back.glb",
        "reference": "longbowman/sheet.png",
        "top": 1.80,
        "equipment": ["longbow"],
    },
}
KEY_LIVERY = (75.0, 170.0)  # saturated green
KEY_CLOTH = (190.0, 265.0)  # saturated blue
KEY_SAT = 0.28
CLOSE_STEPS = (
    8  # px at the 2048 source: dirt spots inside a key colour zone are tinted too
)
METAL_Z = 1.5  # m: steel code on the helmet only (dagger, buckles keep their texture)
TRI_CAP = (11900, 1350, 260)  # battle_fine_figures.TRI_CAP (foot soldiers)
TEX = 1024
EQUIP_TRIS = {"longbowman": 310}  # level-0 equipment triangles (measured)
AUTO_LEVELS = 0.5  # percent clipped at each end (ga3_cleanup, L1)
EQUIP_UV_SHIFT = -4.0
SOLIDIFY = 0.012  # m, thickens the open TRELLIS sheets before the voxel remesh
VOXEL = 0.008  # m
# Material codes (battle_skinned_equipment / battle_soldier_skinned.gdshader).
C_LIVERY, C_PLATE, C_CLOTH, C_EXACT = 0, 2, 4, 5


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


def code_faces(obj, classes, mats):
    """Replace the TRELLIS material by the coded ones, per face class."""
    import numpy as np

    fc = face_classes(obj, classes)
    for i, poly in enumerate(obj.data.polygons):
        if fc[i] == 3 and (obj.matrix_world @ poly.center).z < METAL_Z:
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
    print(f"HEAT direct weighted={share:.3f}")
    return share


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
    islands, dropped = probe.drop_islands(low, keep_ratio=0.3)
    dense = len(low.data.polygons)
    for _ in range(4):
        if len(low.data.polygons) <= target * 1.02:
            break
        bs.weld_and_decimate(low, target)
    print(
        f"LOW voxel={dense} islands={islands} dropped={dropped} -> {len(low.data.polygons)}"
    )
    return low


def bake_low(hi, low, albedo, classes):
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
        target = bpy.data.images.new(f"ga3_low_{name}", TEX, TEX, alpha=False)
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
    print("BAKE albedo + classes", TEX)
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


def equipment(unit, arm, level):
    """Held weapons of the unit at `level` (procedural, bound to the rig's bones)."""
    import battle_fine_gear as gear
    import battle_fine_weapons as fw
    import battle_skinned as bs
    import battle_skinned_equipment as eq

    ctx = eq.Context(arm, level, bs.material, bs.bone_world)
    out = []
    for name in unit["equipment"]:
        objs = getattr(fw, name)(gear.Gear(ctx, None, [], "ga3_" + name, False))
        for obj in objs:
            gear.unwrap(obj)
            uv = obj.data.uv_layers.active
            if uv is not None:
                for d in uv.data:
                    d.uv = (d.uv[0] + EQUIP_UV_SHIFT, d.uv[1])
            bs.set_face_mask(obj, bs.held_mask(name, 0, {}))
            out.append(obj)
    return out


def tris(objs):
    """Triangle count of objects."""
    total = 0
    for o in objs:
        o.data.calc_loop_triangles()
        total += len(o.data.loop_triangles)
    return total


# --- Build --------------------------------------------------------------------------------


def build(unit_name, raw, renders, glb=None):
    """Whole chain for one unit; writes the meshes, the albedo and the manifest entry."""
    import battle_fine as bf
    import battle_fine_proto as fp
    import battle_skinned as bs
    import bpy
    import ga3_figure_probe as probe

    unit = UNITS[unit_name]
    fig = unit["figure"]
    os.makedirs(OUT_DIR, exist_ok=True)
    arm, _ = bf.load_fine_human()
    fp.prime_virtuals(arm)
    probe.HELMET_TOP = unit["top"]
    glb = glb or unit["glb"]
    obj = probe.import_figure(os.path.join(raw, glb))
    raw_tris = probe.tris(obj)
    islands, dropped = probe.drop_islands(obj)
    scale = probe.normalise(obj, arm)
    print(f"RAW tris={raw_tris} islands={islands} dropped={dropped} scale={scale:.3f}")
    base, orm = texture_images(obj)
    src_rgb, src_cls, lum = source_maps(base, orm)
    src_albedo = image_from(src_rgb, "ga3_src_albedo")
    src_classes = image_from(src_cls, "ga3_src_classes", data=True)
    # LOD0 budget: the cap minus the level-0 equipment (longbow, string, arrow: ~310).
    eq_tris = EQUIP_TRIS.get(unit_name, 400)
    low = closed_low(obj, TRI_CAP[0] - eq_tris - 100, f"{fig}_ga3_body")
    baked, classes = bake_low(obj, low, src_albedo, src_classes)
    bpy.data.objects.remove(obj)
    obj = low
    ok, limbs = skin_to_bind(obj, arm)
    print("HEAT_OK", ok, {k: tuple(round(x, 3) for x in v) for k, v in limbs.items()})
    if not ok:
        raise SystemExit("GA3: bone heat failed")
    tint = (classes[..., 0] > 0.5) | (classes[..., 1] > 0.5)
    rgba = baked.copy()
    rgba[..., 3] = tint.astype("float32")
    albedo_name = f"{fig}_albedo.png"
    albedo = save_png(rgba, os.path.join(OUT_DIR, albedo_name), albedo_name)
    mats = coded_materials(albedo)
    code_faces(obj, classes[..., :3], mats)
    bs.set_face_mask(obj, 0)
    obj.name = f"{fig}_ga3_body"
    with open(os.path.join(FINE_DIR, "manifest.json")) as f:
        fine = json.load(f)
    rig = bs.rig_stub("human", fine["rigs"]["human"]["bones"])
    for pb in arm.pose.bones:
        pb.matrix_basis.identity()
    bpy.context.view_layer.update()
    files, counts = [], []
    kept = {}
    for level in range(3):
        gear_objs = equipment(unit, arm, level)
        g_tris = tris(gear_objs)
        if level == 0:
            body_obj = obj
        else:
            body_obj = lod_copy(obj, TRI_CAP[level] - g_tris, f"{fig}_ga3_lod{level}")
        name = f"{fig}_lod{level}.mesh.bin"
        counts.append(
            bs.export_mesh(
                [body_obj, *gear_objs],
                rig,
                bs.human_bone_alias,
                os.path.join(OUT_DIR, name),
                influences=bs.INFLUENCES[level],
            )
        )
        files.append(name)
        kept[level] = (body_obj, gear_objs)
    entry = {
        "lods": files,
        "tris": counts,
        "variants": 1,
        "ga3_albedo": albedo_name,
        "ga3_lum": [round(lum[0], 4), round(lum[1], 4)],
        "unit": unit_name,
        "source": f"fal-ai/nano-banana-2/edit + {os.path.basename(glb)} (tools/experiments/ga3_fal_figure.py)",
    }
    path = os.path.join(OUT_DIR, "manifest.json")
    manifest = {"figures": {}}
    if os.path.exists(path):
        with open(path) as f:
            manifest = json.load(f)
    manifest["figures"][fig] = entry
    manifest["source"] = "tools/blender_scripts/ga3_figures.py (GA3-L3, ADR 0140)"
    with open(path, "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True)
    print("ENTRY", json.dumps(entry))
    if renders:
        tag = os.path.splitext(os.path.basename(glb))[0]
        render_views(unit_name, tag, raw, arm, kept, rgba, classes, lum)
    bpy.ops.wm.save_as_mainfile(
        filepath=os.path.join(raw, unit_name, f"ga3_{unit_name}.blend")
    )
    print("OK")


# --- Renders and sheet --------------------------------------------------------------------

SHOTS = [
    ("bind", None, 0.0, (0.9, -3.3, 1.2), (0.45, 0, 0.95)),
    ("walk", "bow_walk", 0.25, (0.9, -3.3, 1.2), (0.45, 0, 0.95)),
    ("shoot", "bow_shoot", 0.5, (1.7, -2.9, 1.5), (0.45, 0, 1.1)),
    ("back", "bow_walk", 0.6, (0.2, 3.4, 1.3), (0.45, 0, 0.95)),
]


def render_views(unit_name, tag, raw, arm, kept, rgba, classes, lum):
    """LOD0 beside the current fine figure (probe renders), game-like tint of the albedo."""
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
    for o in kept[0][1]:
        fp.attach(o, arm)
    for label, clip, frac, eye, target in SHOTS:
        for o in [
            o
            for o in bpy.data.objects
            if o.name.startswith("archer_0") and "ga3" not in o.name
        ]:
            bpy.data.objects.remove(o)
        for pb in arm.pose.bones:
            pb.matrix_basis.identity()
        if clip:
            fp.pose_clip(arm, clip, frac)
        bpy.context.view_layer.update()
        probe.add_current_archer(clip, frac)
        fp.look_at(cam, eye, target, 40)
        scene.render.resolution_x, scene.render.resolution_y = (560, 620)
        fp.render(os.path.join(out, f"{label}.png"))
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
            tile(main_tag, "bind", f"{main_tag} liaison | archer_0 actuel"),
        ],
        [
            tile(main_tag, "walk", "bow_walk"),
            tile(main_tag, "shoot", "bow_shoot"),
            tile(main_tag, "back", "dos (bow_walk)"),
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


def main():
    """Parse the Blender or sheet command line."""
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
