"""Blender side of the building kit (lot BR1, ADR 0021): meshes, glTF export and previews.

Run headless:
    blender --background --python kit_export.py -- export <out_dir> [kind ...]
    blender --background --python kit_export.py -- preview <prefix> [kind[:seed][!] ...] [--low]

``export`` writes the battle set (``high`` detail, a few seeds per kind, burned ruins) to
``<out_dir>/<kind>_<n>.glb`` plus ``manifest.json`` (footprint, height, triangles per model).
``preview`` renders each building alone with Eevee (textures from
``game/assets/textures/buildings/``) to judge the look without starting Godot.
GLB files carry named materials, metric UVs and vertex colours only; Godot swaps the materials
(``game/scripts/visual/building_materials.gd``).
"""

import contextlib
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import battle_skinned as bs  # noqa: E402
import bpy  # noqa: E402
import building_kit as kit  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TEXTURES = ROOT / "game" / "assets" / "textures" / "buildings"
QUATERNIUS = (
    ROOT / "game" / "assets" / "third_party" / "animals" / "quaternius_animated_animals"
)

# name: (texture id or None, texture tile size in metres, fallback linear colour, roughness)
MATERIALS = {
    "Plaster": ("lime_plaster", 2.2, (0.62, 0.58, 0.5), 0.95),
    "Rubble": ("stone_wall", 2.4, (0.36, 0.33, 0.28), 0.95),
    "Ashlar": ("rustic_stone_wall", 2.1, (0.42, 0.39, 0.33), 0.9),
    "Masonry": ("battle/castle_wall_varriation", 3.0, (0.45, 0.43, 0.4), 0.9),
    "Timber": ("rough_wood", 1.6, (0.2, 0.15, 0.11), 0.9),
    "Planks": ("weathered_brown_planks", 2.2, (0.24, 0.19, 0.14), 0.92),
    "Door": ("weathered_brown_planks", 1.6, (0.14, 0.1, 0.07), 0.9),
    "RoofTile": ("clay_roof_tiles_03", 2.4, (0.3, 0.12, 0.07), 0.85),
    "RoofFlat": ("roof_tiles_14", 2.2, (0.22, 0.12, 0.08), 0.88),
    "RoofSlate": ("battle/roof_slates_02", 2.4, (0.07, 0.075, 0.085), 0.7),
    "Thatch": ("battle/thatch_roof_angled", 2.2, (0.28, 0.22, 0.12), 1.0),
    "Window": (None, 1.0, (0.012, 0.011, 0.01), 0.35),
    "Iron": (None, 1.0, (0.05, 0.05, 0.055), 0.5),
    "Canvas": (None, 1.0, (0.55, 0.5, 0.42), 0.95),
    "Banner": (None, 1.0, (0.7, 0.08, 0.08), 0.8),
}

# Material-wide albedo multiplier (same values in building_materials.gd).
MATERIAL_TINT = {
    "RoofSlate": (0.5, 0.53, 0.61),
    "Thatch": (1.45, 1.2, 0.85),
    "RoofTile": (0.85, 0.78, 0.76),
    "Plaster": (1.05, 1.02, 0.97),
    "Door": (0.7, 0.62, 0.55),
}


# Atlas: every kit material becomes a layer of one ``Building`` material (same order as
# ``building_materials.gd`` ATLAS_LAYERS and ``build_textures.py`` LAYERS); the layer index is
# stored in the vertex colour alpha as (index + 0.5) / 16.
ATLAS_LAYERS = [
    "Plaster",
    "Rubble",
    "Ashlar",
    "Masonry",
    "Timber",
    "Planks",
    "Door",
    "RoofTile",
    "RoofFlat",
    "RoofSlate",
    "Thatch",
    "Window",
    "Iron",
    "Canvas",
]


def atlas(obj) -> None:
    """Merge the kit materials of an object into one ``Building`` material (layer in alpha)."""
    mesh = obj.data
    colors = mesh.color_attributes.get("Color")
    names = [
        slot.material.name.split(".")[0] if slot.material else ""
        for slot in obj.material_slots
    ]
    keep = [n for n in dict.fromkeys(names) if n not in ATLAS_LAYERS]
    order = keep + ["Building"]
    targets = []
    for poly in mesh.polygons:
        name = names[poly.material_index] if poly.material_index < len(names) else ""
        if name in ATLAS_LAYERS:
            alpha = (ATLAS_LAYERS.index(name) + 0.5) / 16.0
            targets.append(order.index("Building"))
            if colors is not None:
                for li in poly.loop_indices:
                    c = colors.data[li].color
                    colors.data[li].color = (c[0], c[1], c[2], alpha)
        else:
            targets.append(order.index(name))
    old = {
        n: next(
            slot.material
            for slot in obj.material_slots
            if slot.material and slot.material.name.split(".")[0] == n
        )
        for n in keep
    }
    building = bpy.data.materials.get("Building") or bpy.data.materials.new("Building")
    # Clearing the slots resets the polygon material indices: assign them afterwards.
    mesh.materials.clear()
    for n in keep:
        mesh.materials.append(old[n])
    mesh.materials.append(building)
    mesh.polygons.foreach_set("material_index", targets)
    mesh.update()


def _tex_path(tex: str, suffix: str) -> Path:
    """Texture file: ``battle/<id>`` lives in the battle folder, else in the building folder."""
    if tex.startswith("battle/"):
        return TEXTURES.parent / f"{tex}_{suffix}.jpg"
    name = "medieval_wall_01" if tex == "lime_plaster" and suffix != "diff" else tex
    return TEXTURES / f"{name}_{suffix}.jpg"


def reset_scene() -> None:
    """Empty the scene."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def _mix_rgb(nodes):
    """Colour multiply node (``ShaderNodeMix`` in RGBA mode, factor 1)."""
    node = nodes.new("ShaderNodeMix")
    node.data_type = "RGBA"
    node.blend_type = "MULTIPLY"
    node.inputs["Factor"].default_value = 1.0
    return node


def _sock(sockets, name):
    """The colour socket called ``name`` (the Mix node has float/vector/colour twins)."""
    return next(s for s in sockets if s.name == name and s.type == "RGBA")


def material(name: str, textured: bool) -> bpy.types.Material:
    """Principled material; ``textured`` hooks the photo textures (previews only)."""
    key = f"{name}{'_tex' if textured else ''}"
    existing = bpy.data.materials.get(key)
    if existing is not None:
        return existing
    tex, tile, color, rough = MATERIALS.get(name, (None, 1.0, (0.5, 0.5, 0.5), 0.9))
    mat = bpy.data.materials.new(key)
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Roughness"].default_value = rough
    vcol = nodes.new("ShaderNodeVertexColor")
    vcol.layer_name = "Color"
    mix = _mix_rgb(nodes)
    links.new(vcol.outputs["Color"], _sock(mix.inputs, "B"))
    links.new(_sock(mix.outputs, "Result"), bsdf.inputs["Base Color"])
    if textured and tex:
        uv = nodes.new("ShaderNodeUVMap")
        mapping = nodes.new("ShaderNodeMapping")
        mapping.inputs["Scale"].default_value = (1.0 / tile, 1.0 / tile, 1.0)
        links.new(uv.outputs["UV"], mapping.inputs["Vector"])
        img = nodes.new("ShaderNodeTexImage")
        img.image = bpy.data.images.load(
            str(_tex_path(tex, "diff")), check_existing=True
        )
        links.new(mapping.outputs["Vector"], img.inputs["Vector"])
        tint = _mix_rgb(nodes)
        _sock(tint.inputs, "B").default_value = (
            *MATERIAL_TINT.get(name, (1.0, 1.0, 1.0)),
            1.0,
        )
        links.new(img.outputs["Color"], _sock(tint.inputs, "A"))
        links.new(_sock(tint.outputs, "Result"), _sock(mix.inputs, "A"))
        nor = nodes.new("ShaderNodeTexImage")
        nor.image = bpy.data.images.load(
            str(_tex_path(tex, "nor")), check_existing=True
        )
        nor.image.colorspace_settings.name = "Non-Color"
        links.new(mapping.outputs["Vector"], nor.inputs["Vector"])
        nmap = nodes.new("ShaderNodeNormalMap")
        links.new(nor.outputs["Color"], nmap.inputs["Color"])
        links.new(nmap.outputs["Normal"], bsdf.inputs["Normal"])
    else:
        _sock(mix.inputs, "A").default_value = (*color, 1.0)
    mat.diffuse_color = (*color, 1.0)
    return mat


def to_object(g: kit.Geometry, name: str, textured: bool = False) -> bpy.types.Object:
    """Blender mesh object from a kit geometry (one material slot per kit material)."""
    verts, faces, uvs, cols, mat_index = [], [], [], [], []
    names = sorted(g.polys)
    for mi, mat_name in enumerate(names):
        for points, puv, pcol in g.polys[mat_name]:
            start = len(verts)
            verts.extend(points)
            faces.append(tuple(range(start, start + len(points))))
            uvs.extend(puv)
            cols.extend(pcol)
            mat_index.append(mi)
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    uv_layer = mesh.uv_layers.new(name="UVMap")
    colors = mesh.color_attributes.new(
        name="Color", type="FLOAT_COLOR", domain="CORNER"
    )
    for poly in mesh.polygons:
        poly.material_index = mat_index[poly.index]
        for li in poly.loop_indices:
            vi = mesh.loops[li].vertex_index
            uv_layer.data[li].uv = uvs[vi]
            c = cols[vi]
            colors.data[li].color = (c[0], c[1], c[2], 1.0)
    mesh.color_attributes.active_color = colors
    for mat_name in names:
        mesh.materials.append(material(mat_name, textured))
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    # Merge coincident corners inside each face group, keep hard edges (flat shading).
    for poly in mesh.polygons:
        poly.use_smooth = False
    return obj


def export_glb(obj: bpy.types.Object, path: Path) -> None:
    """Export one object to a binary glTF (Y up, vertex colours, no textures)."""
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.export_scene.gltf(
        filepath=str(path),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_yup=True,
        export_materials="EXPORT",
        export_vertex_color="ACTIVE",
        export_image_format="NONE",
    )


# Battle set: kind -> list of (seed, dims or {}, ruined)
BATTLE_SET = {
    "cottage": [(s, {}, False) for s in (11, 12, 13, 14, 15, 16)]
    + [(17, {}, True), (18, {}, True)],
    "longere": [(21, {}, False), (22, {}, False), (23, {}, False), (24, {}, True)],
    "timber": [(s, {}, False) for s in (31, 32, 33, 34, 35, 36)]
    + [(37, {}, True), (38, {}, True)],
    "townhouse": [(s, {}, False) for s in (41, 42, 43, 44, 45, 46)]
    + [(47, {}, True), (48, {}, True)],
    "stonehouse": [(51, {}, False), (52, {}, False), (53, {}, False), (54, {}, True)],
    "barn": [(61, {}, False), (62, {}, False), (63, {}, False), (64, {}, True)],
    "church": [
        (71, {"length": 19.0, "depth": 8.0}, False),
        (72, {"length": 21.0, "depth": 8.5}, False),
        (73, {"length": 36.0, "depth": 13.0}, False),
    ],
    "manor": [(81, {}, False)],
    "hall": [(91, {}, False)],
    "well": [(95, {}, False)],
    "windmill": [(97, {}, False)],
    "stall": [(s, {}, False) for s in (101, 102, 103, 104, 105)],
    "cart": [(s, {}, False) for s in (111, 112, 113, 114)],
    "barrels": [(121, {}, False), (122, {}, False), (123, {}, False)],
    "woodpile": [(131, {}, False), (132, {}, False)],
    # EP3: bridge pieces (assembled by game/scripts/battle/battle_bridges.gd).
    "bridge_stone_bay": [(141, {}, False)],
    "bridge_stone_end": [(142, {}, False)],
    "bridge_wood_bay": [(143, {}, False)],
    "bridge_wood_end": [(144, {}, False)],
    # EP6: villages and battlefield decor (seeds from 150).
    "watermill": [(150, {}, False), (151, {}, False)],
    "tent": [(152, {}, False), (153, {}, False), (154, {}, False)],
    "pavilion": [(155, {}, False), (156, {}, False), (157, {}, False)],
    "haystack": [(158, {}, False), (159, {}, False), (160, {}, False)],
    "wagon": [(161, {}, False), (162, {}, False), (163, {}, False)],
    "campfire": [(164, {}, False), (165, {}, False)],
    "wall_run": [(166, {}, False), (167, {}, False)],
    "lychgate": [(168, {}, False)],
    "graves": [(169, {}, False), (170, {}, False), (171, {}, False)],
    "vine_row": [(172, {}, False), (173, {}, False)],
}


def export_battle(out_dir: Path, kinds: list[str] | None = None) -> None:
    """Write the battle building set and its manifest.

    With ``kinds``, only those kinds are written and merged into the existing manifest (EP3:
    adding the bridge pieces without rewriting every building).
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest = {}
    manifest_path = out_dir / "manifest.json"
    if kinds and manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
    for kind, entries in BATTLE_SET.items():
        if kinds and kind not in kinds:
            continue
        index = {"intact": 0, "ruin": 0}
        for seed, dims, ruined in entries:
            reset_scene()
            g, info = kit.build(kind, seed, "high", ruined, **dims)
            tag = "ruin" if ruined else "intact"
            name = f"{kind}_{'ruin_' if ruined else ''}{index[tag]}"
            index[tag] += 1
            obj = to_object(g, name)
            atlas(obj)
            export_glb(obj, out_dir / f"{name}.glb")
            manifest[name] = {
                "kind": kind,
                "ruined": ruined,
                "length": round(info["length"], 2),
                "depth": round(info["depth"], 2),
                "height": round(info["height"], 2),
                "triangles": info["triangles"],
            }
            print("MODEL", name, info["triangles"])
    (out_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=1, sort_keys=True) + "\n"
    )
    print("OK")


def _setup_stage(scene) -> None:
    """Ground, sun, sky and AgX for previews."""
    bpy.ops.mesh.primitive_plane_add(size=600, location=(0, 0, -0.01))
    ground = bpy.context.active_object
    gm = bpy.data.materials.new("Ground")
    gm.use_nodes = True
    gb = next(n for n in gm.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    gb.inputs["Base Color"].default_value = (0.09, 0.11, 0.05, 1)
    gb.inputs["Roughness"].default_value = 1.0
    ground.data.materials.append(gm)
    sun_data = bpy.data.lights.new("Sun", "SUN")
    sun_data.energy = 4.2
    sun_data.angle = math.radians(2.0)
    sun = bpy.data.objects.new("Sun", sun_data)
    sun.rotation_euler = (math.radians(50), math.radians(8), math.radians(35))
    scene.collection.objects.link(sun)
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    bg = next(n for n in world.node_tree.nodes if n.type == "BACKGROUND")
    bg.inputs["Color"].default_value = (0.45, 0.55, 0.75, 1)
    bg.inputs["Strength"].default_value = 0.9
    scene.world = world
    scene.render.engine = "BLENDER_EEVEE"
    with contextlib.suppress(TypeError):
        scene.view_settings.view_transform = "AgX"


def preview(
    prefix: Path, kinds: list[str], detail: str = "high", size=(900, 640)
) -> None:
    """Render each building alone (auto-framed 3/4 view) to ``<prefix>_<i>.png``."""
    import mathutils

    for i, spec in enumerate(kinds or list(kit.RECIPES)):
        reset_scene()
        scene = bpy.context.scene
        _setup_stage(scene)
        ruined = spec.endswith("!")
        kind, _, seed = spec.rstrip("!").partition(":")
        g, info = kit.build(kind, int(seed) if seed else 100 + i * 7, detail, ruined)
        to_object(g, f"{kind}_{i}", textured=True)
        cam_data = bpy.data.cameras.new("Cam")
        cam_data.lens = 40
        cam = bpy.data.objects.new("Cam", cam_data)
        scene.collection.objects.link(cam)
        scene.camera = cam
        top = info["height"]
        span = max(info["length"], info["depth"], top) * 1.0 + 6.0
        elev, yaw = math.radians(22), math.radians(-35)
        target = mathutils.Vector((0.0, 0.0, top * 0.38))
        dist = span * 1.45
        cam.location = (
            target
            + mathutils.Vector(
                (
                    math.sin(yaw) * math.cos(elev),
                    -math.cos(yaw) * math.cos(elev),
                    math.sin(elev),
                )
            )
            * dist
        )
        cam.rotation_euler = (target - cam.location).to_track_quat("-Z", "Y").to_euler()
        scene.render.resolution_x, scene.render.resolution_y = size
        scene.render.filepath = f"{prefix}_{i:02d}.png"
        bpy.ops.render.render(write_still=True)
        print("PREVIEW", scene.render.filepath, info["triangles"])


def preview_glb(png_prefix: Path, glbs: list[str]) -> None:
    """Render existing GLB models with the textured preview materials (campaign settlements)."""
    import mathutils

    for i, path in enumerate(glbs):
        reset_scene()
        scene = bpy.context.scene
        _setup_stage(scene)
        bpy.ops.import_scene.gltf(filepath=path)
        objs = [o for o in scene.objects if o.type == "MESH" and o.name != "Plane"]
        lo = mathutils.Vector((1e9, 1e9, 1e9))
        hi = mathutils.Vector((-1e9, -1e9, -1e9))
        for obj in objs:
            for slot in obj.material_slots:
                if slot.material is not None:
                    base = slot.material.name.split(".")[0]
                    if base in MATERIALS:
                        slot.material = material(base, True)
            for corner in obj.bound_box:
                w = obj.matrix_world @ mathutils.Vector(corner)
                lo = mathutils.Vector(map(min, lo, w))
                hi = mathutils.Vector(map(max, hi, w))
        ground = next(o for o in scene.objects if o.name == "Plane")
        ground.location.z = -0.002
        ground.scale = (0.02, 0.02, 1)
        cam_data = bpy.data.cameras.new("Cam")
        cam_data.lens = 40
        cam_data.clip_start = 0.01
        cam = bpy.data.objects.new("Cam", cam_data)
        scene.collection.objects.link(cam)
        scene.camera = cam
        target = (lo + hi) / 2
        target.z = 0.0
        span = max(hi.x - lo.x, hi.y - lo.y)
        elev, yaw = math.radians(38), math.radians(-30)
        cam.location = (
            target
            + mathutils.Vector(
                (
                    math.sin(yaw) * math.cos(elev),
                    -math.cos(yaw) * math.cos(elev),
                    math.sin(elev),
                )
            )
            * span
            * 1.15
        )
        cam.rotation_euler = (target - cam.location).to_track_quat("-Z", "Y").to_euler()
        for light in [o for o in scene.objects if o.type == "LIGHT"]:
            light.data.energy = 4.2
        scene.render.resolution_x, scene.render.resolution_y = 1200, 800
        scene.render.filepath = f"{png_prefix}_{i:02d}.png"
        bpy.ops.render.render(write_still=True)
        print("PREVIEW", scene.render.filepath)


# --- Horse (lot EP6): static grazing horse from the Quaternius rig -----------------------------
# Not part of ``building_kit.RECIPES`` (it comes from a skinned glTF, not pure-Python geometry):
# a dedicated export path that still writes ``horse_<n>.glb`` into the same directory and merges
# entries of the same shape (kind, ruined, length/depth/height, triangles) into the manifest.

HORSE_COAT_VARIANTS = [
    (
        "bay",
        {
            "Main": (0.24, 0.13, 0.08),
            "Main_Dark": (0.05, 0.04, 0.03),
            "Main_Light": (0.22, 0.12, 0.07),
            "Muzzle": (0.05, 0.04, 0.03),
            "Hair": (0.04, 0.03, 0.02),
        },
    ),
    (
        "chestnut",
        {
            "Main": (0.42, 0.22, 0.12),
            "Main_Dark": (0.30, 0.16, 0.09),
            "Main_Light": (0.40, 0.22, 0.12),
            "Muzzle": (0.20, 0.11, 0.07),
            "Hair": (0.32, 0.17, 0.09),
        },
    ),
    (
        "grey",
        {
            "Main": (0.62, 0.62, 0.62),
            "Main_Dark": (0.40, 0.40, 0.40),
            "Main_Light": (0.82, 0.82, 0.82),
            "Muzzle": (0.30, 0.30, 0.30),
            "Hair": (0.55, 0.55, 0.55),
        },
    ),
]


def _bbox(obj):
    """World-space (lo, hi) corners of a mesh object's vertices."""
    import mathutils

    lo = mathutils.Vector((1e9, 1e9, 1e9))
    hi = mathutils.Vector((-1e9, -1e9, -1e9))
    for v in obj.data.vertices:
        w = obj.matrix_world @ v.co
        lo = mathutils.Vector(map(min, lo, w))
        hi = mathutils.Vector(map(max, hi, w))
    return lo, hi


def _horse_recolor(coat: dict) -> None:
    """Set the flat base colour of the horse's named materials (static export, no textures)."""
    for name, rgb in coat.items():
        mat = bpy.data.materials.get(name)
        if mat is None or not mat.use_nodes:
            continue
        bsdf = next(
            (n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None
        )
        if bsdf is not None:
            bsdf.inputs["Base Color"].default_value = (*rgb, 1.0)
        mat.diffuse_color = (*rgb, 1.0)


def export_horses(out_dir: Path, seed_start: int = 150) -> None:
    """Static grazing horse (head-low ``Eating`` pose baked from the Quaternius rig), three coats, bare (no tack); writes ``horse_0..2.glb`` and merges them into the existing manifest."""
    out_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = out_dir / "manifest.json"
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    for i, (_coat_name, coat) in enumerate(HORSE_COAT_VARIANTS):
        reset_scene()
        arm, meshes, roots = bs.import_glb(str(QUATERNIUS / "horse.glb"))
        horse = next(m for m in meshes if m.name == "Horse")
        for r in roots:
            r.scale = (bs.HORSE_SCALE,) * 3
        bpy.context.view_layer.update()
        act = bs.find_action("Eating", "AnimalArmature")
        bs.set_action(arm, act)
        bpy.context.scene.frame_set(40)
        bpy.context.view_layer.update()
        bpy.ops.object.select_all(action="DESELECT")
        horse.select_set(True)
        bpy.context.view_layer.objects.active = horse
        bpy.ops.object.modifier_apply(modifier="Armature")
        # Flatten onto the mesh: unparent (keep the world transform), then bake it in, turn the
        # horse to face -X (length along X, per the contract) and drop it to the ground.
        bpy.ops.object.parent_clear(type="CLEAR_KEEP_TRANSFORM")
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
        horse.rotation_mode = "XYZ"
        horse.rotation_euler[2] = -math.pi / 2
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
        lo, _hi = _bbox(horse)
        horse.location.z -= lo.z
        bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)
        _horse_recolor(coat)
        for obj in list(bpy.context.scene.objects):
            if obj is not horse:
                bpy.data.objects.remove(obj, do_unlink=True)
        name = f"horse_{i}"
        horse.name = name
        export_glb(horse, out_dir / f"{name}.glb")
        lo, hi = _bbox(horse)
        tris = sum(len(p.vertices) - 2 for p in horse.data.polygons)
        manifest[name] = {
            "kind": "horse",
            "ruined": False,
            "length": round(hi.x - lo.x, 2),
            "depth": round(hi.y - lo.y, 2),
            "height": round(hi.z - lo.z, 2),
            "triangles": tris,
        }
        print("MODEL", name, tris)
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")
    print("OK")


def main() -> None:
    """Command line entry point."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if not argv:
        raise SystemExit("usage: -- export <dir> | preview <png> [kind ...]")
    command = argv[0]
    if command == "export":
        kinds = argv[2:] or None
        out_dir = Path(argv[1])
        building_kinds = [k for k in (kinds or []) if k != "horse"] or None
        if kinds is None or "horse" in kinds:
            export_horses(out_dir)
        if kinds is None or building_kinds:
            export_battle(out_dir, building_kinds)
    elif command == "preview":
        args = argv[2:]
        detail = "low" if "--low" in args else "high"
        preview(Path(argv[1]), [a for a in args if not a.startswith("--")], detail)
    elif command == "preview-glb":
        preview_glb(Path(argv[1]), argv[2:])
    else:
        raise SystemExit(f"unknown command {command}")


if __name__ == "__main__":
    main()
