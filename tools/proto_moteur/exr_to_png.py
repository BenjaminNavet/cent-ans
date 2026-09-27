"""Convert the Unreal capture (linear EXR, 2x size) to an sRGB PNG at scene resolution (Blender headless).

Run: Blender -b --python exr_to_png.py -- in.exr out.png width height
"""

import sys

import bpy

source, target, width, height = sys.argv[sys.argv.index("--") + 1 :]
image = bpy.data.images.load(source)
image.colorspace_settings.name = "Linear Rec.709"
image.scale(int(width), int(height))
scene = bpy.context.scene
scene.render.image_settings.file_format = "PNG"
scene.view_settings.view_transform = "Standard"
image.save_render(target, scene=scene)
print("PROTO png written", target)
