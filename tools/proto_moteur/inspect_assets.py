"""Print node names and bounding boxes of the prototype's glTF assets (Blender headless)."""

import sys

import bpy
from mathutils import Vector

for path in sys.argv[sys.argv.index("--") + 1 :]:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    print("ASSET", path.rsplit("/", 1)[-1])
    for obj in bpy.context.scene.objects:
        if obj.parent is None:
            corners = [
                obj.matrix_world @ Vector(c)
                for child in [obj, *obj.children_recursive]
                if child.type == "MESH"
                for c in child.bound_box
            ]
            if not corners:
                continue
            lo = [min(c[i] for c in corners) for i in range(3)]
            hi = [max(c[i] for c in corners) for i in range(3)]
            print(f"  {obj.name}: min {[round(v, 2) for v in lo]} max {[round(v, 2) for v in hi]}")
