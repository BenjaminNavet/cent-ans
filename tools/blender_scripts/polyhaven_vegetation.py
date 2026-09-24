"""Convert a Poly Haven vegetation .blend into a game-ready GLB (Blender headless).

Usage:
    blender -b src.blend --python polyhaven_vegetation.py -- \
        <prep_dir> <asset> <out.glb> <max_tris> <object> [<object> ...]

- Keeps only the named objects (each becomes a root node placed at the origin).
- Decimates (collapse) every object above <max_tris> triangles (0 = never).
- Replaces the Poly Haven node materials by glTF-friendly Principled materials built from
  8-bit textures found in <prep_dir>: `<asset>_twig_diff_alpha.png` (RGBA) and
  `<asset>_twig_nor_gl.jpg` for twigs, `<asset>_<part>_diff.jpg` / `_nor_gl.jpg` for
  bark and trunks, `<asset>_diff_alpha.png` for grass.
- Alpha-tested materials must then be set to MASK with `glb_alpha_mask.py`.

Used by agent D0 (third-party assets, see docs/wip/d0-assets.md).
"""

import os
import sys

import bpy

args = sys.argv[sys.argv.index("--") + 1 :]
prep_dir, asset, out_path, max_tris = args[0], args[1], args[2], int(args[3])
keep = args[4:]


def tri_count(obj) -> int:
    """Return the triangle count of a mesh object."""
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)


def load_image(name, non_color=False):
    """Load an image from the prep directory, or None if missing."""
    if not name:
        return None
    path = os.path.join(prep_dir, name)
    if not os.path.exists(path):
        return None
    img = bpy.data.images.load(path, check_existing=True)
    if non_color:
        img.colorspace_settings.name = "Non-Color"
    return img


def textures_for(mat_name):
    """Map a Poly Haven material name to (diffuse, normal) prepared texture names."""
    if mat_name.endswith("_twig"):
        return f"{asset}_twig_diff_alpha.png", f"{asset}_twig_nor_gl.jpg"
    for part in ("trunk_a", "trunk_b", "trunk_c"):
        if mat_name.endswith(part):
            return f"{asset}_{part}_diff.jpg", f"{asset}_{part}_nor_gl.jpg"
    if mat_name.endswith(("_bark", "_dead_branches")):
        return f"{asset}_bark_diff.jpg", f"{asset}_bark_nor_gl.jpg"
    return f"{asset}_diff_alpha.png", None


def build_material(src_name):
    """Create a simple Principled material using the prepared textures."""
    diff_name, nor_name = textures_for(src_name)
    mat = bpy.data.materials.new(src_name + "_game")
    mat.use_nodes = True
    tree = mat.node_tree
    bsdf = next(n for n in tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Roughness"].default_value = 0.9
    bsdf.inputs["Metallic"].default_value = 0.0
    diff = load_image(diff_name)
    if diff:
        tex = tree.nodes.new("ShaderNodeTexImage")
        tex.image = diff
        tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
        if diff_name.endswith(".png"):
            tree.links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
    nor = load_image(nor_name, non_color=True)
    if nor:
        ntex = tree.nodes.new("ShaderNodeTexImage")
        ntex.image = nor
        nmap = tree.nodes.new("ShaderNodeNormalMap")
        tree.links.new(ntex.outputs["Color"], nmap.inputs["Color"])
        tree.links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])
    return mat


for obj in list(bpy.data.objects):
    if obj.name not in keep:
        bpy.data.objects.remove(obj, do_unlink=True)

scene_coll = bpy.context.scene.collection
report = []
game_mats = {}
for name in keep:
    obj = bpy.data.objects[name]
    for coll in list(obj.users_collection):
        coll.objects.unlink(obj)
    scene_coll.objects.link(obj)
    obj.hide_viewport = False
    obj.hide_render = False
    obj.parent = None
    obj.location = (0.0, 0.0, 0.0)
    before = tri_count(obj)
    if max_tris and before > max_tris:
        mod = obj.modifiers.new("decimate", "DECIMATE")
        mod.decimate_type = "COLLAPSE"
        mod.ratio = max_tris / before
        mod.use_collapse_triangulate = True
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.modifier_apply(modifier=mod.name)
    for slot in obj.material_slots:
        if slot.material:
            src = slot.material.name
            if src not in game_mats:
                game_mats[src] = build_material(src)
            slot.material = game_mats[src]
    report.append((name, before, tri_count(obj)))

bpy.ops.object.select_all(action="DESELECT")
for name in keep:
    bpy.data.objects[name].select_set(True)
bpy.ops.export_scene.gltf(
    filepath=out_path,
    export_format="GLB",
    use_selection=True,
    export_apply=True,
    export_animations=False,
)
for name, before, after in report:
    print(f"REPORT {name} tris {before} -> {after}")
