"""DN ingest: raw TRELLIS/SF3D glb -> game-ready LOD glb set (bible 14.4-14.6).

Run through ``cent-ans dn-ingest`` (never by hand): arguments after ``--`` are a single JSON
document (see ``tools/cent_ans_tools/dn_ingest.py``). Steps: import, join meshes, bake
transforms, yaw, uniform scale to the target size in metres, pivot at ground centre, +Y up,
albedo grade, quadric decimation to each LOD budget, export one glb per LOD.
Prints one ``DN_RESULT {json}`` line.
"""

import json
import math
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import dn_grade  # noqa: E402


def clear_scene() -> None:
    """Remove every object and orphan datablock."""
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    for block in (bpy.data.meshes, bpy.data.materials, bpy.data.images):
        for item in list(block):
            block.remove(item)


def import_joined(path: str) -> "bpy.types.Object":
    """Import a glb and join all meshes into one object with transforms applied."""
    bpy.ops.import_scene.gltf(filepath=path)
    meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    if not meshes:
        raise SystemExit("no mesh in " + path)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in meshes:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = meshes[0]
    bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    if len(meshes) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    for other in list(bpy.context.scene.objects):
        if other is not obj:
            bpy.data.objects.remove(other, do_unlink=True)
    return obj


def triangulate(obj: "bpy.types.Object") -> None:
    """Triangulate the mesh in place."""
    mesh = obj.data
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    bm.to_mesh(mesh)
    bm.free()


def bounds(obj: "bpy.types.Object") -> tuple[list[float], list[float]]:
    """World bounding box (min, max) in Blender axes (Z up)."""
    coords = [obj.matrix_world @ v.co for v in obj.data.vertices]
    low = [min(c[i] for c in coords) for i in range(3)]
    high = [max(c[i] for c in coords) for i in range(3)]
    return low, high


def normalise(obj: "bpy.types.Object", spec: dict) -> dict:
    """Yaw, scale to target metres, pivot to ground centre. Blender Z is up (glTF exporter maps to +Y)."""
    yaw = spec.get("yaw_deg", 0.0)
    if yaw:
        obj.rotation_euler = (0.0, 0.0, math.radians(yaw))
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    low, high = bounds(obj)
    size = [high[i] - low[i] for i in range(3)]  # x, y (depth), z (height)
    extent = {"length": max(size[0], size[1]), "width": size[0], "height": size[2]}
    scale = spec["target_m"] / extent[spec["axis"]]
    for vertex in obj.data.vertices:
        vertex.co.x = (vertex.co.x - (low[0] + high[0]) / 2) * scale
        vertex.co.y = (vertex.co.y - (low[1] + high[1]) / 2) * scale
        vertex.co.z = (vertex.co.z - low[2]) * scale
    obj.data.update()
    low, high = bounds(obj)
    return {
        "length": round(max(high[0] - low[0], high[1] - low[1]), 3),
        "width": round(high[0] - low[0], 3),
        "height": round(high[2] - low[2], 3),
    }


def base_color_image(obj: "bpy.types.Object") -> "bpy.types.Image | None":
    """Return the albedo image of the first material, if any."""
    for slot in obj.material_slots:
        if slot.material and slot.material.use_nodes:
            for node in slot.material.node_tree.nodes:
                if node.type == "BSDF_PRINCIPLED":
                    link = node.inputs["Base Color"].links
                    if link and link[0].from_node.type == "TEX_IMAGE":
                        return link[0].from_node.image
    return None


def grade_and_resize(obj: "bpy.types.Object", spec: dict) -> dict | None:
    """Grade the albedo to the world palette and resize it to the class texture size."""
    image = base_color_image(obj)
    if image is None:
        return None
    tex = spec["tex"]
    if max(image.size) > tex:
        image.scale(tex, tex)
    width, height = image.size
    pixels = np.empty(width * height * 4, dtype=np.float32)
    image.pixels.foreach_get(pixels)
    pixels = pixels.reshape(-1, 4)
    if image.colorspace_settings.name != "sRGB":
        report = None
    else:
        graded, report = (
            dn_grade.grade_albedo(
                pixels[:, :3].astype(np.float64),
                saturation_cap=spec["saturation_cap"],
                luma_range=tuple(spec["luma_range"])
                if spec.get("luma_range")
                else None,
                gamma=spec.get("gamma", 1.0),
            )
            if spec.get("grade", True)
            else (pixels[:, :3], {"after": dn_grade.albedo_stats(pixels[:, :3])})
        )
        pixels[:, :3] = graded
        image.pixels.foreach_set(pixels.ravel())
        image.update()
    image.pack()
    return report


def decimate(obj: "bpy.types.Object", target: int) -> int:
    """Collapse-decimate to roughly ``target`` triangles (two passes), return the count."""
    for _ in range(3):
        current = len(obj.data.polygons)
        if current <= target * 1.02:
            break
        modifier = obj.modifiers.new("dn_decimate", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = max(target / current, 0.01)
        modifier.use_collapse_triangulate = True
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    return len(obj.data.polygons)


def regrind(obj: "bpy.types.Object") -> dict:
    """Re-centre on the ground after decimation (vertices move slightly); return dimensions."""
    low, high = bounds(obj)
    shift = ((low[0] + high[0]) / 2, (low[1] + high[1]) / 2, low[2])
    for vertex in obj.data.vertices:
        vertex.co.x -= shift[0]
        vertex.co.y -= shift[1]
        vertex.co.z -= shift[2]
    obj.data.update()
    return {
        "length": round(max(high[0] - low[0], high[1] - low[1]), 3),
        "width": round(high[0] - low[0], 3),
        "height": round(high[2] - low[2], 3),
    }


def export(obj: "bpy.types.Object", path: str) -> None:
    """Export the object alone as glb, +Y up, JPEG textures."""
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.export_scene.gltf(
        filepath=path,
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_image_format="JPEG",
        export_jpeg_quality=88,
        export_apply=True,
    )


def main() -> None:
    """Entry point: run the pipeline described by the JSON argument."""
    spec = json.loads(sys.argv[sys.argv.index("--") + 1])
    clear_scene()
    obj = import_joined(spec["raw"])
    raw_tris = None
    triangulate(obj)
    raw_tris = len(obj.data.polygons)
    dims = normalise(obj, spec)
    grade = grade_and_resize(obj, spec)
    out_dir = Path(spec["out_dir"])
    out_dir.mkdir(parents=True, exist_ok=True)
    triangles = []
    drift = []
    ref_box = None
    for lod, target in enumerate(spec["lod_tris"][: spec["lods"]]):
        count = decimate(obj, target)
        dims = regrind(obj) if lod == 0 else dims
        if lod:
            regrind(obj)
        low, high = bounds(obj)
        box = [high[i] - low[i] for i in range(3)]
        ref_box = ref_box or box
        drift.append(
            round(
                max(abs(box[i] - ref_box[i]) for i in range(3)) / max(ref_box[2], 1e-6),
                4,
            )
        )
        export(obj, str(out_dir / f"{spec['id']}_lod{lod}.glb"))
        triangles.append(count)
    print(
        "DN_RESULT "
        + json.dumps(
            {
                "dimensions_m": dims,
                "triangles": triangles,
                "raw_triangles": raw_tris,
                "grade": grade,
                "bbox_drift": drift,
            }
        )
    )


if __name__ == "__main__":
    main()
