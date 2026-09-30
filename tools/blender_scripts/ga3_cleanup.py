"""GA3: clean an image-to-3D mesh (fal.ai TRELLIS output) into game-ready static LODs.

Run headless from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/ga3_cleanup.py -- \
        SOURCE.glb OUT_DIR NAME [--length 8.0] [--lod0 8000] [--tex 1024] \
        [--strip-base 0.0] [--raw-render PATH.png]

Writes ``OUT_DIR/NAME_lod0.glb``, ``NAME_lod1.glb`` and ``NAME_lod2.glb`` (plan
``docs/wip/ga.md`` section GA3, probe S1 in ``docs/wip/ga3.md``):

1. Import, join, apply transforms; merge duplicate vertices, drop small loose islands
   (floating TRELLIS debris) and recompute outward normals.
2. Real scale: the longest horizontal side becomes ``--length`` metres. Origin on the
   ground (z = 0) at the centre of the footprint. Optional ``--strip-base H`` removes the
   ground patch TRELLIS reconstructs from the image (faces below ``H`` metres, and low
   faces outside the building's footprint), then scales again.
3. Collapse decimation: LOD0 <= ``--lod0`` triangles, LOD1 ~50 % and LOD2 ~15 % of LOD0.
4. Fresh UV (smart projection) per LOD and a Cycles bake of the source albedo (diffuse
   colour only, selected-to-active from the cleaned source mesh): ``--tex`` px square for
   LOD0, half for LOD1, quarter for LOD2.
5. glTF export (Z up in Blender -> Y up in Godot), one mesh, one opaque rough material,
   texture embedded as JPEG.

``--raw-render`` also writes a Workbench render of the untouched import (textured), used
for the probe's comparison sheet.
"""

import argparse
import math
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

ISLAND_MIN_FRACTION = 0.01  # islands below 1 % of the vertices are debris
MERGE_DISTANCE = 1e-5  # in source units (TRELLIS meshes fit in a unit cube)
LOD_FRACTIONS = (1.0, 0.5, 0.15)


def parse_args() -> argparse.Namespace:
    """Parse the arguments after ``--``."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source")
    parser.add_argument("out_dir")
    parser.add_argument("name")
    parser.add_argument("--length", type=float, default=8.0)
    parser.add_argument("--lod0", type=int, default=8000)
    parser.add_argument("--tex", type=int, default=1024)
    parser.add_argument("--strip-base", type=float, default=0.0)
    parser.add_argument("--raw-render", default="")
    return parser.parse_args(argv)


def triangle_count(obj: bpy.types.Object) -> int:
    """Number of triangles of a mesh object."""
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def select_only(*objects: bpy.types.Object) -> None:
    """Select exactly ``objects``, the first one active."""
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]


def import_joined(path: str) -> bpy.types.Object:
    """Import a glb and return its meshes joined into one object, transforms applied."""
    bpy.ops.import_scene.gltf(filepath=path)
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    select_only(*meshes)
    if len(meshes) > 1:
        bpy.ops.object.join()
    source = bpy.context.view_layer.objects.active
    bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    for obj in list(bpy.data.objects):
        if obj.type != "MESH":
            bpy.data.objects.remove(obj)
    source.name = "source"
    return source


def render_raw(path: str) -> None:
    """Workbench render (texture colours, soft studio light) of the current scene."""
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "TEXTURE"
    scene.display.shading.show_shadows = False
    scene.display.shading.show_cavity = True
    scene.render.resolution_x = scene.render.resolution_y = 800
    scene.render.film_transparent = False
    world = bpy.data.worlds.new("raw") if scene.world is None else scene.world
    scene.world = world
    world.color = (0.5, 0.5, 0.5)
    scene.view_settings.view_transform = "Standard"
    meshes = [o for o in scene.objects if o.type == "MESH"]
    lo = Vector(min(v[i] for o in meshes for v in o.bound_box) for i in range(3))
    hi = Vector(max(v[i] for o in meshes for v in o.bound_box) for i in range(3))
    centre, radius = (lo + hi) / 2, (hi - lo).length / 2
    cam_data = bpy.data.cameras.new("raw_cam")
    cam_data.lens = 50
    cam = bpy.data.objects.new("raw_cam", cam_data)
    scene.collection.objects.link(cam)
    direction = Vector((1.0, -1.2, 0.7)).normalized()
    cam.location = centre + direction * radius * 2.3
    cam.rotation_euler = (-direction).to_track_quat("-Z", "Y").to_euler()
    scene.camera = cam
    scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    bpy.data.objects.remove(cam)


def clean(obj: bpy.types.Object) -> None:
    """Merge duplicates, drop debris islands, recompute outward normals."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=MERGE_DISTANCE)
    bm.verts.ensure_lookup_table()
    seen: set[int] = set()
    islands: list[list[bmesh.types.BMVert]] = []
    for start in bm.verts:
        if start.index in seen:
            continue
        stack, island = [start], []
        seen.add(start.index)
        while stack:
            vert = stack.pop()
            island.append(vert)
            for edge in vert.link_edges:
                other = edge.other_vert(vert)
                if other.index not in seen:
                    seen.add(other.index)
                    stack.append(other)
        islands.append(island)
    threshold = ISLAND_MIN_FRACTION * len(bm.verts)
    debris = [v for island in islands if len(island) < threshold for v in island]
    bmesh.ops.delete(bm, geom=debris, context="VERTS")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(obj.data)
    bm.free()
    print(f"GA3 clean: {len(islands)} islands, {len(debris)} debris vertices removed")


def normalise(obj: bpy.types.Object, length: float) -> None:
    """Scale to ``length`` metres (longest horizontal side), origin at the footprint centre."""
    verts = [v.co for v in obj.data.vertices]
    lo = Vector(min(c[i] for c in verts) for i in range(3))
    hi = Vector(max(c[i] for c in verts) for i in range(3))
    factor = length / max(hi.x - lo.x, hi.y - lo.y)
    offset = Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z))
    for vert in obj.data.vertices:
        vert.co = (vert.co - offset) * factor
    obj.data.update()


def strip_base(obj: bpy.types.Object, height: float, length: float) -> None:
    """Remove the ground patch TRELLIS models from the image (grass disc, slab).

    Deletes faces lying entirely below ``height`` metres, and faces lying entirely outside
    the building's footprint (bounding box of the vertices higher than ``height`` plus 8 %
    of ``length``, i.e. walls and roof) below that level (tufts on the slab edge).
    """
    level = height + 0.08 * length
    upper = [v.co for v in obj.data.vertices if v.co.z > level]
    lo_x, hi_x = min(c.x for c in upper) - 0.05, max(c.x for c in upper) + 0.05
    lo_y, hi_y = min(c.y for c in upper) - 0.05, max(c.y for c in upper) + 0.05

    def outside(co: Vector) -> bool:
        return co.z < level and not (lo_x <= co.x <= hi_x and lo_y <= co.y <= hi_y)

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    doomed = [
        f
        for f in bm.faces
        if all(v.co.z < height for v in f.verts) or all(outside(v.co) for v in f.verts)
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    loose = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="VERTS")
    bm.to_mesh(obj.data)
    bm.free()
    print(f"GA3 strip-base: {len(doomed)} faces of the ground patch removed")


def decimated_copy(
    source: bpy.types.Object, name: str, target: int
) -> bpy.types.Object:
    """Copy of ``source`` collapse-decimated to at most ``target`` triangles."""
    obj = source.copy()
    obj.data = source.data.copy()
    obj.name = name
    bpy.context.scene.collection.objects.link(obj)
    select_only(obj)
    triangulate = obj.modifiers.new("tri", "TRIANGULATE")
    bpy.ops.object.modifier_apply(modifier=triangulate.name)
    ratio = min(1.0, target / max(1, triangle_count(obj)))
    for _ in range(4):
        decimate = obj.modifiers.new("dec", "DECIMATE")
        decimate.decimate_type = "COLLAPSE"
        decimate.ratio = ratio
        decimate.use_collapse_triangulate = True
        bpy.ops.object.modifier_apply(modifier=decimate.name)
        count = triangle_count(obj)
        if count <= target:
            break
        ratio = 0.97 * target / count
    return obj


def bake_albedo(
    source: bpy.types.Object, low: bpy.types.Object, size: int
) -> bpy.types.Image:
    """Smart-UV ``low`` and bake the source diffuse colour into a ``size`` px image."""
    low.data.materials.clear()
    for layer in list(low.data.uv_layers):
        low.data.uv_layers.remove(layer)
    low.data.uv_layers.new(name="UVMap")
    select_only(low)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.uv.smart_project(angle_limit=math.radians(66.0), island_margin=0.004)
    bpy.ops.object.mode_set(mode="OBJECT")

    image = bpy.data.images.new(f"{low.name}_albedo", size, size, alpha=False)
    material = bpy.data.materials.new(f"{low.name}_mat")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Metallic"].default_value = 0.0
    bsdf.inputs["Roughness"].default_value = 0.9
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = image
    material.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    nodes.active = tex
    low.data.materials.append(material)

    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 4
    bake = scene.render.bake
    bake.use_selected_to_active = True
    bake.cage_extrusion = 0.05
    bake.max_ray_distance = 0.3
    bake.margin = max(2, size // 128)
    bake.use_pass_direct = False
    bake.use_pass_indirect = False
    bake.use_pass_color = True
    select_only(low, source)
    source.select_set(True)
    bpy.context.view_layer.objects.active = low
    bpy.ops.object.bake(type="DIFFUSE", pass_filter={"COLOR"})
    image.pack()
    return image


def export(obj: bpy.types.Object, path: Path) -> None:
    """Export a single object as glb with its texture embedded as JPEG."""
    select_only(obj)
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_image_format="JPEG",
        export_yup=True,
        export_normals=True,
        export_tangents=False,
        export_animations=False,
    )


def main() -> None:
    """Run the cleanup pipeline."""
    args = parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    source = import_joined(args.source)
    raw_tris = triangle_count(source)
    if args.raw_render:
        render_raw(args.raw_render)
    for slot in source.material_slots:
        bsdf = (
            slot.material.node_tree.nodes.get("Principled BSDF")
            if slot.material
            else None
        )
        if bsdf is not None:
            bsdf.inputs["Metallic"].default_value = 0.0
    clean(source)
    normalise(source, args.length)
    if args.strip_base > 0.0:
        strip_base(source, args.strip_base, args.length)
        normalise(source, args.length)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    lod0 = None
    for level, fraction in enumerate(LOD_FRACTIONS):
        target = int(args.lod0 * fraction)
        name = f"{args.name}_lod{level}"
        low = decimated_copy(source if lod0 is None else lod0, name, target)
        bake_albedo(source, low, max(128, args.tex >> level))
        export(low, out_dir / f"{name}.glb")
        dims = low.dimensions
        print(
            f"GA3 {name}: {triangle_count(low)} tris (raw {raw_tris}), "
            f"{dims.x:.2f} x {dims.y:.2f} x {dims.z:.2f} m"
        )
        if lod0 is None:
            lod0 = low


if __name__ == "__main__":
    main()
