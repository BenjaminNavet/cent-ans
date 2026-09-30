"""GA3: render game-ready glb props side by side under one camera and one light.

Run headless from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/ga3_sheet.py -- \
        OUT_PREFIX A.glb [B.glb ...] [--size 560] [--yaw 35]

Writes ``OUT_PREFIX_<i>.png`` per model (Cycles, sun + sky-coloured world, grey ground
disc, same framing: the models are expected at real scale with their foot at z = 0, as
written by ``ga3_cleanup.py``). The panels are assembled into a sheet outside Blender.
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
    parser.add_argument("--size", type=int, default=560)
    parser.add_argument("--yaw", type=float, default=35.0)
    return parser.parse_args(argv)


def setup_scene(size: int, yaw: float) -> None:
    """Camera, sun, world and ground shared by every panel."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 48
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
    sun.rotation_euler = (math.radians(50.0), 0.0, math.radians(yaw + 70.0))
    scene.collection.objects.link(sun)

    bpy.ops.mesh.primitive_circle_add(vertices=64, radius=14.0, fill_type="NGON")
    ground = bpy.context.active_object
    ground.name = "ground"
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
    target = Vector((0.0, 0.0, 3.0))
    direction = Vector(
        (math.sin(math.radians(yaw)), -math.cos(math.radians(yaw)), 0.55)
    ).normalized()
    cam.location = target + direction * 21.0
    cam.rotation_euler = (-direction).to_track_quat("-Z", "Y").to_euler()
    scene.camera = cam


def render_model(path: str, out: str) -> None:
    """Import ``path``, render it to ``out`` and remove it again."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    imported = [o for o in bpy.data.objects if o not in before]
    tris = 0
    for obj in imported:
        if obj.type == "MESH":
            obj.data.calc_loop_triangles()
            tris += len(obj.data.loop_triangles)
    bpy.context.scene.render.filepath = out
    bpy.ops.render.render(write_still=True)
    print(f"GA3 sheet: {path} ({tris} tris) -> {out}")
    for obj in imported:
        bpy.data.objects.remove(obj)


def main() -> None:
    """Render one panel per model."""
    args = parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    setup_scene(args.size, args.yaw)
    for index, model in enumerate(args.models):
        render_model(model, f"{args.out_prefix}_{index}.png")


if __name__ == "__main__":
    main()
