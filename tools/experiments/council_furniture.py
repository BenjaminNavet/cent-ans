"""Hand-made council furniture (RG13-V23, option 3): trestle table, high-backed throne, stool.

Primitive boxes/cylinders with plain PBR materials. Metres, origin on the floor at the centre,
facing -Z (glTF export converts Blender's Z-up to Y-up, so the front is Blender +Y).
Usage: blender -b --python tools/experiments/council_furniture.py
Outputs: game/assets/models/council/handmade/council_{table,throne,stool}.glb
"""

import math
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "game" / "assets" / "models" / "council" / "handmade"

OAK = ((0.30, 0.20, 0.12, 1.0), 0.75)
DARK_WOOD = ((0.10, 0.065, 0.045, 1.0), 0.6)
OXBLOOD = ((0.28, 0.04, 0.04, 1.0), 0.95)
IRON = ((0.07, 0.07, 0.08, 1.0), 0.45)


def material(name: str, spec: tuple) -> bpy.types.Material:
    """Create a simple principled material (colour, roughness)."""
    color, roughness = spec
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Roughness"].default_value = roughness
    return mat


def box(name, size, location, mat, rotation=(0.0, 0.0, 0.0), bevel=0.012):
    """Add a bevelled box. size = (x, y, z) in Blender axes, location = centre."""
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = size
    bpy.ops.object.transform_apply(scale=True, rotation=False)
    if bevel:
        modifier = obj.modifiers.new("Bevel", "BEVEL")
        modifier.width = bevel
        modifier.segments = 2
        bpy.ops.object.modifier_apply(modifier="Bevel")
    obj.data.materials.append(mat)
    return obj


def cylinder(name, radius, depth, location, mat, rotation=(0.0, 0.0, 0.0), vertices=16):
    """Add a cylinder along its local Z axis."""
    bpy.ops.mesh.primitive_cylinder_add(
        radius=radius, depth=depth, location=location, rotation=rotation, vertices=vertices
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.data.materials.append(mat)
    return obj


def reset_scene() -> None:
    """Empty the scene."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def export(filename: str, objects: list, turn: bool = False) -> None:
    """Join `objects` into one mesh with smooth shading and export it as glb."""
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    bpy.ops.object.join()
    bpy.ops.object.shade_smooth()
    if turn:  # designed facing Blender -Y: turn it to face +Y (= glTF -Z)
        bpy.context.active_object.rotation_euler = (0.0, 0.0, math.pi)
        bpy.ops.object.transform_apply(rotation=True)
    OUT.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(OUT / filename), export_format="GLB", use_selection=True, export_apply=True
    )
    print("wrote", OUT / filename)


def build_table() -> None:
    """Trestle table 3.2 (long, Blender Y) x 1.1 x 0.85 m: thick oak top, two trestles, stretcher."""
    reset_scene()
    oak = material("oak", OAK)
    dark = material("oak_dark", DARK_WOOD)
    parts = [box("top", (1.1, 3.2, 0.09), (0, 0, 0.805), oak, bevel=0.015)]
    for index, y in enumerate((-1.1, 1.1)):  # trestles
        parts.append(box(f"trestle_beam_{index}", (0.9, 0.12, 0.1), (0, y, 0.72), dark))
        for side in (-1, 1):
            parts.append(
                box(f"leg_{index}_{side}", (0.09, 0.1, 0.78), (side * 0.36, y, 0.38), dark,
                    rotation=(0.0, side * 0.12, 0.0))
            )
            parts.append(box(f"foot_{index}_{side}", (0.12, 0.5, 0.07), (side * 0.40, y, 0.035), dark))
    parts.append(box("stretcher", (0.07, 2.2, 0.07), (0, 0, 0.33), dark))
    for end in (-1, 1):  # wedges through the stretcher
        parts.append(box(f"wedge_{end}", (0.04, 0.05, 0.14), (0, end * 1.1, 0.33), oak))
    export("council_table.glb", parts)


def build_throne() -> None:
    """High-backed dark-wood throne, 1.6 m tall, oxblood padded seat and arm rests, facing -Z."""
    reset_scene()
    wood = material("throne_wood", DARK_WOOD)
    cloth = material("oxblood", OXBLOOD)
    gold = material("iron", IRON)
    width, depth, seat_h = 0.62, 0.58, 0.48
    parts = [
        box("base", (width, depth, 0.1), (0, 0, 0.30), wood),
        box("seat_cushion", (width - 0.08, depth - 0.08, 0.1), (0, -0.0, seat_h), cloth, bevel=0.03),
    ]
    for sx in (-1, 1):
        for sy in (-1, 1):  # four legs
            parts.append(box("leg", (0.07, 0.07, 0.30), (sx * (width / 2 - 0.04), sy * (depth / 2 - 0.04), 0.15), wood))
        parts.append(box("arm_post", (0.06, 0.06, 0.3), (sx * (width / 2 + 0.01), -0.18, 0.62), wood))
        parts.append(box("arm_rest", (0.09, 0.5, 0.05), (sx * (width / 2 + 0.01), -0.02, 0.79), wood))
        parts.append(box("arm_pad", (0.07, 0.34, 0.03), (sx * (width / 2 + 0.01), -0.02, 0.83), cloth, bevel=0.01))
        parts.append(box("back_post", (0.09, 0.09, 1.6), (sx * (width / 2 + 0.0), depth / 2 - 0.05, 0.8), wood))
        parts.append(box("finial", (0.11, 0.11, 0.08), (sx * (width / 2), depth / 2 - 0.05, 1.62), gold))
    parts.append(box("back_panel", (width - 0.05, 0.06, 0.95), (0, depth / 2 - 0.05, 1.05), wood))
    parts.append(box("back_pad", (width - 0.2, 0.04, 0.6), (0, depth / 2 - 0.09, 1.0), cloth, bevel=0.02))
    parts.append(box("crest_rail", (width + 0.12, 0.1, 0.12), (0, depth / 2 - 0.05, 1.5), wood))
    parts.append(box("foot_rest", (0.5, 0.25, 0.08), (0, -depth / 2 - 0.12, 0.04), wood))
    export("council_throne.glb", parts, turn=True)


def build_stool() -> None:
    """Three-legged stool 0.45 m high with a round oak seat."""
    reset_scene()
    oak = material("stool_oak", OAK)
    dark = material("stool_legs", DARK_WOOD)
    parts = [cylinder("seat", 0.20, 0.05, (0, 0, 0.425), oak, vertices=24)]
    for index in range(3):
        angle = index * 2 * math.pi / 3 + math.pi / 2
        foot = (math.cos(angle) * 0.2, math.sin(angle) * 0.2)
        top = (math.cos(angle) * 0.12, math.sin(angle) * 0.12)
        centre = ((foot[0] + top[0]) / 2, (foot[1] + top[1]) / 2, 0.2)
        tilt = math.atan2(0.08, 0.4)
        leg = cylinder(
            f"leg{index}", 0.027, 0.41, centre, dark, rotation=(0.0, 0.0, 0.0), vertices=10
        )
        # tilt outward: rotate about the axis perpendicular to the radial direction
        leg.rotation_euler = (math.sin(angle) * tilt, -math.cos(angle) * tilt, 0.0)
        parts.append(leg)
    parts.append(cylinder("ring", 0.15, 0.025, (0, 0, 0.2), dark, vertices=3))
    export("council_stool.glb", parts)


if __name__ == "__main__":
    build_table()
    build_throne()
    build_stool()
