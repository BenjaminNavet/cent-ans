"""Contact sheet of one settlement family (lot GC3): every model of the family in one image.

Run headless:
    blender --background --python settlements_sheet.py -- <family> [out.png]

``family`` is ``west`` (the original kit) or one of ``settlements_east.FAMILIES``. The ten
models are laid on a grid (one column per kind, variant ``a`` behind variant ``b``), seen from
three quarters above under a plain sun, on a neutral ground, with their names. The models are
built straight from the generators with their palette colours (no kit atlas), which is how
the flat-coloured families look in Godot. Default output: ``docs/img/gc/kit_<family>.png``
(untracked). A line ``SIZE <name> <width> <depth> <height> <triangles>`` is printed per model.
"""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import bpy  # noqa: E402
import models as m  # noqa: E402
import settlements  # noqa: E402
import settlements_east as east  # noqa: E402
from mathutils import Vector  # noqa: E402

CELL = 4.0
IMAGE_WIDTH = 1600
ELEVATION = math.radians(40.0)  # camera height above the horizon
YAW = math.radians(-28.0)  # each model is turned by this angle (three-quarter view)
REPO = Path(__file__).resolve().parents[2]


def family_names(family: str) -> list[str]:
    """Model names of a family, kind by kind then variant."""
    if family == "west":
        return [f"{kind}_{variant}" for kind in east.KINDS for variant in east.VARIANTS]
    return east.model_names(family)


def build_joined(name: str) -> bpy.types.Object:
    """Build one model and join its parts (palette materials kept)."""
    parts = settlements.MODELS[name]()
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    bpy.ops.object.join()
    obj = bpy.context.active_object
    obj.name = name
    bpy.ops.object.shade_flat()
    return obj


def flat_material(name: str, color) -> bpy.types.Material:
    """Plain diffuse material for the ground and the labels."""
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = next(node for node in mat.node_tree.nodes if node.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = 1.0
    return mat


def add_label(text: str, location, rotation, material) -> None:
    """Text facing the camera."""
    bpy.ops.object.text_add(location=location, rotation=rotation)
    obj = bpy.context.active_object
    obj.data.body = text
    obj.data.size = 0.42
    obj.data.align_x = "CENTER"
    obj.data.materials.append(material)
    obj.visible_shadow = False


def setup_render(columns: int, rows: int, out: Path) -> None:
    """Orthographic camera over the grid, sun, neutral world, Cycles."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 24
    scene.cycles.use_denoising = False
    scene.view_settings.view_transform = "Standard"
    world = bpy.data.worlds.new("neutral")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (
        0.5,
        0.52,
        0.55,
        1.0,
    )
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.7
    scene.world = world
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 3.2
    sun.data.angle = math.radians(3.0)
    sun.rotation_euler = (math.radians(50.0), 0.0, math.radians(35.0))
    scene.collection.objects.link(sun)

    width, depth = columns * CELL, rows * CELL
    seen_depth = depth * math.sin(ELEVATION) + 2.2 * math.cos(ELEVATION)
    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = width
    camera.rotation_euler = (math.pi / 2 - ELEVATION, 0.0, 0.0)
    centre = Vector((width / 2 - CELL / 2, -depth / 2 + CELL / 2, 0.45))
    back = Vector((0.0, -math.cos(ELEVATION), math.sin(ELEVATION)))
    camera.location = centre + back * 60.0
    camera.data.clip_end = 200.0
    scene.collection.objects.link(camera)
    scene.camera = camera
    scene.render.resolution_x = IMAGE_WIDTH
    scene.render.resolution_y = int(IMAGE_WIDTH * seen_depth / width)
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.filepath = str(out)


def main() -> None:
    """Parse ``-- <family> [out.png]`` and render the sheet."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    family = args[0] if args else "med"
    out = (
        Path(args[1])
        if len(args) > 1
        else REPO / "docs" / "img" / "gc" / f"kit_{family}.png"
    )
    out.parent.mkdir(parents=True, exist_ok=True)
    m.KIT = family == "west"
    m.reset_scene()
    names = family_names(family)
    columns, rows = len(east.KINDS), len(east.VARIANTS)
    ink = flat_material("Ink", (0.02, 0.02, 0.02))
    label_rotation = (math.pi / 2 - ELEVATION, 0.0, 0.0)
    for index, name in enumerate(names):
        column, row = divmod(index, rows)
        obj = build_joined(name)
        triangles = m.triangle_count(obj)
        size = obj.dimensions
        print(f"SIZE {name} {size.x:.2f} {size.y:.2f} {size.z:.2f} {triangles}")
        obj.rotation_euler = (0.0, 0.0, YAW)
        obj.location = (column * CELL, -row * CELL, 0.0)
        add_label(
            f"{name}  {triangles}",
            (column * CELL, -row * CELL - CELL * 0.47, 0.05),
            label_rotation,
            ink,
        )
    bpy.ops.mesh.primitive_plane_add(
        size=400.0, location=(columns * CELL / 2, -rows * CELL / 2, 0.0)
    )
    bpy.context.active_object.data.materials.append(
        flat_material("Backdrop", (0.33, 0.36, 0.27))
    )
    setup_render(columns, rows, out)
    bpy.ops.render.render(write_still=True)
    print(f"SHEET {out}")


if __name__ == "__main__":
    main()
