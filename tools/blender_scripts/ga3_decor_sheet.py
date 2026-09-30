"""GA3-L1: one framed Cycles panel per game-ready glb (decor sheet ``docs/img/ga3/l1_decor.jpg``).

Run headless from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/ga3_decor_sheet.py -- \
        OUT_PREFIX A.glb [B.glb ...] [--size 400] [--yaw 35] [--label]

Unlike ``ga3_sheet.py`` (fixed camera for same-size houses), each panel is framed on its model's
bounding sphere, so a 3 m well and a 22 m church both fill their panel. A 1.8 m grey post stands
beside every model for scale. The camera looks from ``--yaw`` degrees off the model's front
(+Z in Godot, -Y in Blender): ``--yaw 0`` checks the facade orientation. Writes
``OUT_PREFIX_<i>.png``; the sheet is assembled outside Blender (``magick montage``).
"""

import argparse
import math
import sys

import bpy
from mathutils import Vector


def parse_args() -> argparse.Namespace:
    """Parse the arguments after ``--``."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out_prefix")
    parser.add_argument("models", nargs="+")
    parser.add_argument("--size", type=int, default=400)
    parser.add_argument("--yaw", type=float, default=35.0)
    parser.add_argument("--samples", type=int, default=32)
    return parser.parse_args(argv)


def setup_scene(size: int, samples: int) -> bpy.types.Object:
    """Sun, sky, ground; returns the camera."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = True
    scene.render.resolution_x = scene.render.resolution_y = size
    scene.view_settings.view_transform = "AgX"
    world = bpy.data.worlds.new("sky")
    world.color = (0.55, 0.62, 0.72)
    scene.world = world
    sun_data = bpy.data.lights.new("sun", "SUN")
    sun_data.energy = 3.5
    sun_data.angle = math.radians(3.0)
    sun = bpy.data.objects.new("sun", sun_data)
    sun.rotation_euler = (math.radians(50.0), 0.0, math.radians(105.0))
    scene.collection.objects.link(sun)
    bpy.ops.mesh.primitive_circle_add(vertices=64, radius=60.0, fill_type="NGON")
    ground = bpy.context.active_object
    material = bpy.data.materials.new("ground")
    material.use_nodes = True
    bsdf = material.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (0.28, 0.27, 0.22, 1.0)
    bsdf.inputs["Roughness"].default_value = 1.0
    ground.data.materials.append(material)
    cam_data = bpy.data.cameras.new("cam")
    cam_data.lens = 50
    cam = bpy.data.objects.new("cam", cam_data)
    scene.collection.objects.link(cam)
    scene.camera = cam
    return cam


def scale_post(x: float) -> bpy.types.Object:
    """A 1.8 m grey post at ``x`` (scale reference)."""
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=12, radius=0.12, depth=1.8, location=(x, 0.0, 0.9)
    )
    return bpy.context.active_object


def render_model(path: str, out: str, cam: bpy.types.Object, yaw: float) -> None:
    """Import ``path``, frame it, render to ``out`` and remove it again."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    imported = [o for o in bpy.data.objects if o not in before and o.type == "MESH"]
    corners = [o.matrix_world @ Vector(c) for o in imported for c in o.bound_box]
    lo = Vector(min(c[i] for c in corners) for i in range(3))
    hi = Vector(max(c[i] for c in corners) for i in range(3))
    post = scale_post(hi.x + 0.6)
    centre = (lo + hi) / 2
    radius = max((hi - lo).length / 2, 1.2)
    direction = Vector(
        (math.sin(math.radians(yaw)), -math.cos(math.radians(yaw)), 0.45)
    ).normalized()
    cam.location = centre + direction * radius * 2.9
    cam.rotation_euler = (-direction).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.render.filepath = out
    bpy.ops.render.render(write_still=True)
    tris = 0
    for obj in imported:
        obj.data.calc_loop_triangles()
        tris += len(obj.data.loop_triangles)
    print(f"GA3 decor sheet: {path} ({tris} tris) -> {out}")
    for obj in [o for o in bpy.data.objects if o not in before]:
        bpy.data.objects.remove(obj)
    del post


def main() -> None:
    """Render one framed panel per model."""
    args = parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    cam = setup_scene(args.size, args.samples)
    for index, model in enumerate(args.models):
        render_model(model, f"{args.out_prefix}_{index}.png", cam, args.yaw)


if __name__ == "__main__":
    main()
