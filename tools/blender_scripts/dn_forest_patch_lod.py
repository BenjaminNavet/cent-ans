"""DN-forets: decimate a generated forest patch glb for the campaign map.

Usage: blender -b --factory-startup --python tools/blender_scripts/dn_forest_patch_lod.py -- IN.glb OUT.glb [FACES]
"""

import sys

import bpy

args = sys.argv[sys.argv.index("--") + 1 :]
source, target = args[0], args[1]
faces = int(args[2]) if len(args) > 2 else 3000
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=source)
for obj in [o for o in bpy.context.scene.objects if o.type == "MESH"]:
    count = len(obj.data.polygons)
    modifier = obj.modifiers.new("decimate", "DECIMATE")
    modifier.ratio = min(1.0, faces / max(count, 1))
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=modifier.name)
    print(f"dn_forest_patch_lod: {obj.name} {count} -> {len(obj.data.polygons)} faces")
bpy.ops.export_scene.gltf(filepath=target, export_format="GLB")
