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


def is_closed(bm: "bmesh.types.BMesh", key: list) -> bool:
    """True when every edge, compared by vertex position, is shared by exactly two faces."""
    counts: dict = {}
    for edge in bm.edges:
        pair = frozenset((key[edge.verts[0].index], key[edge.verts[1].index]))
        counts[pair] = counts.get(pair, 0) + len(edge.link_faces)
    return all(count == 2 for count in counts.values())


def clean(obj: "bpy.types.Object", island_min: float) -> int:
    """Drop debris islands (< ``island_min`` of the vertices) and fix normals of a single shell; return vertices removed.

    Islands are computed on positions, not on vertex indices: UV seams split vertices, and a
    chart cut by a seam must not be mistaken for debris. The mesh itself is not welded
    (welding breaks the later collapse decimation).
    """
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.verts.ensure_lookup_table()
    key = [tuple(round(c, 5) for c in v.co) for v in bm.verts]
    parent: dict = {}

    def find(item):
        parent.setdefault(item, item)
        while parent[item] != item:
            parent[item] = parent[parent[item]]
            item = parent[item]
        return item

    for k in key:
        find(k)
    for edge in bm.edges:
        parent[find(key[edge.verts[0].index])] = find(key[edge.verts[1].index])
    sizes: dict = {}
    for k in key:
        root = find(k)
        sizes[root] = sizes.get(root, 0) + 1
    threshold = island_min * len(key)
    debris = [
        v for v, k in zip(bm.verts, key, strict=True) if sizes[find(k)] < threshold
    ]
    bmesh.ops.delete(bm, geom=debris, context="VERTS")
    # Normals are recalculated only for a single closed shell (an open one has no reliable inside): on a scene of many shells (a town) the
    # recalculation flips whole buildings inside out, and back-face culling then shows torn walls
    # and black interiors.
    if len({find(k) for k in key}) == 1 and is_closed(bm, key):
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(obj.data)
    bm.free()
    return len(debris)


def strip_base(obj: "bpy.types.Object") -> int:
    """Remove a parasitic ground plate (TRELLIS models the image's floor); return faces removed.

    Detected when the vertices in the lowest 4 % of the height spread more than 8 % wider than
    the body (vertices above 12 % of the height) in X or Y. Then faces entirely outside the body
    footprint (+2 %) in the low zone, and faces entirely within the lowest 1.5 %, are deleted.
    """
    verts = [v.co for v in obj.data.vertices]
    low_z = min(c.z for c in verts)
    height = max(c.z for c in verts) - low_z
    body = [c for c in verts if c.z > low_z + 0.12 * height]
    foot = [c for c in verts if c.z < low_z + 0.04 * height]
    if not body or not foot or height <= 0:
        return 0
    span = {a: max(c[a] for c in body) - min(c[a] for c in body) for a in (0, 1)}
    wider = any(
        max(c[a] for c in foot) - min(c[a] for c in foot) > 1.08 * span[a]
        for a in (0, 1)
    )
    if not wider:
        return 0
    margin = {a: 0.02 * span[a] for a in (0, 1)}
    lo = {a: min(c[a] for c in body) - margin[a] for a in (0, 1)}
    hi = {a: max(c[a] for c in body) + margin[a] for a in (0, 1)}
    zone = low_z + 0.12 * height

    def outside(co) -> bool:
        return co.z < zone and not all(lo[a] <= co[a] <= hi[a] for a in (0, 1))

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    doomed = [
        f
        for f in bm.faces
        if all(v.co.z < low_z + 0.015 * height for v in f.verts)
        or all(outside(v.co) for v in f.verts)
    ]
    bmesh.ops.delete(bm, geom=doomed, context="FACES")
    bmesh.ops.delete(
        bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS"
    )
    bm.to_mesh(obj.data)
    bm.free()
    return len(doomed)


def set_surface(obj: "bpy.types.Object", roughness: float) -> None:
    """Constant roughness, metallic 0 (bible 14.4): drop metallic-roughness textures."""
    for slot in obj.material_slots:
        if not (slot.material and slot.material.use_nodes):
            continue
        tree = slot.material.node_tree
        for node in tree.nodes:
            if node.type != "BSDF_PRINCIPLED":
                continue
            for name in ("Roughness", "Metallic"):
                for link in list(node.inputs[name].links):
                    tree.links.remove(link)
            node.inputs["Roughness"].default_value = roughness
            node.inputs["Metallic"].default_value = 0.0


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
        cos, sin = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
        for vertex in obj.data.vertices:
            x, y = vertex.co.x, vertex.co.y
            vertex.co.x, vertex.co.y = x * cos - y * sin, x * sin + y * cos
        obj.data.update()
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
                gamma=spec.get("gamma"),
            )
            if spec.get("grade", True)
            else (pixels[:, :3], {"after": dn_grade.albedo_stats(pixels[:, :3])})
        )
        pixels[:, :3] = graded
        image.pixels.foreach_set(pixels.ravel())
        image.update()
    image.pack()
    return report


def collapse_passes(obj: "bpy.types.Object", target: int) -> int:
    """Up to 8 collapse passes toward ``target`` triangles; return the count."""
    for _ in range(8):
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


def weld(obj: "bpy.types.Object", distance: float) -> None:
    """Merge vertices closer than ``distance`` (loop UVs are kept, so seams stay textured)."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=bm.verts[:], dist=distance)
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def decimate(obj: "bpy.types.Object", target: int) -> int:
    """Collapse-decimate to roughly ``target`` triangles, return the count.

    Collapse stalls on meshes with many UV islands (seams block it); the fallback welds
    vertices at a growing distance (texture seams blur, harmless at low LOD) and retries.
    """
    low, high = bounds(obj)
    diagonal = max(high[i] - low[i] for i in range(3))
    weld(obj, diagonal * 1e-5)  # exact weld: UV-seam splits would otherwise tear on collapse
    count = collapse_passes(obj, target)
    for fraction in (0.002, 0.006, 0.015, 0.04, 0.08, 0.15, 0.25):
        if count <= target * 1.04:
            break
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bmesh.ops.remove_doubles(bm, verts=bm.verts[:], dist=diagonal * fraction)
        bm.to_mesh(obj.data)
        bm.free()
        obj.data.update()
        count = collapse_passes(obj, target)
    return count


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
    removed_debris = clean(obj, spec.get("island_min", 0.01))
    triangulate(obj)
    raw_tris = len(obj.data.polygons)
    dims = normalise(obj, spec)
    stripped = 0
    if spec.get("strip_base"):
        stripped = strip_base(obj)
        if stripped:
            dims = normalise(obj, {**spec, "yaw_deg": 0.0})
    set_surface(obj, spec["roughness"])
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
                "debris_vertices": removed_debris,
                "base_faces_removed": stripped,
                "grade": grade,
                "bbox_drift": drift,
            }
        )
    )


if __name__ == "__main__":
    main()
