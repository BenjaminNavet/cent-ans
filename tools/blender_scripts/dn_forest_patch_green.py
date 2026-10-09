"""Push a forest patch glb texture toward foliage green (chroma x2.2, green bias). Usage: blender -b --python THIS -- IN.glb OUT.glb."""

import sys

import bpy
import numpy as np

src, dst = sys.argv[sys.argv.index("--") + 1 :][:2]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
for mat in bpy.data.materials:
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    link = bsdf.inputs["Base Color"].links[0]
    img = link.from_node.image
    px = np.array(img.pixels[:]).reshape(-1, 4)
    rgb = px[:, :3]
    lum = rgb @ np.array([0.2126, 0.7152, 0.0722])
    # chroma x2.2 around luminance, nudged toward foliage green, darkened a little
    rgb = lum[:, None] + (rgb - lum[:, None]) * 2.2
    rgb = rgb * np.array([0.82, 1.0, 0.62]) * 0.85
    px[:, :3] = np.clip(rgb, 0, 1)
    img.pixels[:] = px.ravel()
    img.pack()
bpy.ops.export_scene.gltf(filepath=dst, export_format="GLB")
