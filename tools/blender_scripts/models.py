"""Procedural campaign-map models (semi-realistic low-poly), exported as glTF binaries (.glb).

Run headless:  blender --background --python models.py -- <out_dir> [name ...]

Models (Y up in Godot, built Z up in Blender; the glTF exporter converts):

* settlements: ``castle`` (faction capital: walled city and castle), ``city_cathedral``
  (walled city around a cathedral), ``town`` (walled town), ``village`` (open village);
* ``cathedral``: the cathedral building alone (also used by the siege battlefield);
* army figurines: ``army`` (mounted commander), ``army_foot`` (man-at-arms with spear and
  shield), ``army_archer`` (crossbowman), ``siege_camp``, ``ship`` (cog).

Each model is one joined mesh with PBR materials (base colour, roughness, metallic): natural
stone, slate and weathered tile roofs, thatch, plaster, timber. Surfaces whose material is
named ``Banner`` are re-tinted by Godot with the faction colour. Buildings extend below
``z = 0`` (foundations) so that they sit on sloped terrain without floating. After export, a
line ``MODEL <name> <triangles>`` is printed per model and ``OK`` at the end.
"""

import math
import random
import sys
from pathlib import Path

import bpy

# name: (base colour (linear RGB), roughness, metallic)
PALETTE = {
    "Stone": ((0.46, 0.43, 0.37), 0.88, 0.0),
    "StoneLight": ((0.60, 0.56, 0.48), 0.85, 0.0),
    "DarkStone": ((0.27, 0.25, 0.22), 0.92, 0.0),
    "Slate": ((0.13, 0.145, 0.17), 0.55, 0.0),
    "Tile": ((0.40, 0.18, 0.10), 0.78, 0.0),
    "TileOld": ((0.32, 0.20, 0.13), 0.82, 0.0),
    "Thatch": ((0.36, 0.28, 0.14), 0.97, 0.0),
    "Plaster": ((0.66, 0.60, 0.48), 0.92, 0.0),
    "Timber": ((0.50, 0.42, 0.31), 0.9, 0.0),
    "Wood": ((0.20, 0.13, 0.08), 0.82, 0.0),
    "Dirt": ((0.27, 0.23, 0.16), 1.0, 0.0),
    "Canvas": ((0.66, 0.61, 0.49), 0.9, 0.0),
    "Horse": ((0.22, 0.14, 0.08), 0.7, 0.0),
    "Leather": ((0.24, 0.15, 0.08), 0.75, 0.0),
    "Cloth": ((0.30, 0.27, 0.22), 0.9, 0.0),
    "Steel": ((0.52, 0.53, 0.55), 0.35, 0.9),
    "Skin": ((0.62, 0.44, 0.32), 0.7, 0.0),
    "Gold": ((0.78, 0.58, 0.20), 0.35, 0.9),
    "Fire": ((0.95, 0.42, 0.08), 0.6, 0.0),
    "Banner": ((0.70, 0.08, 0.08), 0.8, 0.0),
}
EMISSIVE = {"Fire": 4.0}
MAX_TRIANGLES = 3000
FOUNDATION = 0.35  # depth of building foundations below z = 0


def reset_scene() -> None:
    """Empty the scene and purge orphan data."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def material(name: str) -> bpy.types.Material:
    """Return (creating once) a Principled PBR material of the palette."""
    existing = bpy.data.materials.get(name)
    if existing is not None:
        return existing
    color, roughness, metallic = PALETTE[name]
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = next(node for node in mat.node_tree.nodes if node.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if name in EMISSIVE:
        bsdf.inputs["Emission Color"].default_value = (*color, 1.0)
        bsdf.inputs["Emission Strength"].default_value = EMISSIVE[name]
    mat.diffuse_color = (*color, 1.0)
    mat.roughness = roughness
    mat.metallic = metallic
    return mat


def _finish(obj: bpy.types.Object, mat_name: str) -> bpy.types.Object:
    obj.data.materials.append(material(mat_name))
    return obj


def box(size, location, mat, rotation=(0, 0, 0)):
    """Box of full ``size`` (x, y, z) centred at ``location``."""
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


def sphere(radius, location, mat, subdivisions=1, scale=(1, 1, 1)):
    """Icosphere (subdivisions 1 = 80 faces)."""
    bpy.ops.mesh.primitive_ico_sphere_add(
        subdivisions=subdivisions, radius=radius, location=location
    )
    obj = bpy.context.active_object
    obj.scale = scale
    return _finish(obj, mat)


def mesh_object(name, verts, faces, mat, location=(0, 0, 0), angle=0.0):
    """Object from raw vertices / faces."""
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.location = location
    obj.rotation_euler = (0, 0, angle)
    return _finish(obj, mat)


def gable_roof(length, width, height, location, mat, angle=0.0, overhang=0.04):
    """Triangular prism roof: ridge along local X at ``height`` above ``location``."""
    half_l, half_w = length / 2 + overhang, width / 2 + overhang
    verts = []
    for x in (-half_l, half_l):
        verts += [(x, -half_w, -0.01), (x, half_w, -0.01), (x, 0.0, height)]
    faces = [(0, 1, 2), (3, 5, 4), (0, 3, 4, 1), (1, 4, 5, 2), (2, 5, 3, 0)]
    return mesh_object("roof", verts, faces, mat, location, angle)


def hip_roof(length, width, height, location, mat, angle=0.0, overhang=0.03):
    """Hipped roof (short ridge) over a ``length`` x ``width`` rectangle."""
    hl, hw = length / 2 + overhang, width / 2 + overhang
    ridge = max(hl - hw, 0.0)
    verts = [
        (-hl, -hw, 0),
        (hl, -hw, 0),
        (hl, hw, 0),
        (-hl, hw, 0),
        (-ridge, 0, height),
        (ridge, 0, height),
    ]
    faces = [(0, 1, 5, 4), (2, 3, 4, 5), (1, 2, 5), (3, 0, 4), (0, 3, 2, 1)]
    return mesh_object("hip", verts, faces, mat, location, angle)


def house(location, size, roof_mat, wall_mat, angle=0.0, hip=False):
    """House with foundations and a gable (or hipped) roof."""
    x, y, z = location
    w, d, h = size
    body = box(
        (w, d, h + FOUNDATION),
        (x, y, z + (h - FOUNDATION) / 2),
        wall_mat,
        rotation=(0, 0, angle),
    )
    roof_h = d * (0.75 if roof_mat in ("Slate", "Thatch") else 0.55)
    if hip:
        roof = hip_roof(w, d, roof_h, (x, y, z + h), roof_mat, angle=angle)
    else:
        roof = gable_roof(w, d, roof_h, (x, y, z + h), roof_mat, angle=angle)
    return [body, roof]


def merlons(x0, y0, x1, y1, z, mat, size=0.05, spacing=0.11, thickness=0.06):
    """Crenellation merlons along the segment (x0, y0) - (x1, y1) at height ``z``."""
    length = math.hypot(x1 - x0, y1 - y0)
    count = max(1, int(length / spacing))
    angle = math.atan2(y1 - y0, x1 - x0)
    parts = []
    for index in range(count):
        t = (index + 0.5) / count
        parts.append(
            box(
                (size, thickness, size),
                (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z + size / 2),
                "Stone",
                rotation=(0, 0, angle),
            )
        )
    return parts


def wall_segment(x0, y0, x1, y1, height, thickness=0.07, mat="Stone", crenel=True):
    """Curtain wall between two points (with foundations and merlons)."""
    length = math.hypot(x1 - x0, y1 - y0)
    angle = math.atan2(y1 - y0, x1 - x0)
    parts = [
        box(
            (length + thickness * 0.5, thickness, height + FOUNDATION),
            ((x0 + x1) / 2, (y0 + y1) / 2, (height - FOUNDATION) / 2),
            mat,
            rotation=(0, 0, angle),
        )
    ]
    if crenel:
        parts += merlons(x0, y0, x1, y1, height, mat, spacing=0.12)
    return parts


def round_tower(x, y, radius, height, roof_mat="Slate", vertices=8, roof=True):
    """Round tower on foundations, conical roof or crenellated top."""
    parts = [
        cylinder(
            radius,
            height + FOUNDATION,
            (x, y, (height - FOUNDATION) / 2),
            "Stone",
            vertices,
        )
    ]
    if roof:
        parts.append(
            cone(
                radius * 1.2,
                radius * 2.4,
                (x, y, height + radius * 1.2),
                roof_mat,
                vertices,
            )
        )
    else:
        parts.append(
            cylinder(radius * 1.12, 0.05, (x, y, height + 0.025), "Stone", vertices)
        )
    return parts


def square_tower(x, y, side, height, roof_mat="Slate", spire=0.0, angle=0.0):
    """Square tower with a pyramidal roof (or spire of height ``spire``)."""
    parts = [
        box(
            (side, side, height + FOUNDATION),
            (x, y, (height - FOUNDATION) / 2),
            "Stone",
            rotation=(0, 0, angle),
        )
    ]
    roof_height = spire if spire > 0 else side * 0.9
    parts.append(
        cone(
            side * 0.75,
            roof_height,
            (x, y, height + roof_height / 2),
            roof_mat,
            4,
            rotation=(0, 0, angle + math.pi / 4),
        )
    )
    return parts


def ground_patch(radius, mat="Dirt", sides=16):
    """Flat irregular ground disc slightly above z = 0 (streets / yards), with a skirt below."""
    return [
        cylinder(radius, 0.02 + FOUNDATION, (0, 0, 0.01 - FOUNDATION / 2), mat, sides)
    ]


def ring_points(radius, sides, rng, jitter=0.12, phase=0.0):
    """Irregular polygon (wall line of a medieval town)."""
    points = []
    for index in range(sides):
        angle = phase + index * 2 * math.pi / sides
        r = radius * (1.0 + rng.uniform(-jitter, jitter))
        points.append((r * math.cos(angle), r * math.sin(angle)))
    return points


def town_walls(points, height, tower_radius, gates=(0,)):
    """Walls along a closed polygon, towers at the corners, gatehouses on some sides."""
    parts = []
    count = len(points)
    for index in range(count):
        x0, y0 = points[index]
        x1, y1 = points[(index + 1) % count]
        parts += wall_segment(x0, y0, x1, y1, height, crenel=False)
        parts += round_tower(x0, y0, tower_radius, height * 1.45, "Slate", 8)
        if index in gates:
            mx, my = (x0 + x1) / 2, (y0 + y1) / 2
            angle = math.atan2(y1 - y0, x1 - x0)
            parts.append(
                box(
                    (0.2, 0.16, height * 1.6 + FOUNDATION),
                    (mx, my, (height * 1.6 - FOUNDATION) / 2),
                    "Stone",
                    rotation=(0, 0, angle),
                )
            )
            parts.append(
                gable_roof(0.2, 0.16, 0.1, (mx, my, height * 1.6), "Slate", angle=angle)
            )
    return parts


def point_in_polygon(x, y, points):
    """Ray casting test."""
    inside = False
    count = len(points)
    for index in range(count):
        x0, y0 = points[index]
        x1, y1 = points[(index + 1) % count]
        if (y0 > y) != (y1 > y) and x < (x1 - x0) * (y - y0) / (y1 - y0 + 1e-9) + x0:
            inside = not inside
    return inside


def scatter_houses(
    rng,
    count,
    radius,
    keep_out,
    min_gap,
    inside=None,
    roofs=("Tile", "TileOld", "Slate"),
    walls=("Plaster", "Timber", "Stone"),
    size_scale=1.0,
):
    """Houses aligned on radial streets, avoiding ``keep_out`` circles [(x, y, r)]."""
    parts = []
    placed = []
    tries = 0
    while len(placed) < count and tries < count * 60:
        tries += 1
        r = radius * math.sqrt(rng.random())
        a = rng.random() * 2 * math.pi
        x, y = r * math.cos(a), r * math.sin(a)
        if inside is not None and not point_in_polygon(x, y, inside):
            continue
        if any(math.hypot(x - kx, y - ky) < kr for kx, ky, kr in keep_out):
            continue
        w = rng.uniform(0.15, 0.24) * size_scale
        d = rng.uniform(0.10, 0.14) * size_scale
        if any(
            math.hypot(x - px, y - py) < (w + pw) * 0.5 * min_gap
            for px, py, pw in placed
        ):
            continue
        placed.append((x, y, w))
        # Façade sur la rue : axe long perpendiculaire au rayon (ou le long, une fois sur trois).
        angle = (
            a + (math.pi / 2 if rng.random() < 0.67 else 0.0) + rng.uniform(-0.15, 0.15)
        )
        h = rng.uniform(0.09, 0.15) * size_scale
        parts += house(
            (x, y, 0.0),
            (w, d, h),
            rng.choice(roofs),
            rng.choice(walls),
            angle=angle,
            hip=rng.random() < 0.2,
        )
    return parts


def church(x, y, angle, scale=1.0, roof="Slate"):
    """Parish church: nave, choir, west tower with spire."""
    parts = []
    ca, sa = math.cos(angle), math.sin(angle)

    def at(dx, dy):
        return x + dx * ca - dy * sa, y + dx * sa + dy * ca

    nx, ny = at(0.0, 0.0)
    parts.append(
        box(
            (0.42 * scale, 0.2 * scale, 0.2 * scale + FOUNDATION),
            (nx, ny, (0.2 * scale - FOUNDATION) / 2),
            "StoneLight",
            rotation=(0, 0, angle),
        )
    )
    parts.append(
        gable_roof(
            0.42 * scale,
            0.2 * scale,
            0.14 * scale,
            (nx, ny, 0.2 * scale),
            roof,
            angle=angle,
        )
    )
    cx, cy = at(0.27 * scale, 0.0)
    parts.append(
        box(
            (0.14 * scale, 0.16 * scale, 0.17 * scale + FOUNDATION),
            (cx, cy, (0.17 * scale - FOUNDATION) / 2),
            "StoneLight",
            rotation=(0, 0, angle),
        )
    )
    parts.append(
        hip_roof(
            0.14 * scale,
            0.16 * scale,
            0.1 * scale,
            (cx, cy, 0.17 * scale),
            roof,
            angle=angle,
        )
    )
    tx, ty = at(-0.25 * scale, 0.0)
    parts += square_tower(
        tx, ty, 0.12 * scale, 0.42 * scale, roof, spire=0.32 * scale, angle=angle
    )
    return parts


# --- Buildings ---------------------------------------------------------------------


def cathedral_building(x=0.0, y=0.0, angle=0.0, s=1.0):
    """Gothic cathedral: nave, aisles, transept, apse, twin west towers, crossing spire."""
    parts = []
    ca, sa = math.cos(angle), math.sin(angle)

    def at(dx, dy):
        return x + dx * ca - dy * sa, y + dx * sa + dy * ca

    def b(size, d, z, mat="StoneLight"):
        px, py = at(*d)
        return box(
            (size[0] * s, size[1] * s, size[2] * s),
            (px, py, z * s),
            mat,
            rotation=(0, 0, angle),
        )

    parts.append(b((1.5, 0.36, 0.62 + FOUNDATION), (0, 0), (0.62 - FOUNDATION) / 2))
    px, py = at(0, 0)
    parts.append(
        gable_roof(
            1.5 * s, 0.36 * s, 0.26 * s, (px, py, 0.62 * s), "Slate", angle=angle
        )
    )
    for side in (-1, 1):
        parts.append(
            b(
                (1.3, 0.16, 0.36 + FOUNDATION),
                (0.05, side * 0.26),
                (0.36 - FOUNDATION) / 2,
            )
        )
        ax, ay = at(0.05, side * 0.26)
        parts.append(
            gable_roof(
                1.3 * s, 0.16 * s, 0.07 * s, (ax, ay, 0.36 * s), "Slate", angle=angle
            )
        )
        for k in range(5):  # buttresses
            parts.append(
                b((0.05, 0.1, 0.5), (-0.5 + k * 0.25, side * 0.37), 0.2, "Stone")
            )
    parts.append(b((0.36, 1.0, 0.58 + FOUNDATION), (0.3, 0), (0.58 - FOUNDATION) / 2))
    tx, ty = at(0.3, 0)
    parts.append(
        gable_roof(
            1.0 * s,
            0.36 * s,
            0.24 * s,
            (tx, ty, 0.58 * s),
            "Slate",
            angle=angle + math.pi / 2,
        )
    )
    apx, apy = at(0.78, 0)
    parts.append(
        cylinder(
            0.19 * s,
            (0.56 + FOUNDATION) * s,
            (apx, apy, (0.56 - FOUNDATION) / 2 * s),
            "StoneLight",
            10,
        )
    )
    parts.append(cone(0.21 * s, 0.24 * s, (apx, apy, 0.68 * s), "Slate", 10))
    for side in (-1, 1):
        wx, wy = at(-0.82, side * 0.16)
        parts.append(
            box(
                (0.24 * s, 0.24 * s, (1.05 + FOUNDATION) * s),
                (wx, wy, (1.05 - FOUNDATION) / 2 * s),
                "StoneLight",
                rotation=(0, 0, angle),
            )
        )
        parts.append(
            cone(
                0.19 * s,
                0.42 * s,
                (wx, wy, 1.26 * s),
                "Slate",
                4,
                rotation=(0, 0, angle + math.pi / 4),
            )
        )
    parts.append(
        box(
            (0.1 * s, 0.1 * s, 0.34 * s),
            (tx, ty, 0.85 * s),
            "StoneLight",
            rotation=(0, 0, angle),
        )
    )
    parts.append(cone(0.08 * s, 0.6 * s, (tx, ty, 1.32 * s), "Slate", 8))
    parts.append(sphere(0.022 * s, (tx, ty, 1.63 * s), "Gold", 1))
    return parts


def castle_keep(x, y, s=1.0):
    """Donjon and inner bailey with four round towers."""
    parts = []
    half = 0.36 * s
    corners = [
        (x - half, y - half),
        (x + half, y - half),
        (x + half, y + half),
        (x - half, y + half),
    ]
    for index in range(4):
        x0, y0 = corners[index]
        x1, y1 = corners[(index + 1) % 4]
        parts += wall_segment(x0, y0, x1, y1, 0.3 * s, 0.07)
        parts += round_tower(x0, y0, 0.08 * s, 0.48 * s, "Slate", 8)
    parts.append(
        box(
            (0.3 * s, 0.3 * s, (0.72 + FOUNDATION) * s),
            (x + 0.04 * s, y + 0.03 * s, (0.72 - FOUNDATION) / 2 * s),
            "Stone",
        )
    )
    k = 0.15 * s
    kx, ky = x + 0.04 * s, y + 0.03 * s
    for a, b_ in (
        ((-k, -k), (k, -k)),
        ((k, -k), (k, k)),
        ((k, k), (-k, k)),
        ((-k, k), (-k, -k)),
    ):
        parts += merlons(
            kx + a[0],
            ky + a[1],
            kx + b_[0],
            ky + b_[1],
            0.72 * s,
            "Stone",
            size=0.045,
            spacing=0.09,
        )
    parts += round_tower(kx + k, ky + k, 0.06 * s, 0.92 * s, "Slate", 8)
    parts += house(
        (x - 0.12 * s, y - 0.2 * s, 0.0),
        (0.34 * s, 0.12 * s, 0.16 * s),
        "Slate",
        "Stone",
    )
    parts.append(cylinder(0.006, 0.3 * s, (kx + k, ky + k, 1.22 * s), "Wood", 4))
    parts.append(
        box(
            (0.005, 0.14 * s, 0.09 * s), (kx + k, ky + k + 0.07 * s, 1.31 * s), "Banner"
        )
    )
    return parts


# --- Models ------------------------------------------------------------------------


def build_castle():
    """Faction capital: large walled city, castle on the high side, cathedral-sized church."""
    rng = random.Random(1337)
    points = ring_points(1.45, 12, rng, 0.08)
    parts = ground_patch(1.38)
    parts += town_walls(points, 0.24, 0.08, gates=(1, 5, 9))
    parts += castle_keep(0.62, -0.55, 1.0)
    parts += church(-0.35, 0.35, 0.4, 1.3)
    parts += scatter_houses(
        rng,
        70,
        1.3,
        [(0.62, -0.55, 0.62), (-0.35, 0.35, 0.42), (0.0, 0.0, 0.14)],
        1.05,
        inside=[(x * 0.93, y * 0.93) for x, y in points],
    )
    return parts


def build_city_cathedral():
    """Walled episcopal city dominated by its cathedral."""
    rng = random.Random(4242)
    points = ring_points(1.3, 11, rng, 0.1)
    parts = ground_patch(1.25)
    parts += town_walls(points, 0.22, 0.075, gates=(0, 4, 8))
    parts += cathedral_building(0.05, 0.1, 0.25, 0.62)
    parts += scatter_houses(
        rng,
        58,
        1.18,
        [(0.05, 0.1, 0.62)],
        1.05,
        inside=[(x * 0.92, y * 0.92) for x, y in points],
    )
    return parts


def build_town():
    """Walled market town: irregular walls, towers, a parish church and ~45 houses."""
    rng = random.Random(7)
    points = ring_points(1.05, 9, rng, 0.12)
    parts = ground_patch(1.0)
    parts += town_walls(points, 0.2, 0.07, gates=(0, 5))
    parts += church(0.1, 0.12, 0.8, 1.0)
    parts += scatter_houses(
        rng,
        44,
        0.95,
        [(0.1, 0.12, 0.34)],
        1.08,
        inside=[(x * 0.9, y * 0.9) for x, y in points],
    )
    return parts


def build_village():
    """Open village along a road: thatch and tile cottages, barns, a small church."""
    rng = random.Random(99)
    parts = [
        box(
            (2.2, 0.12, 0.01 + FOUNDATION),
            (0, 0, 0.005 - FOUNDATION / 2),
            "Dirt",
            rotation=(0, 0, 0.3),
        )
    ]
    parts += church(0.05, 0.28, 0.3, 0.8, roof="TileOld")
    parts += scatter_houses(
        rng,
        16,
        0.9,
        [(0.05, 0.28, 0.3)],
        1.3,
        roofs=("Thatch", "Thatch", "TileOld", "Tile"),
        walls=("Plaster", "Timber"),
        size_scale=1.05,
    )
    for dx, dy, angle in ((-0.75, -0.35, 0.3), (0.72, 0.4, 1.9)):
        parts += house((dx, dy, 0.0), (0.3, 0.16, 0.12), "Thatch", "Wood", angle=angle)
    return parts


def build_cathedral():
    """Cathedral building alone (siege battlefield decoration)."""
    return cathedral_building(0.0, 0.0, 0.0, 1.0)


def _horse(x, y, z, parts, coat="Horse", caparison=True):
    """Low-poly horse facing +X with its feet at ``z``."""
    parts.append(box((0.62, 0.22, 0.24), (x, y, z + 0.55), coat))
    parts.append(sphere(0.13, (x - 0.24, y, z + 0.56), coat, 1, scale=(1.2, 0.9, 1.0)))
    parts.append(
        box(
            (0.16, 0.12, 0.3),
            (x + 0.33, y, z + 0.72),
            coat,
            rotation=(0, math.radians(-35), 0),
        )
    )
    parts.append(
        box(
            (0.24, 0.1, 0.11),
            (x + 0.47, y, z + 0.84),
            coat,
            rotation=(0, math.radians(25), 0),
        )
    )
    for dx in (-0.22, 0.22):
        for dy in (-0.07, 0.07):
            parts.append(box((0.06, 0.06, 0.46), (x + dx, y + dy, z + 0.23), coat))
    parts.append(
        box(
            (0.07, 0.04, 0.3),
            (x - 0.36, y, z + 0.5),
            "Wood",
            rotation=(0, math.radians(25), 0),
        )
    )
    if caparison:
        parts.append(box((0.58, 0.26, 0.16), (x, y, z + 0.43), "Banner"))


def _man(x, y, z, parts, body="Steel", helm="Steel", surcoat="Banner", seated=False):
    """Standing (or seated) man facing +X with feet at ``z``: legs, surcoat, arms, helmet."""
    if not seated:
        for dy in (-0.05, 0.05):
            parts.append(box((0.07, 0.06, 0.34), (x, y + dy, z + 0.17), "Leather"))
        base = z + 0.34
    else:
        for dy in (-0.13, 0.13):
            parts.append(
                box((0.16, 0.05, 0.08), (x + 0.04, y + dy, z + 0.02), "Leather")
            )
        base = z
    parts.append(box((0.14, 0.2, 0.3), (x, y, base + 0.15), body))
    parts.append(box((0.15, 0.21, 0.2), (x, y, base + 0.1), surcoat))
    parts.append(sphere(0.075, (x, y, base + 0.38), "Skin", 1))
    parts.append(sphere(0.085, (x, y, base + 0.41), helm, 1, scale=(1.0, 1.0, 0.8)))
    return base


def build_army():
    """Mounted commander: armoured knight on a caparisoned horse, lance with pennon."""
    parts = []
    _horse(0.0, 0.0, 0.0, parts)
    base = _man(-0.02, 0.0, 0.68, parts, seated=True)
    parts.append(box((0.05, 0.16, 0.2), (0.05, 0.13, base + 0.13), "Steel"))  # arm
    parts.append(box((0.03, 0.2, 0.26), (0.03, -0.16, base + 0.12), "Banner"))  # shield
    parts.append(
        cylinder(
            0.014,
            1.3,
            (0.05, 0.2, base + 0.45),
            "Wood",
            5,
            rotation=(0, math.radians(-12), 0),
        )
    )
    parts.append(
        box(
            (0.004, 0.2, 0.1),
            (0.2, 0.3, base + 1.0),
            "Banner",
            rotation=(0, math.radians(-12), 0),
        )
    )
    return parts


def build_army_foot():
    """Man-at-arms: mail, surcoat, kite shield and a tall spear."""
    parts = []
    base = _man(0.0, 0.0, 0.0, parts)
    parts.append(box((0.04, 0.2, 0.34), (0.1, -0.13, base + 0.1), "Banner"))  # shield
    parts.append(box((0.045, 0.21, 0.035), (0.1, -0.13, base + 0.28), "Steel"))
    parts.append(cylinder(0.012, 1.05, (0.07, 0.14, base + 0.2), "Wood", 5))
    parts.append(cone(0.022, 0.08, (0.07, 0.14, base + 0.76), "Steel", 4))
    return parts


def build_army_archer():
    """Crossbowman: gambeson, kettle hat, crossbow held at the chest, pavise on the back."""
    parts = []
    base = _man(0.0, 0.0, 0.0, parts, body="Cloth", helm="Steel")
    parts.append(
        cylinder(0.12, 0.012, (0.0, 0.0, base + 0.44), "Steel", 8)
    )  # kettle hat brim
    parts.append(
        box((0.26, 0.04, 0.04), (0.14, 0.0, base + 0.2), "Wood")
    )  # crossbow stock
    parts.append(box((0.03, 0.26, 0.025), (0.26, 0.0, base + 0.21), "Wood"))  # bow
    parts.append(box((0.03, 0.26, 0.42), (-0.1, 0.0, base + 0.1), "Banner"))  # pavise
    return parts


def build_siege_camp():
    """Siege camp: striped tents, palisade, a trebuchet and a campfire."""
    rng = random.Random(3)
    parts = [
        cylinder(1.3, 0.01 + FOUNDATION, (0, 0, 0.005 - FOUNDATION / 2), "Dirt", 12)
    ]
    for x, y in (
        (-0.6, -0.4),
        (-0.15, -0.72),
        (0.45, -0.55),
        (-0.75, 0.25),
        (-0.3, 0.55),
    ):
        size = rng.uniform(0.24, 0.32)
        parts.append(
            cone(
                size,
                size * 1.6,
                (x, y, size * 0.8),
                rng.choice(["Canvas", "Canvas", "Banner"]),
                8,
            )
        )
        parts.append(cylinder(0.012, 0.2, (x, y, size * 1.6 + 0.05), "Wood", 4))
    for index in range(14):
        angle = math.radians(160 + index * 12)
        parts.append(
            cone(
                0.035,
                0.34,
                (1.15 * math.cos(angle) + 0.25, 1.15 * math.sin(angle) + 0.35, 0.12),
                "Wood",
                4,
                rotation=(math.radians(rng.uniform(-8, 8)), 0, 0),
            )
        )
    for y in (-0.12, 0.12):
        parts.append(
            box(
                (0.06, 0.05, 0.9),
                (0.5, 0.5 + y, 0.47),
                "Wood",
                rotation=(0, math.radians(12), 0),
            )
        )
        parts.append(
            box(
                (0.06, 0.05, 0.9),
                (0.72, 0.5 + y, 0.47),
                "Wood",
                rotation=(0, math.radians(-12), 0),
            )
        )
    parts.append(box((0.7, 0.36, 0.05), (0.61, 0.5, 0.05), "Wood"))
    parts.append(
        box(
            (1.4, 0.04, 0.04),
            (0.61, 0.5, 0.95),
            "Wood",
            rotation=(0, math.radians(-35), 0),
        )
    )
    parts.append(box((0.18, 0.18, 0.18), (1.1, 0.5, 1.2), "DarkStone"))
    parts.append(cone(0.08, 0.16, (-0.1, 0.05, 0.1), "Fire", 5))
    for angle in (0.0, 2.1, 4.2):
        parts.append(
            box(
                (0.22, 0.035, 0.035), (-0.1, 0.05, 0.03), "Wood", rotation=(0, 0, angle)
            )
        )
    return parts


def build_ship():
    """Cog: rounded clinker hull, fore and stern castles, mast, square sail with a band."""
    parts = []
    # Coque : sections elliptiques le long de X (lofting manuel).
    sections = [
        (-0.95, 0.12, 0.62),
        (-0.7, 0.28, 0.5),
        (-0.3, 0.34, 0.44),
        (0.2, 0.34, 0.44),
        (0.6, 0.26, 0.5),
        (0.95, 0.06, 0.66),
    ]
    ring = 8
    verts = []
    for x, half_width, top in sections:
        for index in range(ring + 1):
            t = (
                index / ring
            )  # 0 = bord bâbord haut, 1 = tribord haut, en passant par la quille
            angle = math.pi * t
            verts.append(
                (
                    x,
                    -half_width * math.cos(angle),
                    top - (top - 0.02) * math.sin(angle) ** 0.8,
                )
            )
    faces = []
    per = ring + 1
    for s in range(len(sections) - 1):
        for index in range(ring):
            a = s * per + index
            faces.append((a, a + 1, a + per + 1, a + per))
    parts.append(mesh_object("hull", verts, faces, "Wood"))
    parts.append(box((1.5, 0.6, 0.03), (0.0, 0, 0.42), "Leather"))  # deck
    parts.append(box((0.36, 0.5, 0.22), (-0.78, 0, 0.72), "Wood"))  # stern castle
    parts.append(box((0.26, 0.36, 0.18), (0.8, 0, 0.74), "Wood"))  # fore castle
    parts.append(cylinder(0.03, 1.9, (0.0, 0, 1.3), "Wood", 6))
    parts.append(box((0.03, 0.95, 0.04), (0.02, 0, 1.95), "Wood"))
    parts.append(box((0.02, 0.9, 0.82), (0.04, 0, 1.52), "Canvas"))
    parts.append(box((0.025, 0.9, 0.2), (0.045, 0, 1.52), "Banner"))
    return parts


MODELS = {
    "castle": build_castle,
    "city_cathedral": build_city_cathedral,
    "town": build_town,
    "village": build_village,
    "cathedral": build_cathedral,
    "army": build_army,
    "army_foot": build_army_foot,
    "army_archer": build_army_archer,
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
