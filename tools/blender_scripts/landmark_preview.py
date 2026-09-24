"""Preview renders of a landmark GLB (lot L1): top view and close-ups, for review.

Run headless:
    blender --background --python landmark_preview.py -- <model.glb> <out_dir> [view ...]

Views: ``top`` (whole zone), ``centre`` (oblique over the Cité), ``notre_dame`` (close-up of
the west front), ``notre_dame_side`` (south side, flying buttresses). Renders with EEVEE and a sun,
at 1280 x 800. Only for documentation: the game renders the GLB itself.
"""

import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector

# name: (camera location (Blender, Z up), look-at point, lens mm)
VIEWS = {
    "top": ((0.0, -0.01, 16.0), (0.0, 0.0, 0.0), 30),
    "centre": ((0.6, -3.4, 2.2), (-0.4, 0.4, 0.0), 35),
    "notre_dame": ((-1.5, 0.45, 0.42), (-0.35, 0.12, 0.3), 40),
    "notre_dame_side": ((-0.75, -1.55, 0.62), (0.0, 0.0, 0.2), 38),
    # Siege backdrop (metres): from above, and from the besieged town's side.
    "siege_top": ((0.0, 0.0, 3200.0), (0.0, 0.01, 0.0), 30),
    "siege_view": ((0.0, -700.0, 120.0), (0.0, 200.0, 20.0), 35),
}


def look_at(obj, target):
    """Point a camera at ``target``."""
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def main() -> None:
    """Import the model, light it and render the requested views."""
    args = sys.argv[sys.argv.index("--") + 1 :]
    model, out_dir = Path(args[0]), Path(args[1])
    views = args[2:] or list(VIEWS)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(model))
    for obj in bpy.data.objects:
        if obj.type == "MESH" and obj.name in ("blocks", "louvre", "charles_v_hidden"):
            obj.hide_render = True
    # Ground plane under everything.
    bpy.ops.mesh.primitive_plane_add(
        size=8000 if "siege" in model.stem else 40, location=(0, 0, -0.001)
    )
    ground = bpy.context.active_object
    mat = bpy.data.materials.new("Terrain")
    mat.use_nodes = True
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (0.2, 0.22, 0.12, 1)
    ground.data.materials.append(mat)
    sun_data = bpy.data.lights.new("Sun", "SUN")
    sun_data.energy = 4.0
    sun = bpy.data.objects.new("Sun", sun_data)
    sun.rotation_euler = (math.radians(50), 0, math.radians(35))
    bpy.context.collection.objects.link(sun)
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.55, 0.6, 0.65, 1)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.6
    scene = bpy.context.scene
    scene.world = world
    try:
        scene.render.engine = "BLENDER_EEVEE_NEXT"
    except TypeError:
        scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1280
    scene.render.resolution_y = 800
    scene.view_settings.view_transform = "Standard"
    cam_data = bpy.data.cameras.new("Camera")
    cam_data.clip_start = 0.01 if "siege" not in model.stem else 1.0
    cam_data.clip_end = 20000.0
    camera = bpy.data.objects.new("Camera", cam_data)
    bpy.context.collection.objects.link(camera)
    scene.camera = camera
    out_dir.mkdir(parents=True, exist_ok=True)
    for view in views:
        location, target, lens = VIEWS[view]
        camera.location = location
        cam_data.lens = lens
        look_at(camera, target)
        scene.render.filepath = str(out_dir / f"{model.stem}_{view}.png")
        bpy.ops.render.render(write_still=True)
        print(f"VIEW {view} {scene.render.filepath}")
    print("OK")


if __name__ == "__main__":
    main()
