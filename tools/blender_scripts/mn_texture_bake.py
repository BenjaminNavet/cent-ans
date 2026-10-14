"""MN: texture an untextured Meshy mesh for free, then render a comparison row.

blender -b -P mn_texture_bake.py -- <raw_dir> <source.png> <t1.glb> <detail.png> <out_dir>

Methods (each exported as ``<out_dir>/<method>.glb`` and rendered front 3/4 + back 3/4):
- ``proj``: orthographic projection of the source image along the best-matching axis;
- ``detail``: ``proj`` multiplied by a box-mapped micro texture (stone or wood grain);
- ``transfer``: the shipped TRELLIS 1 texture baked onto the Meshy mesh (selected to active).
Reference rows: the textured Meshy 6-lite model and the shipped T1 model.
"""

import math
import sys
from pathlib import Path

import bpy
import numpy as np
from mathutils import Matrix, Vector
from mathutils.kdtree import KDTree

argv = [a for a in sys.argv[sys.argv.index("--") + 1 :] if not a.startswith("--")]
RAW, SOURCE, T1, DETAIL, OUT = (Path(a) for a in argv)
OUT.mkdir(parents=True, exist_ok=True)
TEX_SIZE = 2048


def reset():
    """An empty scene with a Cycles CPU engine."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 16
    return scene


def import_glb(path):
    """Import a glb and return its single joined mesh, centred on the ground, height 1."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    meshes = [o for o in bpy.data.objects if o not in before and o.type == "MESH"]
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    for leftover in [o for o in bpy.data.objects if o not in before and o != obj]:
        bpy.data.objects.remove(leftover)
    pts = np.array([v.co[:] for v in obj.data.vertices])
    lo, hi = pts.min(0), pts.max(0)
    centre = Vector(((lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, lo[2]))
    scale = 1.0 / (hi[2] - lo[2])
    obj.data.transform(Matrix.Scale(scale, 4) @ Matrix.Translation(-centre))
    return obj


def alpha_mask(image_path, size=128):
    """The source alpha, cropped to its bounding box and resampled to size x size."""
    img = bpy.data.images.load(str(image_path))
    w, h = img.size
    px = np.array(img.pixels[:]).reshape(h, w, 4)
    alpha = px[..., 3] > 0.5
    rows, cols = np.where(alpha.any(1))[0], np.where(alpha.any(0))[0]
    box = (cols[0], rows[0], cols[-1] + 1, rows[-1] + 1)  # x0, y0 (bottom row), x1, y1
    crop = alpha[box[1] : box[3], box[0] : box[2]]
    ys = np.linspace(0, crop.shape[0] - 1, size).astype(int)
    xs = np.linspace(0, crop.shape[1] - 1, size).astype(int)
    return img, (box[0] / w, box[1] / h, box[2] / w, box[3] / h), crop[np.ix_(ys, xs)]


def view_axes(yaw, pitch):
    """Right and up axes of an orthographic view (yaw about Z, pitch = elevation)."""
    right = Vector((math.cos(yaw), -math.sin(yaw), 0))
    depth = Vector(
        (
            math.sin(yaw) * math.cos(pitch),
            math.cos(yaw) * math.cos(pitch),
            -math.sin(pitch),
        )
    )
    return right, right.cross(depth).normalized()


def surface_points(obj, count=60000):
    """Random points on the mesh surface (area-weighted), for silhouettes."""
    obj.data.calc_loop_triangles()
    co = np.array([v.co[:] for v in obj.data.vertices])
    tris = np.array([t.vertices[:] for t in obj.data.loop_triangles])
    a, b, c = co[tris[:, 0]], co[tris[:, 1]], co[tris[:, 2]]
    area = np.linalg.norm(np.cross(b - a, c - a), axis=1)
    rng = np.random.default_rng(0)
    pick = rng.choice(len(tris), count, p=area / area.sum())
    r1, r2 = rng.random((2, count))
    flip = r1 + r2 > 1
    r1[flip], r2[flip] = 1 - r1[flip], 1 - r2[flip]
    return (
        a[pick] + (b[pick] - a[pick]) * r1[:, None] + (c[pick] - a[pick]) * r2[:, None]
    )


def project(pts, yaw, pitch):
    """Normalised (0..1) screen coordinates of points in a view, cropped to their box."""
    right, up = view_axes(yaw, pitch)
    u, v = pts @ np.array(right[:]), pts @ np.array(up[:])
    return (
        (u - u.min()) / np.ptp(u),
        (v - v.min()) / np.ptp(v),
        (u.min(), np.ptp(u), v.min(), np.ptp(v)),
    )


def best_view(obj, target, size=128):
    """(yaw, pitch) whose silhouette best matches the source alpha (IoU)."""
    pts = surface_points(obj)
    best = (-1.0, 0.0, 0.0)
    for k in range(24):
        for pitch_deg in range(0, 61, 5):
            yaw, pitch = k * math.pi / 12, math.radians(pitch_deg)
            u, v, _ = project(pts, yaw, pitch)
            sil = np.zeros((size, size), bool)
            sil[(v * (size - 1)).astype(int), (u * (size - 1)).astype(int)] = True
            iou = (sil & target).sum() / max((sil | target).sum(), 1)
            best = max(best, (iou, yaw, pitch))
    print(
        f"best view iou {best[0]:.3f} yaw {math.degrees(best[1]):.0f} pitch {math.degrees(best[2]):.0f}"
    )
    return best[1], best[2]


def projection_uv(obj, yaw, pitch, box):
    """A 'proj' UV map placing every vertex where the source image shows it."""
    _, _, (u0, du, v0, dv) = project(surface_points(obj), yaw, pitch)
    right, up = view_axes(yaw, pitch)
    pts = np.array([v.co[:] for v in obj.data.vertices])
    u = (pts @ np.array(right[:]) - u0) / du
    v = (pts @ np.array(up[:]) - v0) / dv
    u = box[0] + u * (box[2] - box[0])
    v = box[1] + v * (box[3] - box[1])
    layer = obj.data.uv_layers.new(name="proj")
    for loop in obj.data.loops:
        layer.data[loop.index].uv = (u[loop.vertex_index], v[loop.vertex_index])


def bake_uv(obj):
    """A fresh 'bake' UV map (smart projection) made active for rendering and baking."""
    for old in [uv for uv in obj.data.uv_layers if uv.name != "proj"]:
        obj.data.uv_layers.remove(old)
    layer = obj.data.uv_layers.new(name="bake")
    obj.data.uv_layers.active = layer
    layer.active_render = True
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(66), island_margin=0.003)
    bpy.ops.object.mode_set(mode="OBJECT")


def emission_material(name, source_img, detail_img=None):
    """Emission = source sampled through 'proj' (times box-mapped detail), plus a bake target."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    emit = nodes.new("ShaderNodeEmission")
    uv = nodes.new("ShaderNodeUVMap")
    uv.uv_map = "proj"
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = source_img
    links.new(uv.outputs["UV"], tex.inputs["Vector"])
    colour = tex.outputs["Color"]
    if detail_img is not None:
        coord = nodes.new("ShaderNodeTexCoord")
        mapping = nodes.new("ShaderNodeMapping")
        mapping.inputs["Scale"].default_value = (6, 6, 6)
        links.new(coord.outputs["Object"], mapping.inputs["Vector"])
        det = nodes.new("ShaderNodeTexImage")
        det.image = detail_img
        det.projection = "BOX"
        det.projection_blend = 0.3
        links.new(mapping.outputs["Vector"], det.inputs["Vector"])
        bw = nodes.new("ShaderNodeRGBToBW")
        links.new(det.outputs["Color"], bw.inputs["Color"])
        gain = nodes.new("ShaderNodeMath")  # detail around 1.0: 0.6 + 0.8 * luminance
        gain.operation = "MULTIPLY_ADD"
        gain.inputs[1].default_value = 0.8
        gain.inputs[2].default_value = 0.6
        links.new(bw.outputs["Val"], gain.inputs[0])
        mix = nodes.new("ShaderNodeMix")
        mix.data_type = "RGBA"
        mix.blend_type = "MULTIPLY"
        mix.inputs["Factor"].default_value = 1.0
        links.new(colour, mix.inputs["A"])
        links.new(gain.outputs["Value"], mix.inputs["B"])
        colour = mix.outputs["Result"]
    links.new(colour, emit.inputs["Color"])
    links.new(emit.outputs["Emission"], out.inputs["Surface"])
    return mat


def add_target(mat, name):
    """An active image node the bake writes into."""
    img = bpy.data.images.new(name, TEX_SIZE, TEX_SIZE)
    node = mat.node_tree.nodes.new("ShaderNodeTexImage")
    node.image = img
    mat.node_tree.nodes.active = node
    return img


def finish(obj, img, method):
    """Swap to a plain baked material, drop the 'proj' UV and export the glb."""
    mat = bpy.data.materials.new(f"{method}_baked")
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.9
    tex = mat.node_tree.nodes.new("ShaderNodeTexImage")
    tex.image = img
    mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    if "proj" in obj.data.uv_layers:
        obj.data.uv_layers.remove(obj.data.uv_layers["proj"])
    img.filepath_raw = str(OUT / f"{method}.png")
    img.file_format = "PNG"
    img.save()
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.ops.export_scene.gltf(filepath=str(OUT / f"{method}.glb"), use_selection=True)


def bake_projection(method, detail):
    """Methods 'proj' and 'detail'."""
    reset()
    obj = import_glb(RAW / "notex.glb")
    source_img, box, target = alpha_mask(SOURCE)
    yaw, pitch = best_view(obj, target)
    projection_uv(obj, yaw, pitch, box)
    bake_uv(obj)
    mat = emission_material(
        method, source_img, bpy.data.images.load(str(DETAIL)) if detail else None
    )
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    img = add_target(mat, method)
    bpy.context.scene.render.bake.margin = 8
    bpy.ops.object.bake(type="EMIT")
    finish(obj, img, method)


def align_to(src, dst):
    """Rotate src about Z (8 yaws, optional mirror) to minimise the mean distance to dst."""
    tree = KDTree(len(dst.data.vertices))
    for i, v in enumerate(dst.data.vertices):
        tree.insert(v.co, i)
    tree.balance()
    sample = np.array([v.co[:] for v in src.data.vertices])
    sample = sample[
        np.random.default_rng(0).choice(
            len(sample), min(3000, len(sample)), replace=False
        )
    ]
    best = None
    for mirror in (1, -1):
        for k in range(8):
            mat = Matrix.Rotation(k * math.pi / 4, 4, "Z") @ Matrix.Diagonal(
                (mirror, 1, 1, 1)
            )
            dist = np.mean([tree.find(mat @ Vector(p))[2] for p in sample])
            if best is None or dist < best[0]:
                best = (dist, mat)
    print("transfer alignment distance", round(best[0], 4))
    src.data.transform(best[1])
    if best[1].determinant() < 0:
        src.data.flip_normals()


def bake_transfer():
    """Method 'transfer': T1 UVs copied onto the Meshy mesh (nearest face), T1 image re-baked."""
    reset()
    target = import_glb(RAW / "notex.glb")
    source = import_glb(T1)
    align_to(source, target)
    for old in list(target.data.uv_layers):
        target.data.uv_layers.remove(old)
    target.data.uv_layers.new(name="proj")
    mod = target.modifiers.new("t1_uv", "DATA_TRANSFER")
    mod.object = source
    mod.use_loop_data = True
    mod.data_types_loops = {"UV"}
    mod.loop_mapping = "POLYINTERP_NEAREST"
    bpy.context.view_layer.objects.active = target
    bpy.ops.object.modifier_apply(modifier=mod.name)
    t1_image = next(
        n.image
        for n in source.data.materials[0].node_tree.nodes
        if n.type == "TEX_IMAGE"
    )
    bpy.data.objects.remove(source)
    bake_uv(target)
    mat = emission_material("transfer", t1_image)
    target.data.materials.clear()
    target.data.materials.append(mat)
    img = add_target(mat, "transfer")
    bpy.context.scene.render.bake.margin = 8
    bpy.ops.object.bake(type="EMIT")
    finish(target, img, "transfer")


def render_row(glb, name):
    """Front 3/4 and back 3/4 renders of a glb, sun + sky, 512 px each."""
    scene = reset()
    scene.cycles.samples = 32
    obj = import_glb(glb)
    world = bpy.data.worlds.new("w")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 1.0
    scene.world = world
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 3.5
    sun.rotation_euler = (math.radians(25), 0, math.radians(30))
    scene.collection.objects.link(sun)
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    cam.data.type = "ORTHO"
    pts = np.array([v.co[:] for v in obj.data.vertices])
    cam.data.ortho_scale = float(max(np.ptp(pts[:, 0]), np.ptp(pts[:, 1]), 1.0)) * 1.25
    scene.collection.objects.link(cam)
    scene.camera = cam
    scene.render.resolution_x = scene.render.resolution_y = 512
    scene.render.film_transparent = True
    for label, yaw in (("front", -30), ("back", 150)):
        a = math.radians(yaw)
        cam.location = Vector((math.sin(a) * -4, math.cos(a) * -4, 2.6))
        cam.rotation_euler = (
            (Vector((0, 0, 0.4)) - cam.location).to_track_quat("-Z", "Y").to_euler()
        )
        scene.render.filepath = str(OUT / f"render_{name}_{label}.png")
        bpy.ops.render.render(write_still=True)


if "--render-only" not in sys.argv:
    bake_projection("proj", detail=False)
    bake_projection("detail", detail=True)
    bake_transfer()
for glb_path, label in (
    (
        RAW / "meshy.glb" if (RAW / "meshy.glb").exists() else RAW / "meshy-6-lite.glb",
        "meshy_tex",
    ),
    (T1, "t1"),
    (OUT / "proj.glb", "proj"),
    (OUT / "detail.glb", "detail"),
    (OUT / "transfer.glb", "transfer"),
):
    render_row(glb_path, label)
