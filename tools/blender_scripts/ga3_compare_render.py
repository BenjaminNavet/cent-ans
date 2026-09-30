"""GA3-S3: side-by-side sheet of image-to-3D figures (same camera, same light).

Blender headless::

    blender -b --factory-startup -P tools/blender_scripts/ga3_compare_render.py -- \
        --out docs/img/ga3/s3_compare.jpg --stats stats.json \
        --ref front.png:back.png "Référence" \
        --panel model.glb "Tripo H3.1" [--yaw 0] [--frame N|rest] ...

Each ``--panel`` is a glb plus a label; ``--yaw`` (degrees) turns that model so it faces the
camera, ``--frame`` picks an animation frame (``rest`` = armature rest pose, default: no
animation change). Two rows: three-quarter front and three-quarter back. Each model is scaled
to 1.80 m, feet on the ground. The mesh statistics (triangles, textures and the material
inputs they feed, mesh objects and loose parts, bones) are written to ``--stats``.
"""

import json
import math
import sys

import bmesh
import bpy
import numpy as np

PANEL_W, PANEL_H = 320, 480
HEIGHT = 1.80
ROW_YAWS = (-25.0, 155.0)


def parse_args(argv):
    """Return (out, stats, reference, panels) from the argv after ``--``."""
    argv = argv[argv.index("--") + 1 :] if "--" in argv else []
    out, stats, reference, panels = None, None, None, []
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--out":
            out, i = argv[i + 1], i + 2
        elif arg == "--stats":
            stats, i = argv[i + 1], i + 2
        elif arg == "--ref":
            reference, i = (argv[i + 1].split(":"), argv[i + 2]), i + 3
        elif arg == "--panel":
            panels.append(
                {"path": argv[i + 1], "label": argv[i + 2], "yaw": 0.0, "frame": None}
            )
            i += 3
        elif arg == "--yaw":
            panels[-1]["yaw"], i = float(argv[i + 1]), i + 2
        elif arg == "--frame":
            panels[-1]["frame"], i = argv[i + 1], i + 2
        else:
            raise SystemExit(f"unknown argument {arg}")
    return out, stats, reference, panels


def reset_scene():
    """Empty scene with the shared camera, sun and world."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = PANEL_W, PANEL_H
    scene.render.film_transparent = False
    scene.view_settings.view_transform = "Standard"
    scene.world = bpy.data.worlds.new("world")
    scene.world.use_nodes = True
    background = scene.world.node_tree.nodes["Background"]
    background.inputs[0].default_value = (0.42, 0.42, 0.44, 1)
    background.inputs[1].default_value = 0.9
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 3.0
    sun.rotation_euler = (math.radians(50), 0, math.radians(-30))
    scene.collection.objects.link(sun)
    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    camera.data.lens = 70
    camera.location = (0.0, -4.3, 0.93)
    camera.rotation_euler = (math.radians(90), 0, 0)
    scene.collection.objects.link(camera)
    scene.camera = camera
    return scene, camera


def add_label(camera, text):
    """White label pinned to the top of the camera frame."""
    curve = bpy.data.curves.new("label", "FONT")
    curve.body = text
    curve.size = 0.022
    curve.align_x = "CENTER"
    label = bpy.data.objects.new("label", curve)
    material = bpy.data.materials.new("label")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    nodes.clear()
    emission = nodes.new("ShaderNodeEmission")
    emission.inputs[0].default_value = (1, 1, 1, 1)
    output = nodes.new("ShaderNodeOutputMaterial")
    material.node_tree.links.new(emission.outputs[0], output.inputs[0])
    curve.materials.append(material)
    label.parent = camera
    label.location = (0.0, 0.232, -1.0)
    bpy.context.scene.collection.objects.link(label)
    return label


def import_glb(path):
    """Import a glb under a root empty; return (root, meshes, armatures)."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    root = bpy.data.objects.new("root", None)
    bpy.context.scene.collection.objects.link(root)
    for obj in new:
        if obj.parent is None:
            obj.parent = root
    meshes = [o for o in new if o.type == "MESH"]
    armatures = [o for o in new if o.type == "ARMATURE"]
    return root, meshes, armatures


def world_points(meshes):
    """Evaluated world-space vertex positions (armature deformation included)."""
    depsgraph = bpy.context.evaluated_depsgraph_get()
    points = []
    for obj in meshes:
        evaluated = obj.evaluated_get(depsgraph)
        mesh = evaluated.to_mesh()
        coords = np.empty(len(mesh.vertices) * 3)
        mesh.vertices.foreach_get("co", coords)
        coords = coords.reshape(-1, 3)
        matrix = np.array(evaluated.matrix_world)
        points.append(coords @ matrix[:3, :3].T + matrix[:3, 3])
        evaluated.to_mesh_clear()
    return np.concatenate(points)


def normalise(root, meshes):
    """Scale to HEIGHT, feet at z = 0, centred on the vertical axis."""
    bpy.context.view_layer.update()
    points = world_points(meshes)
    low, high = points.min(axis=0), points.max(axis=0)
    scale = HEIGHT / max(high[2] - low[2], 1e-6)
    root.scale = (scale, scale, scale)
    bpy.context.view_layer.update()
    points = world_points(meshes)
    low, high = points.min(axis=0), points.max(axis=0)
    centre = (low + high) / 2
    root.location = (-centre[0], -centre[1], -low[2])


def set_frame(armatures, frame):
    """Rest pose, a given animation frame, or leave as imported."""
    if frame is None:
        return
    for arm in armatures:
        if frame == "rest":
            arm.data.pose_position = "REST"
        else:
            arm.data.pose_position = "POSE"
    if frame != "rest":
        bpy.context.scene.frame_set(int(frame))
    bpy.context.view_layer.update()


def material_inputs(material):
    """Map BSDF input name -> image size for the textures feeding a material."""
    found = {}
    if not material or not material.use_nodes:
        return found
    for node in material.node_tree.nodes:
        if node.type != "TEX_IMAGE" or node.image is None:
            continue
        size = f"{node.image.size[0]}x{node.image.size[1]}"
        targets = []
        stack = [link for out in node.outputs for link in out.links]
        while stack:
            link = stack.pop()
            if link.to_node.type == "BSDF_PRINCIPLED":
                targets.append(link.to_socket.name)
            else:
                stack.extend(lnk for out in link.to_node.outputs for lnk in out.links)
        for target in targets or ["(unlinked)"]:
            found[target] = size
    return found


def loose_parts(obj):
    """Sizes (vertex counts) of the connected parts of a mesh, biggest first."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.verts.ensure_lookup_table()
    seen, sizes = set(), []
    for start in bm.verts:
        if start.index in seen:
            continue
        stack, count = [start], 0
        seen.add(start.index)
        while stack:
            vert = stack.pop()
            count += 1
            for edge in vert.link_edges:
                other = edge.other_vert(vert)
                if other.index not in seen:
                    seen.add(other.index)
                    stack.append(other)
        sizes.append(count)
    bm.free()
    return sorted(sizes, reverse=True)


def stats_of(meshes, armatures):
    """Triangles, textures, parts and bones of an imported glb."""
    triangles, materials, parts = 0, {}, {}
    for obj in meshes:
        obj.data.calc_loop_triangles()
        triangles += len(obj.data.loop_triangles)
        for slot in obj.material_slots:
            if slot.material:
                materials[slot.material.name] = material_inputs(slot.material)
        sizes = loose_parts(obj)
        total = sum(sizes)
        parts[obj.name] = {
            "vertices": total,
            "parts": len(sizes),
            "parts_over_1pct": [s for s in sizes if s > 0.01 * total],
        }
    bones = [b.name for arm in armatures for b in arm.data.bones]
    actions = [
        {"name": a.name, "frames": list(a.frame_range)} for a in bpy.data.actions
    ]
    return {
        "triangles": triangles,
        "mesh_objects": parts,
        "materials": materials,
        "bones": bones,
        "actions": actions,
    }


def render_to_array(scene):
    """Render the current scene, return an RGBA float array (H, W, 4), top row first."""
    scene.render.filepath = "/tmp/ga3_compare_panel.png"
    bpy.ops.render.render(write_still=True)
    image = bpy.data.images.load(scene.render.filepath, check_existing=False)
    pixels = np.array(image.pixels[:]).reshape(PANEL_H, PANEL_W, 4)[::-1]
    bpy.data.images.remove(image)
    return pixels


def reference_panel(path, label, camera):
    """Render a reference crop as an emissive card filling the frame (same pipeline)."""
    scene = bpy.context.scene
    image = bpy.data.images.load(path)
    material = bpy.data.materials.new("reference")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    nodes.clear()
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = image
    emission = nodes.new("ShaderNodeEmission")
    output = nodes.new("ShaderNodeOutputMaterial")
    links = material.node_tree.links
    links.new(texture.outputs[0], emission.inputs[0])
    links.new(emission.outputs[0], output.inputs[0])
    bpy.ops.mesh.primitive_plane_add(size=1.0)
    card = bpy.context.active_object
    card.data.materials.append(material)
    # Card in front of the camera, square crop covering the frame height.
    depth = 2.0
    frame_h = depth * 36.0 / camera.data.lens * PANEL_H / max(PANEL_W, PANEL_H)
    card.parent = camera
    card.location = (0.0, 0.0, -depth)
    card.rotation_euler = (0.0, 0.0, 0.0)
    card.scale = (frame_h, frame_h, 1.0)
    add_label(camera, label)
    return render_to_array(scene)


def main():
    """Render every panel in two rows and write the sheet and the stats."""
    out, stats_path, reference, panels = parse_args(sys.argv)
    columns, all_stats = [], {}
    if reference:
        (front, back), label = reference
        column = []
        for path in (front, back):
            _, camera = reset_scene()
            column.append(reference_panel(path, label, camera))
        columns.append(column)
    for panel in panels:
        column = []
        for row, row_yaw in enumerate(ROW_YAWS):
            _, camera = reset_scene()
            root, meshes, armatures = import_glb(panel["path"])
            set_frame(armatures, panel["frame"])
            if row == 0:
                all_stats[panel["label"]] = stats_of(meshes, armatures)
            normalise(root, meshes)
            # The model is centred on the vertical axis: turn it about the origin.
            pivot = bpy.data.objects.new("pivot", None)
            bpy.context.scene.collection.objects.link(pivot)
            root.parent = pivot
            pivot.rotation_euler = (0.0, 0.0, math.radians(panel["yaw"] + row_yaw))
            add_label(camera, panel["label"])
            column.append(render_to_array(bpy.context.scene))
        columns.append(column)
    sheet = np.concatenate([np.concatenate(col, axis=0) for col in columns], axis=1)
    height, width = sheet.shape[:2]
    image = bpy.data.images.new("sheet", width, height, alpha=False)
    image.pixels = sheet[::-1].ravel()
    image.filepath_raw = out
    image.file_format = "JPEG"
    bpy.context.scene.render.image_settings.quality = 88
    image.save()
    if stats_path:
        with open(stats_path, "w") as handle:
            json.dump(all_stats, handle, indent=1)
    print("SHEET", out, width, height)
    for label, stats in all_stats.items():
        print(
            "STATS", label, stats["triangles"], len(stats["bones"]), stats["materials"]
        )


if __name__ == "__main__":
    main()
