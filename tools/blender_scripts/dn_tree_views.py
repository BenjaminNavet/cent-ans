"""DN nature: render the 8 impostor views of an ingested tree glb (flat albedo, transparent).

Run through ``ga3_vegetation_l2.py sheet <species>`` (never by hand)::

    blender -b --factory-startup --python tools/blender_scripts/dn_tree_views.py -- GLB OUT_DIR [SIZE]

Writes ``OUT_DIR/view_0.png`` .. ``view_7.png``: the tree rotated 45 degrees per view about its
vertical axis, orthographic camera at 25 degrees elevation (the campaign impostor pitch, same as
the GA3 turnaround sheets), Workbench flat texture colour (no lighting: the impostor shader lights
the card), film transparent, whole tree framed with a small margin.
"""

import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

ELEVATION = math.radians(25.0)
VIEWS = 8


def main() -> None:
    """Import the glb, frame it, render the views."""
    glb, out_dir = sys.argv[sys.argv.index("--") + 1 : sys.argv.index("--") + 3]
    size = (
        int(sys.argv[sys.argv.index("--") + 3])
        if len(sys.argv) > sys.argv.index("--") + 3
        else 768
    )
    Path(out_dir).mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=glb)
    meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    corners = [o.matrix_world @ Vector(c) for o in meshes for c in o.bound_box]
    low = Vector(
        (
            min(c.x for c in corners),
            min(c.y for c in corners),
            min(c.z for c in corners),
        )
    )
    high = Vector(
        (
            max(c.x for c in corners),
            max(c.y for c in corners),
            max(c.z for c in corners),
        )
    )
    height = high.z - low.z
    radius = max(high.x - low.x, high.y - low.y) * 0.5
    pivot = bpy.data.objects.new("pivot", None)
    bpy.context.scene.collection.objects.link(pivot)
    pivot.location = ((low.x + high.x) / 2, (low.y + high.y) / 2, low.z)
    for obj in meshes:
        obj.parent = pivot
        obj.matrix_parent_inverse = pivot.matrix_world.inverted()
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "FLAT"
    shading.color_type = "TEXTURE"
    scene.view_settings.view_transform = "Standard"
    scene.render.film_transparent = True
    scene.render.resolution_x = scene.render.resolution_y = size
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    camera_data = bpy.data.cameras.new("cam")
    camera_data.type = "ORTHO"
    extent = (
        max(height * math.cos(ELEVATION) + 2 * radius * math.sin(ELEVATION), 2 * radius)
        * 1.08
    )
    camera_data.ortho_scale = extent
    camera = bpy.data.objects.new("cam", camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera
    distance = 50.0 * max(height, 1.0)
    centre = Vector((0.0, 0.0, height * 0.5))
    camera.location = (
        centre + Vector((0.0, -math.cos(ELEVATION), math.sin(ELEVATION))) * distance
    )
    direction = centre - camera.location
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    camera_data.clip_end = distance * 4
    for view in range(VIEWS):
        pivot.rotation_euler[2] = view * math.tau / VIEWS
        bpy.context.view_layer.update()
        scene.render.filepath = str(Path(out_dir) / f"view_{view}.png")
        bpy.ops.render.render(write_still=True)
    print("OK views", out_dir)


main()
