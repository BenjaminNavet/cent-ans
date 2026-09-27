"""Convert the game's shader-driven glTF models into plain PBR glTF for both engines.

Buildings store their material layer in vertex-colour alpha (read by building_atlas.gdshader):
faces are split by layer into textured materials (same Poly Haven maps, tiles and tints as
BuildingMaterials.SPECS). Figures store flat colours in vertex RGB: one material per colour,
white (the livery slot) becomes the French azure. Vertex colours are then dropped.

Run: Blender -b --python materialize.py -- <asset name> <source glb> <output glb> ...
"""

import sys
from pathlib import Path

import bmesh
import bpy

REPO = Path(__file__).resolve().parents[2]
TEX = REPO / "game/assets/textures"
# layer name -> (albedo, normal, roughness map or "", tile metres, tint, roughness)
SPECS = {
    "Plaster": ("buildings/lime_plaster_diff", "buildings/medieval_wall_01_nor", "buildings/medieval_wall_01_rough", 2.2, (1.05, 1.02, 0.97), 0.95),
    "Rubble": ("buildings/stone_wall_diff", "buildings/stone_wall_nor", "buildings/stone_wall_rough", 2.4, (1.0, 1.0, 1.0), 0.95),
    "Ashlar": ("buildings/rustic_stone_wall_diff", "buildings/rustic_stone_wall_nor", "buildings/rustic_stone_wall_rough", 2.1, (1.15, 1.12, 1.06), 0.9),
    "Masonry": ("battle/castle_wall_varriation_diff", "battle/castle_wall_varriation_nor", "", 3.0, (1.05, 1.03, 0.98), 0.9),
    "Timber": ("buildings/rough_wood_diff", "buildings/rough_wood_nor", "buildings/rough_wood_rough", 1.6, (1.0, 1.0, 1.0), 0.9),
    "Planks": ("buildings/weathered_brown_planks_diff", "buildings/weathered_brown_planks_nor", "buildings/weathered_brown_planks_rough", 2.2, (1.0, 1.0, 1.0), 0.92),
    "Door": ("buildings/weathered_brown_planks_diff", "buildings/weathered_brown_planks_nor", "", 1.6, (0.7, 0.62, 0.55), 0.9),
    "RoofTile": ("buildings/clay_roof_tiles_03_diff", "buildings/clay_roof_tiles_03_nor", "buildings/clay_roof_tiles_03_rough", 2.4, (0.85, 0.78, 0.76), 0.85),
    "RoofFlat": ("buildings/roof_tiles_14_diff", "buildings/roof_tiles_14_nor", "buildings/roof_tiles_14_rough", 2.2, (1.0, 1.0, 1.0), 0.88),
    "RoofSlate": ("battle/roof_slates_02_diff", "battle/roof_slates_02_nor", "", 2.4, (0.5, 0.53, 0.61), 0.6),
    "Thatch": ("battle/thatch_roof_angled_diff", "battle/thatch_roof_angled_nor", "", 2.2, (1.45, 1.2, 0.85), 1.0),
}
PLAIN = {"Window": ((0.035, 0.032, 0.03), 0.35), "Iron": ((0.09, 0.09, 0.1), 0.5), "Canvas": ((0.78, 0.74, 0.66), 0.95)}
ATLAS_LAYERS = ["Plaster", "Rubble", "Ashlar", "Masonry", "Timber", "Planks", "Door", "RoofTile", "RoofFlat", "RoofSlate", "Thatch", "Window", "Iron", "Canvas"]
LIVERY = (0.08, 0.16, 0.45)


def srgb_to_linear(value: float) -> float:
    """Convert one sRGB channel to linear (Blender colours are linear)."""
    return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4


def image(path: str, non_color: bool):
    """Load a game texture (jpg) once."""
    img = bpy.data.images.load(str(TEX / f"{path}.jpg"), check_existing=True)
    if non_color:
        img.colorspace_settings.name = "Non-Color"
    return img


def textured_material(name: str):
    """Build a Principled material from a SPECS entry (tint folded into a base-colour factor)."""
    albedo, normal, rough, _tile, tint, roughness = SPECS[name]
    mat = bpy.data.materials.new(name)
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = image(albedo, False)
    if max(tint) != 1.0 or min(tint) != 1.0:
        # glTF has no albedo multiplier above 1: bake the tint into a copy of the image.
        tinted = tex.image.copy()
        tinted.name = f"{name}_albedo"
        pixels = list(tinted.pixels)
        for i in range(0, len(pixels), 4):
            for c in range(3):
                pixels[i + c] = min(1.0, pixels[i + c] * tint[c])
        tinted.pixels = pixels
        tinted.file_format = "JPEG"
        tex.image = tinted
    links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    nor = nodes.new("ShaderNodeTexImage")
    nor.image = image(normal, True)
    normal_map = nodes.new("ShaderNodeNormalMap")
    links.new(nor.outputs["Color"], normal_map.inputs["Color"])
    links.new(normal_map.outputs["Normal"], bsdf.inputs["Normal"])
    if rough:
        rough_tex = nodes.new("ShaderNodeTexImage")
        rough_tex.image = image(rough, True)
        # glTF reads roughness from the green channel of the metallic-roughness texture.
        sep = nodes.new("ShaderNodeSeparateColor")
        links.new(rough_tex.outputs["Color"], sep.inputs["Color"])
        links.new(sep.outputs["Green"], bsdf.inputs["Roughness"])
    else:
        bsdf.inputs["Roughness"].default_value = roughness
    return mat


def plain_material(name: str, color, roughness: float):
    """Build an untextured Principled material (colour given in sRGB)."""
    mat = bpy.data.materials.new(name)
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*[srgb_to_linear(c) for c in color], 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    return mat


def convert_building(obj) -> None:
    """Split faces by vertex-alpha layer, scale their UVs by the layer tile, assign materials."""
    mesh = obj.data
    colors = mesh.color_attributes[0]
    per_corner = colors.domain == "CORNER"
    bm = bmesh.new()
    bm.from_mesh(mesh)
    uv = bm.loops.layers.uv.active
    layer_of_face = {}
    for face in bm.faces:
        loop = face.loops[0]
        alpha = colors.data[loop.index if per_corner else loop.vert.index].color[3]
        layer_of_face[face.index] = min(max(int(alpha * 16.0), 0), len(ATLAS_LAYERS) - 1)
    obj.data.materials.clear()
    slots = {}
    for face in bm.faces:
        name = ATLAS_LAYERS[layer_of_face[face.index]]
        if name not in slots:
            slots[name] = len(slots)
            mat = textured_material(name) if name in SPECS else plain_material(name, *PLAIN[name])
            obj.data.materials.append(mat)
        face.material_index = slots[name]
        if name in SPECS:
            for loop in face.loops:
                loop[uv].uv = loop[uv].uv / SPECS[name][3]
    bm.to_mesh(mesh)
    bm.free()
    mesh.color_attributes.remove(colors)
    if obj.name == "haystack_0" or "Canvas" in slots:
        for mat in obj.data.materials:
            mat.use_backface_culling = False


def convert_figure(obj) -> None:
    """One flat material per vertex colour; white becomes the livery colour."""
    mesh = obj.data
    colors = mesh.color_attributes[0]
    per_corner = colors.domain == "CORNER"
    obj.data.materials.clear()
    slots = {}
    for poly in mesh.polygons:
        index = poly.loop_indices[0] if per_corner else poly.vertices[0]
        rgb = tuple(round(c, 2) for c in colors.data[index].color[:3])
        if rgb not in slots:
            slots[rgb] = len(slots)
            linear = LIVERY_LINEAR if rgb == (1.0, 1.0, 1.0) else rgb
            metal = abs(rgb[0] - rgb[2]) < 0.1 and 0.15 < rgb[0] < 0.5
            mat = bpy.data.materials.new(f"figure_{len(slots)}")
            bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
            bsdf.inputs["Base Color"].default_value = (*linear, 1.0)
            bsdf.inputs["Roughness"].default_value = 0.35 if metal else 0.8
            bsdf.inputs["Metallic"].default_value = 0.9 if metal else 0.0
            obj.data.materials.append(mat)
        poly.material_index = slots[rgb]
    mesh.color_attributes.remove(colors)


LIVERY_LINEAR = tuple(srgb_to_linear(c) for c in LIVERY)

args = sys.argv[sys.argv.index("--") + 1 :]
for name, source, output in zip(args[0::3], args[1::3], args[2::3], strict=True):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    for obj in [o for o in bpy.context.scene.objects if o.type == "MESH"]:
        if not obj.data.color_attributes:
            continue
        (convert_figure if name.startswith(("infantry", "archer")) else convert_building)(obj)
    bpy.ops.export_scene.gltf(filepath=output, export_format="GLB", export_image_format="JPEG", export_vertex_color="NONE")
    print("PROTO materialized", name)
