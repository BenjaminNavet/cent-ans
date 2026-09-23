"""Procedural low-poly campaign-map models, exported as glTF binaries (.glb).

Run headless:  blender --background --python models.py -- <out_dir> [name ...]

Models (Y up in Godot, built Z up in Blender; the glTF exporter converts):
castle, town, village, cathedral, army (mounted standard-bearer), siege_camp, ship (cog).
Each model is one joined mesh with flat-coloured materials; the material named
``Banner`` is re-tinted by Godot with the faction colour. After export, a JSON line
``MODEL <name> <triangles>`` is printed per model and ``OK`` at the end.
"""

import math
import sys
from pathlib import Path

import bpy

PALETTE = {
    "Stone": (0.62, 0.58, 0.50),
    "DarkStone": (0.42, 0.39, 0.34),
    "Slate": (0.20, 0.25, 0.36),
    "Roof": (0.55, 0.20, 0.12),
    "Thatch": (0.70, 0.58, 0.30),
    "Plaster": (0.86, 0.80, 0.66),
    "Wood": (0.36, 0.22, 0.11),
    "Canvas": (0.88, 0.84, 0.72),
    "Horse": (0.40, 0.26, 0.15),
    "Steel": (0.62, 0.64, 0.68),
    "Skin": (0.85, 0.66, 0.52),
    "Gold": (0.85, 0.66, 0.18),
    "Fire": (0.95, 0.45, 0.10),
    "Grass": (0.35, 0.45, 0.20),
    "Banner": (0.80, 0.10, 0.10),
}
MAX_TRIANGLES = 3000


def reset_scene() -> None:
    """Empty the scene and purge orphan data."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def material(name: str) -> bpy.types.Material:
    """Return (creating once) a flat Principled material of the palette."""
    existing = bpy.data.materials.get(name)
    if existing is not None:
        return existing
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = next(node for node in mat.node_tree.nodes if node.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*PALETTE[name], 1.0)
    bsdf.inputs["Roughness"].default_value = 0.35 if name in ("Steel", "Gold") else 0.85
    if name in ("Steel", "Gold"):
        bsdf.inputs["Metallic"].default_value = 0.8
    mat.diffuse_color = (*PALETTE[name], 1.0)
    return mat


def _finish(obj: bpy.types.Object, mat_name: str) -> bpy.types.Object:
    obj.data.materials.append(material(mat_name))
    return obj


def box(size, location, mat, rotation=(0, 0, 0)):
    """Axis-aligned box of full ``size`` (x, y, z) centred at ``location``."""
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location, rotation=rotation)
    obj = bpy.context.active_object
    obj.scale = size
    return _finish(obj, mat)


def cylinder(radius, depth, location, mat, vertices=8, rotation=(0, 0, 0)):
    """Low-poly cylinder standing on Z."""
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radius,
        depth=depth,
        location=location,
        rotation=rotation,
    )
    return _finish(bpy.context.active_object, mat)


def cone(radius, depth, location, mat, vertices=8, radius_top=0.0, rotation=(0, 0, 0)):
    """Low-poly cone (or frustum with ``radius_top``)."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radius,
        radius2=radius_top,
        depth=depth,
        location=location,
        rotation=rotation,
    )
    return _finish(bpy.context.active_object, mat)


def sphere(radius, location, mat, subdivisions=1):
    """Icosphere (subdivisions 1 = 20 faces)."""
    bpy.ops.mesh.primitive_ico_sphere_add(
        subdivisions=subdivisions, radius=radius, location=location
    )
    return _finish(bpy.context.active_object, mat)


def gable_roof(length, width, height, location, mat, angle=0.0):
    """Triangular prism roof: ridge along local X at ``height`` above ``location``."""
    half_l, half_w = length / 2, width / 2
    verts = []
    for x in (-half_l, half_l):
        verts += [(x, -half_w, 0.0), (x, half_w, 0.0), (x, 0.0, height)]
    faces = [(0, 1, 2), (3, 5, 4), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    mesh = bpy.data.meshes.new("roof")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new("roof", mesh)
    bpy.context.collection.objects.link(obj)
    obj.location = location
    obj.rotation_euler = (0, 0, angle)
    return _finish(obj, mat)


def house(location, size, roof_mat="Roof", wall_mat="Plaster", angle=0.0):
    """Box house with a gable roof."""
    x, y, z = location
    w, d, h = size
    body = box((w, d, h), (x, y, z + h / 2), wall_mat, rotation=(0, 0, angle))
    roof = gable_roof(w * 1.1, d * 1.15, h * 0.7, (x, y, z + h), roof_mat, angle=angle)
    return [body, roof]


def crenellations(center, half_x, half_y, z, mat, spacing=0.3, size=0.12):
    """Merlons along the rim of a rectangle."""
    parts = []
    cx, cy = center
    steps_x = max(2, int(2 * half_x / spacing))
    steps_y = max(2, int(2 * half_y / spacing))
    for index in range(steps_x + 1):
        x = cx - half_x + index * 2 * half_x / steps_x
        for y in (cy - half_y, cy + half_y):
            if index % 2 == 0:
                parts.append(box((size, size, size), (x, y, z + size / 2), mat))
    for index in range(1, steps_y):
        y = cy - half_y + index * 2 * half_y / steps_y
        for x in (cx - half_x, cx + half_x):
            if index % 2 == 0:
                parts.append(box((size, size, size), (x, y, z + size / 2), mat))
    return parts


def tower(location, radius, height, roof_mat="Slate", vertices=8):
    """Round tower with a conical roof."""
    x, y, z = location
    return [
        cylinder(radius, height, (x, y, z + height / 2), "Stone", vertices),
        cone(
            radius * 1.25,
            height * 0.55,
            (x, y, z + height + height * 0.27),
            roof_mat,
            vertices,
        ),
    ]


def pennant(location, height, mat="Banner"):
    """Pole with a small flag."""
    x, y, z = location
    return [
        cylinder(0.02, height, (x, y, z + height / 2), "Wood", 4),
        box((0.02, 0.35, 0.2), (x, y + 0.18, z + height - 0.12), mat),
    ]


# --- Models ------------------------------------------------------------------------


def build_castle():
    """Capital castle: keep, curtain wall, four corner towers, gatehouse, banner."""
    parts = [box((2.6, 2.6, 0.1), (0, 0, 0.05), "Grass")]
    half = 1.0
    for x, y, sx, sy in (
        (0, -half, 2 * half, 0.18),
        (0, half, 2 * half, 0.18),
        (-half, 0, 0.18, 2 * half),
        (half, 0, 0.18, 2 * half),
    ):
        parts.append(box((sx, sy, 0.8), (x, y, 0.4), "Stone"))
    parts += crenellations((0, 0), half, half, 0.8, "Stone", spacing=0.25, size=0.1)
    for x in (-half, half):
        for y in (-half, half):
            parts += tower((x, y, 0), 0.25, 1.2)
    parts.append(box((0.5, 0.35, 1.0), (0, -half, 0.5), "DarkStone"))
    parts.append(box((0.22, 0.4, 0.4), (0, -half, 0.2), "Wood"))
    parts.append(box((0.9, 0.9, 1.9), (0.1, 0.2, 0.95), "Stone"))
    parts += crenellations((0.1, 0.2), 0.45, 0.45, 1.9, "Stone", spacing=0.22, size=0.1)
    parts += tower((0.55, 0.65, 0), 0.18, 2.3)
    parts += pennant((0.1, 0.2, 2.0), 0.9)
    return parts


def build_town():
    """Walled town: octagonal wall with towers, houses, a church with spire."""
    parts = [cylinder(1.35, 0.08, (0, 0, 0.04), "Grass", 12)]
    radius = 1.15
    sides = 8
    for index in range(sides):
        angle = index * 2 * math.pi / sides
        next_angle = (index + 1) * 2 * math.pi / sides
        x0, y0 = radius * math.cos(angle), radius * math.sin(angle)
        x1, y1 = radius * math.cos(next_angle), radius * math.sin(next_angle)
        length = math.hypot(x1 - x0, y1 - y0)
        parts.append(
            box(
                (length, 0.12, 0.5),
                ((x0 + x1) / 2, (y0 + y1) / 2, 0.25),
                "Stone",
                rotation=(0, 0, (angle + next_angle) / 2 + math.pi / 2),
            )
        )
        parts += tower((x0, y0, 0), 0.14, 0.75, "Roof", 6)
    for (x, y), angle in zip(
        (
            (-0.45, -0.3),
            (0.35, -0.5),
            (0.5, 0.25),
            (-0.2, 0.5),
            (-0.6, 0.2),
            (0.1, -0.05),
            (0.55, -0.1),
        ),
        (0.2, 0.8, 1.4, 0.5, 1.9, 0.0, 2.5),
        strict=True,
    ):
        parts += house((x, y, 0.08), (0.32, 0.24, 0.3), angle=angle)
    parts.append(box((0.5, 0.25, 0.45), (-0.1, 0.1, 0.3), "Stone"))
    parts.append(gable_roof(0.55, 0.3, 0.2, (-0.1, 0.1, 0.52), "Slate"))
    parts.append(box((0.16, 0.16, 0.8), (-0.4, 0.1, 0.48), "Stone"))
    parts.append(cone(0.14, 0.5, (-0.4, 0.1, 1.13), "Slate", 4))
    return parts


def build_village():
    """Open village: thatched cottages around a small chapel."""
    parts = [cylinder(1.0, 0.06, (0, 0, 0.03), "Grass", 10)]
    for (x, y), angle in zip(
        ((-0.5, -0.3), (0.4, -0.45), (0.55, 0.3), (-0.35, 0.45), (0.0, -0.05)),
        (0.3, 1.0, 1.7, 2.3, 0.0),
        strict=True,
    ):
        parts += house((x, y, 0.06), (0.36, 0.26, 0.26), roof_mat="Thatch", angle=angle)
    parts.append(box((0.3, 0.18, 0.3), (0.05, 0.6, 0.21), "Stone"))
    parts.append(gable_roof(0.33, 0.2, 0.14, (0.05, 0.6, 0.36), "Roof"))
    parts.append(box((0.1, 0.1, 0.5), (-0.12, 0.6, 0.31), "Stone"))
    parts.append(cone(0.09, 0.25, (-0.12, 0.6, 0.68), "Roof", 4))
    parts.append(box((0.8, 0.5, 0.02), (0.3, -1.0, 0.07), "Thatch"))
    return parts


def build_cathedral():
    """Gothic cathedral: nave, transept, apse, twin west towers, crossing spire."""
    parts = [box((2.4, 1.3, 0.06), (0, 0, 0.03), "Grass")]
    parts.append(box((1.8, 0.5, 0.9), (0, 0, 0.51), "Stone"))
    parts.append(gable_roof(1.8, 0.56, 0.35, (0, 0, 0.96), "Slate"))
    parts.append(box((0.45, 1.2, 0.8), (0.2, 0, 0.46), "Stone"))
    parts.append(gable_roof(1.2, 0.5, 0.3, (0.2, 0, 0.86), "Slate", angle=math.pi / 2))
    parts.append(cylinder(0.25, 0.8, (0.9, 0, 0.46), "Stone", 8))
    parts.append(cone(0.28, 0.35, (0.9, 0, 1.03), "Slate", 8))
    for y in (-0.2, 0.2):
        parts.append(box((0.3, 0.3, 1.5), (-0.95, y, 0.8), "Stone"))
        parts.append(
            cone(0.2, 0.55, (-0.95, y, 1.83), "Slate", 4, rotation=(0, 0, math.pi / 4))
        )
    parts.append(box((0.16, 0.16, 0.5), (0.2, 0, 1.3), "Stone"))
    parts.append(cone(0.12, 0.8, (0.2, 0, 1.95), "Slate", 8))
    for x in (-0.6, -0.2, 0.4):
        for y in (-0.33, 0.33):
            parts.append(box((0.08, 0.14, 0.7), (x, y, 0.41), "DarkStone"))
    parts.append(sphere(0.03, (0.2, 0, 2.37), "Gold", 1))
    return parts


def build_army():
    """Mounted standard-bearer: horse, rider, lance with a large banner (Banner material)."""
    parts = []
    parts.append(box((1.1, 0.38, 0.42), (0, 0, 0.95), "Horse"))
    for x in (-0.42, 0.42):
        for y in (-0.13, 0.13):
            parts.append(box((0.1, 0.1, 0.75), (x, y, 0.38), "Horse"))
    parts.append(
        box(
            (0.28, 0.2, 0.5),
            (0.6, 0, 1.28),
            "Horse",
            rotation=(0, math.radians(-30), 0),
        )
    )
    parts.append(
        box(
            (0.38, 0.16, 0.18),
            (0.82, 0, 1.5),
            "Horse",
            rotation=(0, math.radians(20), 0),
        )
    )
    parts.append(
        box(
            (0.12, 0.06, 0.4),
            (-0.6, 0, 0.95),
            "DarkStone",
            rotation=(0, math.radians(25), 0),
        )
    )
    parts.append(box((1.0, 0.44, 0.22), (0, 0, 0.72), "Banner"))  # caparison
    parts.append(box((0.26, 0.3, 0.5), (-0.05, 0, 1.4), "Steel"))
    parts.append(box((0.12, 0.1, 0.4), (0.05, -0.2, 1.35), "Steel"))
    parts.append(sphere(0.12, (-0.05, 0, 1.75), "Steel", 1))
    parts.append(cone(0.08, 0.12, (-0.05, 0, 1.9), "Gold", 4))
    parts.append(box((0.08, 0.22, 0.3), (0.12, 0.26, 1.35), "Banner"))  # shield
    parts.append(cylinder(0.025, 2.6, (-0.1, -0.24, 1.9), "Wood", 6))
    parts.append(box((0.03, 0.9, 0.62), (-0.1, -0.7, 2.8), "Banner"))
    parts.append(cone(0.05, 0.14, (-0.1, -0.24, 3.27), "Gold", 4))
    return parts


def build_siege_camp():
    """Siege camp: tents, palisade stakes, a trebuchet and a campfire."""
    parts = [cylinder(1.4, 0.05, (0, 0, 0.025), "Grass", 10)]
    for (x, y), mat in zip(
        ((-0.6, -0.4), (-0.1, -0.7), (0.5, -0.5), (-0.7, 0.3)),
        ("Canvas", "Banner", "Canvas", "Canvas"),
        strict=True,
    ):
        parts.append(cone(0.32, 0.55, (x, y, 0.32), mat, 6))
        parts.append(cylinder(0.015, 0.2, (x, y, 0.65), "Wood", 4))
    for index in range(9):
        angle = math.radians(200 + index * 15)
        parts.append(
            cone(
                0.05,
                0.4,
                (1.2 * math.cos(angle) + 0.4, 1.2 * math.sin(angle) + 0.6, 0.2),
                "Wood",
                4,
            )
        )
    # Trebuchet
    for y in (-0.12, 0.12):
        parts.append(
            box(
                (0.08, 0.06, 0.9),
                (0.5, 0.5 + y, 0.47),
                "Wood",
                rotation=(0, math.radians(12), 0),
            )
        )
        parts.append(
            box(
                (0.08, 0.06, 0.9),
                (0.72, 0.5 + y, 0.47),
                "Wood",
                rotation=(0, math.radians(-12), 0),
            )
        )
    parts.append(box((0.7, 0.36, 0.06), (0.61, 0.5, 0.06), "Wood"))
    parts.append(
        box(
            (1.4, 0.05, 0.05),
            (0.61, 0.5, 0.95),
            "Wood",
            rotation=(0, math.radians(-35), 0),
        )
    )
    parts.append(box((0.2, 0.2, 0.2), (1.1, 0.5, 1.2), "DarkStone"))
    parts.append(cone(0.1, 0.18, (-0.1, 0.1, 0.12), "Fire", 5))
    for angle in (0.0, 2.1, 4.2):
        parts.append(
            box((0.24, 0.04, 0.04), (-0.1, 0.1, 0.06), "Wood", rotation=(0, 0, angle))
        )
    return parts


def build_ship():
    """Cog: clinker hull (tapered box), fore and stern castles, mast, sail, pennant."""
    parts = []
    hull = box((1.8, 0.6, 0.45), (0, 0, 0.23), "Wood")
    for vertex in hull.data.vertices:
        if vertex.co.z < 0:  # bottom narrower
            vertex.co.y *= 0.45
            vertex.co.x *= 0.8
    parts.append(hull)
    parts.append(box((0.35, 0.62, 0.3), (0.78, 0, 0.6), "Wood"))
    parts.append(box((0.45, 0.62, 0.35), (-0.72, 0, 0.62), "Wood"))
    parts.append(box((1.1, 0.5, 0.04), (0.02, 0, 0.44), "Horse"))
    parts.append(cylinder(0.035, 2.0, (0, 0, 1.4), "Wood", 6))
    parts.append(box((0.03, 0.9, 0.9), (0.05, 0, 1.5), "Canvas"))
    parts.append(box((0.035, 0.5, 0.4), (0.055, 0, 1.5), "Banner"))
    parts.append(box((0.02, 0.4, 0.18), (0, 0.2, 2.35), "Banner"))
    parts.append(box((0.04, 0.9, 0.04), (0, 0, 1.97), "Wood"))
    return parts


MODELS = {
    "castle": build_castle,
    "town": build_town,
    "village": build_village,
    "cathedral": build_cathedral,
    "army": build_army,
    "siege_camp": build_siege_camp,
    "ship": build_ship,
}


def triangle_count(obj: bpy.types.Object) -> int:
    """Triangles of the evaluated mesh (n-gons fan-triangulated)."""
    return sum(len(polygon.vertices) - 2 for polygon in obj.data.polygons)


def export_model(name: str, out_dir: Path) -> int:
    """Build one model, join it into a single mesh, export ``<name>.glb``; return triangles."""
    reset_scene()
    parts = MODELS[name]()
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    bpy.ops.object.join()
    obj = bpy.context.active_object
    obj.name = name
    bpy.ops.object.shade_flat()
    triangles = triangle_count(obj)
    if triangles >= MAX_TRIANGLES:
        raise RuntimeError(f"{name}: {triangles} triangles >= {MAX_TRIANGLES}")
    out_dir.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(out_dir / f"{name}.glb"),
        export_format="GLB",
        export_yup=True,
        export_apply=True,
        use_selection=False,
    )
    return triangles


def main() -> None:
    """Parse ``-- <out_dir> [names...]`` and export the requested models."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(args[0]) if args else Path.cwd() / "models"
    names = args[1:] or list(MODELS)
    for name in names:
        print(f"MODEL {name} {export_model(name, out_dir)}")
    print("OK")


main()
