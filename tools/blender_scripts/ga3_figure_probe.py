"""GA3-S2 probe: can an image-to-3D figure (fal TRELLIS) replace an animated battle soldier?

The generated longbowman (one merged, textured mesh) is cleaned, decimated to the LOD0 budget
of the fine foot soldiers (~12 k triangles, ``battle_fine`` manifest), scaled and placed on
the fine ``human`` rig (``battle_fine.load_fine_human``), its bow cut out and moved to the
left-fist rest grip of the game longbow (``battle_fine_weapons.longbow``), the arms and legs
of the rig posed onto the mesh for automatic (heat) weights, then brought back to the bind
pose so that the unchanged clips (``bow_walk``, ``bow_shoot``) play on it. The current
``archer_0`` (exported ``CAM1`` mesh skinned by the baked ``CAB1`` bones, as
``battle_fine_check``) is rendered beside it.

Run from the repository root (the probe is not part of the game pipeline):

    blender -b --factory-startup --python tools/blender_scripts/ga3_figure_probe.py -- \
        <glb> <out dir>
    python3 tools/blender_scripts/ga3_figure_probe.py sheet <out dir> <reference png> <jpg>
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

LOD0_TRIS = 11500  # fine foot soldiers: LOD0 10.1-11.9 k (battle_fine/manifest.json)
HELMET_TOP = 1.78  # MakeHuman body 1.70 m + kettle hat, metres
BOW_TRIS = 400
# Shots: (label, clip, fraction, camera eye, target); None = bind pose.
SHOTS = [
    ("bind", None, 0.0, (0.9, -3.3, 1.2), (0.45, 0, 0.95)),
    ("walk", "bow_walk", 0.25, (0.9, -3.3, 1.2), (0.45, 0, 0.95)),
    ("shoot_side", "bow_shoot", 0.5, (-2.6, -1.9, 1.5), (0.45, -0.1, 1.1)),
    ("shoot_front", "bow_shoot", 0.5, (1.7, -2.9, 1.5), (0.45, 0, 1.1)),
]
CURRENT_OFFSET = 0.9  # metres along +X for the current archer_0


# --- Import and cleanup -------------------------------------------------------------------


def import_figure(path):
    """Import the generated glb as one mesh object, transforms applied."""
    import bpy

    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    meshes = [o for o in new if o.type == "MESH"]
    for o in new:
        if o.type != "MESH":
            for c in o.children:
                mw = c.matrix_world.copy()
                c.parent = None
                c.matrix_world = mw
            bpy.data.objects.remove(o)
    obj = meshes[0]
    if len(meshes) > 1:
        with bpy.context.temp_override(
            active_object=obj, selected_editable_objects=meshes
        ):
            bpy.ops.object.join()
    obj.data.transform(obj.matrix_world)
    obj.matrix_world.identity()
    obj.name = "ga3_figure"
    return obj


def tris(obj):
    """Triangle count of a mesh object."""
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def drop_islands(obj, keep_ratio=0.002):
    """Remove loose islands smaller than `keep_ratio` of the vertices (TRELLIS crumbs)."""
    import bmesh

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    seen, islands = set(), []
    for v in bm.verts:
        if v in seen:
            continue
        stack, isl = [v], []
        seen.add(v)
        while stack:
            u = stack.pop()
            isl.append(u)
            for e in u.link_edges:
                w = e.other_vert(u)
                if w not in seen:
                    seen.add(w)
                    stack.append(w)
        islands.append(isl)
    small = [v for isl in islands if len(isl) < keep_ratio * len(bm.verts) for v in isl]
    bmesh.ops.delete(bm, geom=small, context="VERTS")
    bm.to_mesh(obj.data)
    bm.free()
    return len(islands), len(small)


def normalise(obj, arm):
    """Scale to the helmet height, feet on the ground, trunk centred on the rig's chest."""
    from mathutils import Matrix, Vector

    me = obj.data
    zs = [v.co.z for v in me.vertices]
    zmin, zmax = min(zs), max(zs)
    h = zmax - zmin
    centre_x = sum(v.co.x for v in me.vertices) / len(me.vertices)
    # Helmet top: highest vertex of the central column (the bow may rise above it).
    column = [v.co for v in me.vertices if abs(v.co.x - centre_x) < 0.06 * h]
    top = max(p.z for p in column)
    s = HELMET_TOP / (top - zmin)
    me.transform(Matrix.Translation((0, 0, -zmin)))
    me.transform(Matrix.Scale(s, 4))
    chest = arm.matrix_world @ arm.data.bones["Chest"].head_local
    band = [v.co for v in me.vertices if abs(v.co.z - chest.z) < 0.08]
    band = [p for p in band if abs(p.x - centre_x * s) < 0.2]
    cx = sum(p.x for p in band) / len(band)
    cy = sum(p.y for p in band) / len(band)
    me.transform(Matrix.Translation(Vector((chest.x - cx, chest.y - cy, 0))))
    return s


def split_bow(obj):
    """Cut the bow out of the figure (the figure's left side, +X): thin tall part.

    Vertices beyond the left fist that belong to a tall thin vertical sliver: the bow
    stands out of the body silhouette above the shoulder and below the hand.
    Returns the bow object (or None).
    """
    import bmesh
    import bpy

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.verts.ensure_lookup_table()
    xs = sorted(v.co.x for v in bm.verts)
    body_right = xs[int(0.02 * len(xs))]
    # Outermost +X column over the full height: bow limbs. Its x at each height band.
    bow_x = {}
    for v in bm.verts:
        k = int(v.co.z / 0.05)
        bow_x[k] = max(bow_x.get(k, -9), v.co.x)
    limb = sorted(bow_x[k] for k in bow_x if k * 0.05 > 1.45 or k * 0.05 < 0.55)
    ref_x = limb[len(limb) // 2] if limb else -body_right

    def is_bow(co):
        k = int(co.z / 0.05)
        edge = bow_x.get(k, ref_x)
        return co.x > edge - 0.045 and co.x > 0.2 and (co.z > 1.38 or co.z < 0.78)

    sel = {v for v in bm.verts if is_bow(v.co)}
    # Grip: the part of the limb line inside the hand band, bounded by the limb x.
    for v in bm.verts:
        if 0.78 <= v.co.z <= 1.38 and abs(v.co.x - ref_x) < 0.03 and v.co.x > 0.28:
            sel.add(v)
    if len(sel) < 30:
        bm.free()
        return None
    faces = [f for f in bm.faces if all(v in sel for v in f.verts)]
    bow_bm = bmesh.new()
    vmap = {}
    for f in faces:
        vs = []
        for v in f.verts:
            if v not in vmap:
                vmap[v] = bow_bm.verts.new(v.co)
            vs.append(vmap[v])
        nf = bow_bm.faces.new(vs)
        nf.material_index = f.material_index
    uv_src = bm.loops.layers.uv.active
    if uv_src is not None:
        uv_dst = bow_bm.loops.layers.uv.new(uv_src.name)
        for f, nf in zip(faces, bow_bm.faces, strict=True):
            for lo, nlo in zip(f.loops, nf.loops, strict=True):
                nlo[uv_dst].uv = lo[uv_src].uv
    bmesh.ops.delete(bm, geom=faces, context="FACES_ONLY")
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    bm.to_mesh(obj.data)
    bm.free()
    me = bpy.data.meshes.new("ga3_bow")
    bow_bm.to_mesh(me)
    bow_bm.free()
    for m in obj.data.materials:
        me.materials.append(m)
    bow = bpy.data.objects.new("ga3_bow", me)
    bpy.context.scene.collection.objects.link(bow)
    return bow


def game_longbow(arm):
    """The fine pipeline's longbow limbs (``battle_fine_weapons.longbow``) on ``Wrist.L``.

    The generated bow is unusable (thin limbs broken into fragments by TRELLIS), so the
    figure keeps the procedural weapon, as the current archers do. String and arrow ride
    the virtual ``Nock`` / ``Arrow`` bones of the game shader: left out of this probe.
    """
    import battle_fine_gear as gear
    import battle_fine_proto as fp
    import battle_fine_weapons as fw
    import battle_skinned as bs
    import battle_skinned_equipment as eq
    import bpy

    ctx = eq.Context(arm, 0, bs.material, bs.bone_world)
    objs = fw.longbow(gear.Gear(ctx, None, [], "ga3_probe", False))
    bow = objs[0]
    for o in objs[1:]:
        bpy.data.objects.remove(o)
    wood = bpy.data.materials.new("ga3_yew")
    wood.use_nodes = True
    wood.node_tree.nodes["Principled BSDF"].inputs[0].default_value = (
        0.45,
        0.28,
        0.12,
        1,
    )
    bow.data.materials.clear()
    bow.data.materials.append(wood)
    fp.attach(bow, arm)
    return bow


# --- Rig fit ------------------------------------------------------------------------------


def _extreme_centroid(pts, key, frac=0.03):
    """Centroid of the `frac` share of `pts` with the largest `key`."""
    from mathutils import Vector

    pts = sorted(pts, key=key, reverse=True)
    top = pts[: max(1, int(frac * len(pts)))]
    return sum(top, Vector()) / len(top)


def mesh_limbs(obj):
    """Hand and ankle positions of the (bowless) figure, per side."""
    vs = [v.co.copy() for v in obj.data.vertices]
    out = {}
    for side, sgn in (("L", 1), ("R", -1)):
        hand_band = [p for p in vs if 0.7 < p.z < 1.0 and p.x * sgn > 0.15]
        out["hand." + side] = _extreme_centroid(hand_band, lambda p, s=sgn: p.x * s)
        feet = [p for p in vs if p.z < 0.12 and p.x * sgn > 0.02]
        ankle = sum(feet, feet[0] * 0) / len(feet)
        ankle.z = 0.09
        out["ankle." + side] = ankle
    return out


def _rotate_bone(pb, rot):
    """Rotate pose bone `pb` about its head by `rot` (3x3, armature space)."""
    import bpy
    from mathutils import Matrix

    head = pb.head.copy()
    pb.matrix = (
        Matrix.Translation(head) @ rot.to_4x4() @ Matrix.Translation(-head) @ pb.matrix
    )
    bpy.context.view_layer.update()


def fit_rig_to_mesh(arm, limbs):
    """Pose `arm` so that its arms and legs lie in the mesh's limbs (world `limbs`).

    Pose matrices live in armature space (the glTF rig is rotated and scaled 1/100):
    the limbs are brought there first. Returns the rotation angles in degrees.
    """
    import bpy
    from mathutils import Matrix

    inv = arm.matrix_world.inverted()
    loc = {k: inv @ v for k, v in limbs.items()}
    world_up = (inv.to_3x3() @ Matrix.Identity(3).col[2]).normalized()
    angles = {}
    bpy.context.view_layer.update()
    for side in ("L", "R"):
        pb = arm.pose.bones[f"UpperArm.{side}"]
        tip = arm.pose.bones[f"Middle2.{side}"].head
        rot = (tip - pb.head).rotation_difference(loc["hand." + side] - pb.head)
        _rotate_bone(pb, rot.to_matrix())
        angles[pb.name] = round(math.degrees(rot.angle), 1)
    for side in ("L", "R"):
        pb = arm.pose.bones[f"UpperLeg.{side}"]
        foot = arm.pose.bones[f"Foot.{side}"]
        target = loc["ankle." + side]
        # Keep the foot's height: only its ground position follows the mesh.
        target = target - world_up * (target - foot.head).dot(world_up)
        rot = (foot.head - pb.head).rotation_difference(target - pb.head)
        _rotate_bone(pb, rot.to_matrix())
        angles[pb.name] = round(math.degrees(rot.angle), 1)
        foot.matrix = Matrix.Translation(target - foot.head) @ foot.matrix
        bpy.context.view_layer.update()
    print("FIT angles", angles)
    return angles


# Tail of each weight bone: the joint it points to (the Quaternius bones are 7 mm stubs, so
# bone heat on them diffuses from points and gives the feet to ``Body``).
WEIGHT_TAILS = {
    "Hips": "Abdomen",
    "Abdomen": "Torso",
    "Torso": "Chest",
    "Chest": "Neck",
    "Neck": "Head",
    "Shoulder.{s}": "UpperArm.{s}",
    "UpperArm.{s}": "LowerArm.{s}",
    "LowerArm.{s}": "Wrist.{s}",
    "Wrist.{s}": "Middle2.{s}",
    "UpperLeg.{s}": "LowerLeg.{s}",
    "LowerLeg.{s}": "Foot.{s}",
}
NO_DEFORM = ("Root", "Body", "PT.")


def weight_rig(fit):
    """Armature with the deform bones of `fit` (at its rest) as joint-to-joint segments."""
    import bpy
    from mathutils import Vector

    tails = {}
    for k, v in WEIGHT_TAILS.items():
        for side in ("L", "R"):
            tails[k.format(s=side)] = v.format(s=side)
    heads = {b.name: b.head_local.copy() for b in fit.data.bones}
    scale = fit.matrix_world.to_scale().x
    data = bpy.data.armatures.new("ga3_weights")
    rig = bpy.data.objects.new("ga3_weights", data)
    bpy.context.scene.collection.objects.link(rig)
    rig.matrix_world = fit.matrix_world.copy()
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for b in fit.data.bones:
        if b.name.startswith(NO_DEFORM):
            continue
        eb = data.edit_bones.new(b.name)
        eb.head = heads[b.name]
        if b.name in tails:
            eb.tail = heads[tails[b.name]]
        elif b.name == "Head":
            eb.tail = eb.head + Vector((0, 0, 0.22 / scale))
        elif b.name.startswith("Foot."):
            eb.tail = eb.head + Vector((0, -0.14 / scale, -0.05 / scale))
        elif b.children:
            eb.tail = heads[b.children[0].name]
        elif b.parent is not None:
            d = heads[b.name] - heads[b.parent.name]
            eb.tail = eb.head + d.normalized() * max(d.length, 0.02 / scale)
        if (eb.tail - eb.head).length < 1e-3 / scale:
            eb.tail = eb.head + Vector((0, 0, 0.02 / scale))
    bpy.ops.object.mode_set(mode="OBJECT")
    return rig


def skin_to_bind(obj, arm):
    """Auto-weight `obj` on the rig posed onto the mesh, then bake it to the bind pose.

    A copy of `arm` is posed onto the mesh's limbs and the pose applied as its rest; heat
    weights are computed on a segment rig of the same bones (``weight_rig``); the copy is
    then posed back onto `arm`'s rest, which carries the mesh to the bind pose of the clips.
    Returns (heat ok flag, measured limbs).
    """
    import battle_fine_proto as fp
    import bpy

    fit = arm.copy()
    fit.data = arm.data.copy()
    fit.name = "ga3_fit"
    bpy.context.scene.collection.objects.link(fit)
    fit.animation_data_clear()
    # obj.copy() keeps the IK targets on the original armature: drop them, every bone of
    # the copy is posed explicitly.
    for pb in fit.pose.bones:
        for c in list(pb.constraints):
            pb.constraints.remove(c)
    limbs = mesh_limbs(obj)
    fit_rig_to_mesh(fit, limbs)
    for o in bpy.context.selected_objects:
        o.select_set(False)
    bpy.context.view_layer.objects.active = fit
    fit.select_set(True)
    bpy.ops.object.mode_set(mode="POSE")
    bpy.ops.pose.armature_apply(selected=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    fit.select_set(False)
    wrig = weight_rig(fit)
    obj.select_set(True)
    wrig.select_set(True)
    bpy.context.view_layer.objects.active = wrig
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    weighted = sum(1 for v in obj.data.vertices if v.groups)
    ok = weighted > 0.95 * len(obj.data.vertices)
    print(f"HEAT weighted={weighted}/{len(obj.data.vertices)}")
    for m in [m for m in obj.modifiers if m.type == "ARMATURE"]:
        obj.modifiers.remove(m)
    mw = obj.matrix_world.copy()
    obj.parent = None
    obj.matrix_world = mw
    bpy.data.objects.remove(wrig)
    mod = obj.modifiers.new("fit", "ARMATURE")
    mod.object = fit
    # Pose the copy back onto the original rest, bake the mesh there.
    for b in bs_hierarchy(fit):
        fit.pose.bones[b.name].matrix = arm.data.bones[b.name].matrix_local.copy()
        bpy.context.view_layer.update()
    with bpy.context.temp_override(object=obj, active_object=obj):
        bpy.ops.object.modifier_apply(modifier=mod.name)
    obj.data.transform(obj.matrix_world)
    obj.matrix_world.identity()
    bpy.data.objects.remove(fit)
    fp.attach(obj, arm)
    return ok, limbs


def bs_hierarchy(arm):
    """Bones parents first."""
    import battle_fine_rig as fr

    return fr._hierarchy(arm)


# --- Rendering ----------------------------------------------------------------------------


def textured_look(objs):
    """Workbench texture shading needs the image on the active node: nothing to do for glb."""
    for o in objs:
        o.data.shade_smooth()


def load_cam(path):
    """``battle_fine_check.load_mesh`` for ``CAM1`` and ``CAM2`` (packed atlas UV) files.

    Returns (positions, colours, bones, weights, masks, indices, atlas or None).
    """
    import struct
    import zlib

    with open(path, "rb") as f:
        data = f.read()
    magic = data[:4]
    n, m, _size = struct.unpack("<III", data[4:16])
    raw = zlib.decompress(data[16:])
    per = 22 if magic == b"CAM2" else 21
    floats = struct.unpack(f"<{n * per}f", raw[: n * per * 4])
    idx = struct.unpack(f"<{m}I", raw[n * per * 4 : n * per * 4 + m * 4])

    def cols(offset, width):
        return [floats[offset + i * width : offset + (i + 1) * width] for i in range(n)]

    atlas = floats[n * 21 : n * 22] if per == 22 else None
    return (
        cols(0, 3),
        cols(n * 6, 4),
        cols(n * 12, 4),
        cols(n * 16, 4),
        floats[n * 20 : n * 21],
        idx,
        atlas,
    )


def _vertex_colour_material():
    """Base colour from the ``col`` attribute (alpha holds the shader code, ignored)."""
    import bpy

    mat = bpy.data.materials.new("archer_0_look")
    mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.8
    col = nt.nodes.new("ShaderNodeVertexColor")
    col.layer_name = "col"
    nt.links.new(col.outputs[0], bsdf.inputs["Base Color"])
    return mat


def add_current_archer(clip, frac):
    """The exported ``archer_0`` LOD0 skinned at `clip` / `frac`, shifted along +X.

    Skinned on the CPU like ``battle_fine_check``; vertex colours only (the game adds the
    baked detail atlas and the livery on top).
    """
    import json

    import battle_fine_check as fc
    import battle_skinned as bs
    from mathutils import Matrix

    mesh_dir = os.path.join(bs.ROOT, "game", "assets", "models", "battle_fine")
    with open(os.path.join(mesh_dir, "manifest.json")) as f:
        manifest = json.load(f)
    entry = manifest["figures"]["archer_0"]
    rig = manifest["rigs"]["human"]
    frames = fc.load_bones(os.path.join(mesh_dir, rig["texture"]))
    *mesh, atlas = load_cam(os.path.join(mesh_dir, entry["lods"][0]))
    c = rig["clips"][clip or "bow_idle"]
    fr = c["start"] + int(round((frac if clip else 0.0) * (c["frames"] - 1)))
    obj = fc.skinned_object("archer_0", mesh, frames[fr], 0)
    obj.data.transform(Matrix.Translation((CURRENT_OFFSET, 0, 0)))
    me = obj.data
    _ = atlas  # packed UV of the detail/normal atlas: not needed for the look
    me.materials.append(_vertex_colour_material())
    return obj


def setup_eevee(res=(520, 620)):
    """Eevee render with a sun and soft world light, textures shown."""
    import bpy

    scene = bpy.context.scene
    engines = [
        e.identifier
        for e in bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items
    ]
    scene.render.engine = (
        "BLENDER_EEVEE_NEXT" if "BLENDER_EEVEE_NEXT" in engines else "BLENDER_EEVEE"
    )
    scene.render.resolution_x, scene.render.resolution_y = res
    scene.world = scene.world or bpy.data.worlds.new("w")
    scene.world.use_nodes = True
    bg = scene.world.node_tree.nodes.get("Background")
    bg.inputs[0].default_value = (0.42, 0.42, 0.44, 1)
    bg.inputs[1].default_value = 0.9
    sun = bpy.data.objects.get("sun")
    if sun is None:
        sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
        scene.collection.objects.link(sun)
        sun.data.energy = 3.0
        sun.rotation_euler = (math.radians(50), 0, math.radians(-30))
    scene.view_settings.view_transform = "Standard"
    return scene


def main():
    """Probe steps: import, cleanup, rig fit, renders."""
    import battle_fine as bf
    import battle_fine_proto as fp
    import bpy

    args = sys.argv[sys.argv.index("--") + 1 :]
    glb, out = os.path.expanduser(args[0]), os.path.expanduser(args[1])
    os.makedirs(out, exist_ok=True)
    arm, _ = bf.load_fine_human()
    fp.prime_virtuals(arm)
    obj = import_figure(glb)
    raw_tris = tris(obj)
    islands, dropped = drop_islands(obj)
    s = normalise(obj, arm)
    print(
        f"RAW tris={raw_tris} islands={islands} dropped_verts={dropped} scale={s:.3f}"
    )
    # Raw render (before any cut), front and three-quarter.
    scene = setup_eevee()
    cam = fp.camera()
    textured_look([obj])
    for k, eye in enumerate(((0.0, -3.6, 1.0), (-2.4, -2.6, 1.1))):
        fp.look_at(cam, eye, (0.1, 0, 0.92), 45)
        arm.hide_render = True
        fp.render(os.path.join(out, f"raw_{k}.png"))
    bow = split_bow(obj)
    print(f"TRELLIS bow verts={len(bow.data.vertices) if bow else 0} (dropped)")
    body_tris = bs_decimate(obj, LOD0_TRIS - BOW_TRIS)
    print(f"LOD0 body={body_tris}")
    ok, limbs = skin_to_bind(obj, arm)
    print("LIMBS", {k: tuple(round(x, 3) for x in v) for k, v in limbs.items()})
    if bow:
        bpy.data.objects.remove(bow)
    game_bow = game_longbow(arm)
    textured_look([obj])
    print(f"BOW procedural tris={tris(game_bow)}")
    for label, clip, frac, eye, target in SHOTS:
        for o in [o for o in bpy.data.objects if o.name.startswith("archer_0")]:
            bpy.data.objects.remove(o)
        for pb in arm.pose.bones:
            pb.matrix_basis.identity()
        if clip:
            fp.pose_clip(arm, clip, frac)
        bpy.context.view_layer.update()
        add_current_archer(clip, frac)
        fp.look_at(cam, eye, target, 40)
        scene.render.resolution_x, scene.render.resolution_y = (620, 620)
        fp.render(os.path.join(out, f"pose_{label}.png"))
    print(f"HEAT_OK {ok}")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out, "ga3_s2_probe.blend"))
    print("OK")


def bs_decimate(obj, target):
    """Collapse-decimate `obj` to about `target` triangles (UVs kept by the modifier)."""
    import battle_skinned as bs

    return bs.weld_and_decimate(obj, target)


# --- Contact sheet (plain Python + Pillow) ------------------------------------------------


def sheet(out, reference, jpg):
    """Reference | raw mesh (2 views) | rigged poses beside the current archer."""
    from PIL import Image, ImageDraw, ImageFont

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 15)
    except OSError:
        font = ImageFont.load_default()
    h = 520
    tiles = []
    ref = Image.open(reference).convert("RGB")
    tiles.append(("Référence (NB2)", ref.resize((int(ref.width * h / ref.height), h))))
    for name, label in (
        ("raw_0", "TRELLIS brut (face)"),
        ("raw_1", "TRELLIS brut (3/4)"),
        ("pose_bind", "GA3 riggée (liaison) | archer_0 actuel"),
        ("pose_walk", "bow_walk"),
        ("pose_shoot_side", "bow_shoot (profil)"),
        ("pose_shoot_front", "bow_shoot (3/4 face)"),
    ):
        im = Image.open(os.path.join(out, name + ".png")).convert("RGB")
        tiles.append((label, im.resize((int(im.width * h / im.height), h))))
    rows = [tiles[:3], tiles[3:]]
    width = min(1600, max(sum(t.width for _, t in r) for r in rows))
    canvas_rows = []
    for r in rows:
        w = sum(t.width for _, t in r)
        row = Image.new("RGB", (w, h + 26), (30, 30, 32))
        x = 0
        for label, t in r:
            row.paste(t, (x, 26))
            ImageDraw.Draw(row).text((x + 6, 5), label, fill=(235, 230, 215), font=font)
            x += t.width
        scale = width / w if w > width else 1.0
        canvas_rows.append(
            row.resize((int(row.width * scale), int(row.height * scale)))
        )
    total_h = sum(r.height for r in canvas_rows)
    canvas = Image.new("RGB", (width, total_h), (30, 30, 32))
    y = 0
    for r in canvas_rows:
        canvas.paste(r, (0, y))
        y += r.height
    canvas.save(jpg, quality=86)
    print("SHEET", jpg, canvas.size)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "sheet":
        sheet(*sys.argv[2:5])
    else:
        main()
