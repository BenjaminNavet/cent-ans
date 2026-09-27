"""Build the prototype ground mesh with its PBR material and export it to assets/ground.glb.

Run: Blender -b --python make_ground.py
"""

import sys
from pathlib import Path

import bpy

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import terrain  # noqa: E402

bpy.ops.wm.read_factory_settings(use_empty=True)
count = terrain.RESOLUTION + 1
step = terrain.SIZE_M / terrain.RESOLUTION
half = terrain.SIZE_M / 2
vertices, faces, uvs = [], [], []
for row in range(count):
    for col in range(count):
        x = -half + col * step
        z = -half + row * step
        # Blender is Z up and glTF export maps Blender -Y to glTF +Z.
        vertices.append((x, -z, terrain.height(x, z)))
for row in range(terrain.RESOLUTION):
    for col in range(terrain.RESOLUTION):
        a = row * count + col
        faces.append((a, a + 1, a + count + 1, a + count))
mesh = bpy.data.meshes.new("ground")
mesh.from_pydata(vertices, [], faces)
uv_layer = mesh.uv_layers.new(name="UVMap")
for loop in mesh.loops:
    vx, vy, _ = mesh.vertices[loop.vertex_index].co
    uv_layer.data[loop.index].uv = (vx / terrain.UV_TILE_M, vy / terrain.UV_TILE_M)
for polygon in mesh.polygons:
    polygon.use_smooth = True
ground = bpy.data.objects.new("ground", mesh)
bpy.context.scene.collection.objects.link(ground)

material = bpy.data.materials.new("ground_grass_path")
material.use_nodes = True
nodes = material.node_tree.nodes
links = material.node_tree.links
bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")


def image_node(name: str, non_color: bool):
    node = nodes.new("ShaderNodeTexImage")
    node.image = bpy.data.images.load(str(HERE / "assets" / f"grass_path_2_{name}.jpg"))
    if non_color:
        node.image.colorspace_settings.name = "Non-Color"
    return node


links.new(image_node("Diffuse", False).outputs["Color"], bsdf.inputs["Base Color"])
links.new(image_node("Rough", True).outputs["Color"], bsdf.inputs["Roughness"])
normal_map = nodes.new("ShaderNodeNormalMap")
links.new(image_node("nor_gl", True).outputs["Color"], normal_map.inputs["Color"])
links.new(normal_map.outputs["Normal"], bsdf.inputs["Normal"])
mesh.materials.append(material)

bpy.ops.export_scene.gltf(filepath=str(HERE / "assets" / "ground.glb"), export_format="GLB", export_image_format="JPEG")
print("PROTO ground exported")
