"""Push a forest patch glb texture toward foliage green (chroma x CHROMA, green bias).

Usage: blender -b --python THIS -- IN.glb OUT.glb [CHROMA=1.6].
"""

import sys

import bpy
import numpy as np

args = sys.argv[sys.argv.index("--") + 1 :]
src, dst = args[:2]
chroma = float(args[2]) if len(args) > 2 else 1.6
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
for mat in bpy.data.materials:
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    link = bsdf.inputs["Base Color"].links[0]
    img = link.from_node.image
    px = np.array(img.pixels[:]).reshape(-1, 4)
    rgb = px[:, :3]
    lum = rgb @ np.array([0.2126, 0.7152, 0.0722])
    # chroma boost around luminance, nudged toward a blue-green foliage (x2.2 + more red was too yellow)
    rgb = lum[:, None] + (rgb - lum[:, None]) * chroma
    rgb = rgb * np.array([0.75, 1.0, 0.7]) * 0.8
    px[:, :3] = np.clip(rgb, 0, 1)
    img.pixels[:] = px.ravel()
    img.pack()
bpy.ops.export_scene.gltf(filepath=dst, export_format="GLB")
