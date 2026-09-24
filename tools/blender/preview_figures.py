"""Quick look at the battle figures exported by `battle_figures.py` (Workbench render with
vertex colours; livery shown as a flat blue). Development aid, not part of the build:

    blender --background --python tools/blender/preview_figures.py -- out.png [names...]
"""

import math
import os
import sys

import bpy
from mathutils import Vector

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MODELS = os.path.join(ROOT, "game", "assets", "models", "battle")
LIVERY = (0.08, 0.12, 0.45)


def main():
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = args[0] if args else os.path.join(ROOT, "preview.png")
    names = args[1:] or ["infantry_0", "infantry_1", "archer_0", "archer_2", "cavalry_0", "cavalry_2"]
    yaw = float(os.environ.get("PREVIEW_YAW", "35"))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    x = 0.0
    for name in names:
        bpy.ops.import_scene.gltf(filepath=os.path.join(MODELS, name + ".glb"))
        for obj in bpy.context.selected_objects:
            if obj.type != "MESH":
                continue
            obj.location.x = x
            obj.rotation_mode = "XYZ"
            obj.rotation_euler.z = math.radians(yaw)
            col = obj.data.color_attributes[0]
            for d in col.data:
                r, g, b, a = d.color
                code = round(a * 5.0)
                if code in (0, 5):
                    d.color = (LIVERY[0] * r, LIVERY[1] * g, LIVERY[2] * b, 1.0)
                elif code == 1:
                    d.color = (0.6, 0.45, 0.1, 1.0)
                else:
                    d.color = (r, g, b, 1.0)
        x += 2.2 if name.startswith("cavalry") else 1.1
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "VERTEX"
    shading.show_shadows = True
    shading.show_cavity = True
    scene.render.resolution_x = int(os.environ.get("PREVIEW_W", "1600"))
    scene.render.resolution_y = int(os.environ.get("PREVIEW_H", "800"))
    scene.render.filepath = out
    cam_data = bpy.data.cameras.new("cam")
    cam_data.lens = float(os.environ.get("PREVIEW_LENS", "50"))
    cam = bpy.data.objects.new("cam", cam_data)
    scene.collection.objects.link(cam)
    scene.camera = cam
    centre = Vector((x * 0.5 - 0.6, 0.0, 1.1))
    dist = float(os.environ.get("PREVIEW_DIST", str(max(6.0, x * 1.2))))
    elev = math.radians(float(os.environ.get("PREVIEW_ELEV", "12")))
    cam.location = centre + Vector((0.0, -math.cos(elev) * dist, math.sin(elev) * dist))
    direction = centre - cam.location
    cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    world = bpy.data.worlds.new("w")
    scene.world = world
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    main()
