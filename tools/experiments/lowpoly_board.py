"""Comparison board: one campaign vignette with the current DN models vs code-made low poly.

Run inside Blender (Eevee, background):
    blender -b --factory-startup -P tools/experiments/lowpoly_board.py -- \
        --models game/assets/models/dn --out ~/dev/cent-ans-raw/lowpoly/board

Writes <out>_current.png and <out>_lowpoly.png from the same camera, sun and layout:
a castle on a hill, a village with its church, a water mill on the river, a
windmill, fields and a wood. The low poly side is built only from parametric
primitives (boxes, prisms, 8-sided cylinders, icospheres) with flat shading and
one muted palette, which is what a procedural pipeline would produce.
"""

import math
import random
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Vector

PALETTE = {
    "grass": (0.36, 0.48, 0.25),
    "grass_dark": (0.27, 0.38, 0.19),
    "wheat": (0.78, 0.66, 0.36),
    "fallow": (0.52, 0.42, 0.28),
    "meadow": (0.45, 0.56, 0.28),
    "water": (0.30, 0.45, 0.55),
    "stone": (0.78, 0.74, 0.64),
    "stone_dark": (0.58, 0.55, 0.50),
    "plaster": (0.90, 0.86, 0.76),
    "timber": (0.36, 0.25, 0.17),
    "tile": (0.62, 0.30, 0.20),
    "slate": (0.33, 0.36, 0.42),
    "thatch": (0.68, 0.57, 0.34),
    "leaf": (0.25, 0.40, 0.18),
    "leaf_light": (0.36, 0.50, 0.22),
    "conifer": (0.17, 0.30, 0.17),
    "trunk": (0.30, 0.22, 0.15),
    "sail": (0.88, 0.84, 0.74),
}

CASTLE = (-26.0, 14.0)
CHURCH = (8.0, 2.0)
HOUSES = [
    (0.0, -6.0, 15),
    (6.0, -9.0, -20),
    (14.0, -6.0, 75),
    (17.0, 3.0, 95),
    (-2.0, 6.0, 5),
    (2.0, 11.0, -10),
    (13.0, 10.0, 30),
    (-6.0, -1.0, 80),
]
MILL = (24.0, -14.0)
WINDMILL = (-6.0, -20.0)
FIELDS = [
    (-30.0, -18.0, 16.0, 10.0, "wheat", 8),
    (-14.0, -26.0, 14.0, 9.0, "fallow", -6),
    (-34.0, -32.0, 12.0, 9.0, "meadow", 4),
    (4.0, -26.0, 13.0, 8.0, "wheat", -3),
    (34.0, 10.0, 12.0, 14.0, "fallow", 12),
    (30.0, 26.0, 14.0, 8.0, "wheat", 0),
]
WOOD = ((-4.0, 30.0), 13.0, 34)

CURRENT = {
    "castle": ("buildings/castle_west_lod1.glb", 30.0),
    "church": ("buildings/env_chapel_pilgrim_lod1.glb", 13.0),
    "houses": [
        ("buildings/house_brick_gable_lod1.glb", 6.5),
        ("buildings/farm_norman_timber_courtyard_lod1.glb", 9.0),
        ("buildings/farm_german_fachwerk_lod1.glb", 8.0),
        ("buildings/hall_house_lowgerman_lod1.glb", 7.5),
    ],
    "mill": ("buildings/village_mill_watermill_small_lod1.glb", 9.0),
    "windmill": ("buildings/econ_mill_windmill_post_italian_lod1.glb", 8.0),
    "trees": [
        ("vegetation/tree_beech_lod1.glb", 6.0),
        ("vegetation/tree_ash_lod1.glb", 6.0),
        ("vegetation/tree_alder_lod1.glb", 5.0),
    ],
}


def parse_args():
    """Read the arguments after '--'."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    options = {"--models": "game/assets/models/dn", "--out": "/tmp/board"}
    for key, value in zip(argv[::2], argv[1::2], strict=False):
        options[key] = value
    return Path(options["--models"]).expanduser(), Path(options["--out"]).expanduser()


def height(x: float, y: float) -> float:
    """Gentle rolling ground with the castle hill."""
    hill = 7.0 * math.exp(-((x - CASTLE[0]) ** 2 + (y - CASTLE[1]) ** 2) / 260.0)
    roll = 0.8 * math.sin(x * 0.09) * math.cos(y * 0.07) + 0.5 * math.sin(
        x * 0.05 + y * 0.04
    )
    return hill + roll - river_cut(x, y)


def river_distance(x: float, y: float) -> float:
    """Distance to the river centreline (a gentle diagonal curve)."""
    centre_y = -14.0 + 0.25 * (x - 24.0) + 3.0 * math.sin(x * 0.08)
    return abs(y - centre_y) * 0.97


def river_cut(x: float, y: float) -> float:
    """Valley carved under the river."""
    return 1.6 * math.exp(-(river_distance(x, y) ** 2) / 18.0)


def material(name: str, flat: bool):
    """Principled material from the palette; flat = matte low poly."""
    key = f"{name}_{'flat' if flat else 'smooth'}"
    if key in bpy.data.materials:
        return bpy.data.materials[key]
    mat = bpy.data.materials.new(key)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*PALETTE[name], 1.0)
    bsdf.inputs["Roughness"].default_value = 0.9 if name != "water" else 0.25
    return mat


def mesh_object(name: str, build, mat, collection):
    """New object from a bmesh builder, flat shaded."""
    data = bpy.data.meshes.new(name)
    bm = bmesh.new()
    build(bm)
    bm.to_mesh(data)
    bm.free()
    for polygon in data.polygons:
        polygon.use_smooth = False
    data.materials.append(mat)
    obj = bpy.data.objects.new(name, data)
    collection.objects.link(obj)
    return obj


def box(bm, centre, size, rotation=0.0):
    """Axis box (rotated about Z) with its base at centre.z."""
    cx, cy, cz = centre
    sx, sy, sz = size
    cos, sin = math.cos(rotation), math.sin(rotation)
    corners = []
    for dz in (0, sz):
        for dx, dy in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            x, y = dx * sx / 2, dy * sy / 2
            corners.append(
                bm.verts.new((cx + x * cos - y * sin, cy + x * sin + y * cos, cz + dz))
            )
    for face in (
        (0, 3, 2, 1),
        (4, 5, 6, 7),
        (0, 1, 5, 4),
        (1, 2, 6, 5),
        (2, 3, 7, 6),
        (3, 0, 4, 7),
    ):
        bm.faces.new([corners[i] for i in face])


def gable_roof(bm, centre, size, rise, rotation=0.0, overhang=0.35):
    """Two-slope roof along the local X axis, sitting at centre.z."""
    cx, cy, cz = centre
    sx, sy = size[0] + overhang * 2, size[1] + overhang * 2
    cos, sin = math.cos(rotation), math.sin(rotation)

    def vert(x, y, z):
        return bm.verts.new((cx + x * cos - y * sin, cy + x * sin + y * cos, cz + z))

    a, b = vert(-sx / 2, -sy / 2, 0), vert(sx / 2, -sy / 2, 0)
    c, d = vert(sx / 2, sy / 2, 0), vert(-sx / 2, sy / 2, 0)
    e, f = vert(-sx / 2, 0, rise), vert(sx / 2, 0, rise)
    for face in ((a, b, f, e), (c, d, e, f), (a, e, d), (b, c, f), (a, d, c, b)):
        bm.faces.new(face)


def cylinder(bm, centre, radius, depth, sides=8, top_radius=None):
    """Prism/frustum with its base at centre.z."""
    cx, cy, cz = centre
    top_radius = radius if top_radius is None else top_radius
    bottom = [
        bm.verts.new((cx + radius * math.cos(a), cy + radius * math.sin(a), cz))
        for a in _angles(sides)
    ]
    if top_radius <= 1e-6:
        apex = bm.verts.new((cx, cy, cz + depth))
        for i in range(sides):
            bm.faces.new((bottom[i], bottom[(i + 1) % sides], apex))
    else:
        top = [
            bm.verts.new(
                (
                    cx + top_radius * math.cos(a),
                    cy + top_radius * math.sin(a),
                    cz + depth,
                )
            )
            for a in _angles(sides)
        ]
        for i in range(sides):
            j = (i + 1) % sides
            bm.faces.new((bottom[i], bottom[j], top[j], top[i]))
        bm.faces.new(top)
    bm.faces.new(list(reversed(bottom)))


def _angles(sides):
    return [2 * math.pi * i / sides + math.pi / sides for i in range(sides)]


def crenels(bm, start, end, z, spacing=1.2, size=0.55):
    """Merlons along a wall top."""
    length = (Vector(end) - Vector(start)).length
    count = max(int(length / spacing), 1)
    angle = math.atan2(end[1] - start[1], end[0] - start[0])
    for i in range(count + 1):
        t = i / count
        x, y = start[0] + (end[0] - start[0]) * t, start[1] + (end[1] - start[1]) * t
        box(bm, (x, y, z), (size, size, size), angle)


def build_terrain(collection, flat: bool):
    """Ground grid; coarse and faceted for low poly, fine and smooth otherwise."""
    steps = 36 if flat else 160
    extent = 70.0

    def build(bm):
        rows = []
        for j in range(steps + 1):
            row = []
            for i in range(steps + 1):
                x = -extent + 2 * extent * i / steps
                y = -extent + 2 * extent * j / steps
                if flat:
                    rng = random.Random(i * 1000 + j)
                    x += rng.uniform(-0.8, 0.8)
                    y += rng.uniform(-0.8, 0.8)
                row.append(bm.verts.new((x, y, height(x, y))))
            rows.append(row)
        for j in range(steps):
            for i in range(steps):
                quad = (rows[j][i], rows[j][i + 1], rows[j + 1][i + 1], rows[j + 1][i])
                if flat:
                    bm.faces.new(quad[:3])
                    bm.faces.new((quad[0], quad[2], quad[3]))
                else:
                    bm.faces.new(quad)

    ground = mesh_object("ground", build, material("grass", flat), collection)
    if not flat:
        for polygon in ground.data.polygons:
            polygon.use_smooth = True
    ground.data.materials.append(material("grass_dark", flat))
    for polygon in ground.data.polygons:  # darker on slopes: cheap relief reading
        if polygon.normal.z < 0.93:
            polygon.material_index = 1


def build_water(collection, flat: bool):
    """River ribbon following the centreline, just above the carved bed."""

    def build(bm):
        left, right = [], []
        for i in range(61):
            x = -70.0 + 140.0 * i / 60
            centre_y = -14.0 + 0.25 * (x - 24.0) + 3.0 * math.sin(x * 0.08)
            z = height(x, centre_y) + 0.9
            left.append(bm.verts.new((x, centre_y - 2.4, z)))
            right.append(bm.verts.new((x, centre_y + 2.4, z)))
        for i in range(60):
            bm.faces.new((left[i], left[i + 1], right[i + 1], right[i]))

    mesh_object("river", build, material("water", flat), collection)


def build_fields(collection, flat: bool):
    """Coloured parcels draped on the ground (shared by both sides)."""
    for index, (x, y, w, d, kind, degrees) in enumerate(FIELDS):
        angle = math.radians(degrees)

        def build(bm, x=x, y=y, w=w, d=d, angle=angle):
            n = 6
            grid = []
            for j in range(n + 1):
                row = []
                for i in range(n + 1):
                    lx, ly = (i / n - 0.5) * w, (j / n - 0.5) * d
                    px = x + lx * math.cos(angle) - ly * math.sin(angle)
                    py = y + lx * math.sin(angle) + ly * math.cos(angle)
                    row.append(bm.verts.new((px, py, height(px, py) + 0.08)))
                grid.append(row)
            for j in range(n):
                for i in range(n):
                    bm.faces.new(
                        (grid[j][i], grid[j][i + 1], grid[j + 1][i + 1], grid[j + 1][i])
                    )

        mesh_object(f"field_{index}", build, material(kind, flat), collection)


def lowpoly_house(collection, x, y, degrees, rng, index):
    """Timber-framed or stone house with a tile or thatch roof."""
    angle = math.radians(degrees)
    w, d, h = rng.uniform(5.0, 7.5), rng.uniform(3.6, 4.6), rng.uniform(3.0, 4.2)
    z = height(x, y) - 0.2
    wall = rng.choice(["plaster", "plaster", "stone"])
    roof = rng.choice(["tile", "tile", "thatch", "slate"])
    mesh_object(
        f"house_{index}",
        lambda bm: box(bm, (x, y, z), (w, d, h), angle),
        material(wall, True),
        collection,
    )
    mesh_object(
        f"roof_{index}",
        lambda bm: gable_roof(bm, (x, y, z + h), (w, d), d * 0.75, angle),
        material(roof, True),
        collection,
    )
    if wall == "plaster":  # timber posts at the corners and middle

        def posts(bm):
            for fx in (-0.5, 0.0, 0.5):
                for fy in (-0.5, 0.5):
                    lx, ly = fx * w, fy * d * 1.01
                    px = x + lx * math.cos(angle) - ly * math.sin(angle)
                    py = y + lx * math.sin(angle) + ly * math.cos(angle)
                    box(bm, (px, py, z), (0.3, 0.3, h), angle)

        mesh_object(f"posts_{index}", posts, material("timber", True), collection)


def lowpoly_church(collection, x, y):
    """Nave, choir, west tower and spire."""
    z = height(x, y) - 0.3
    mesh_object(
        "nave",
        lambda bm: box(bm, (x, y, z), (12.0, 5.5, 6.5)),
        material("stone", True),
        collection,
    )
    mesh_object(
        "nave_roof",
        lambda bm: gable_roof(bm, (x, y, z + 6.5), (12.0, 5.5), 3.6),
        material("slate", True),
        collection,
    )
    mesh_object(
        "choir",
        lambda bm: cylinder(bm, (x + 6.0, y, z), 2.6, 5.5, 8),
        material("stone", True),
        collection,
    )
    mesh_object(
        "choir_roof",
        lambda bm: cylinder(bm, (x + 6.0, y, z + 5.5), 3.0, 2.6, 8, 0.0),
        material("slate", True),
        collection,
    )
    mesh_object(
        "tower",
        lambda bm: box(bm, (x - 7.5, y, z), (3.8, 3.8, 12.0)),
        material("stone", True),
        collection,
    )
    mesh_object(
        "spire",
        lambda bm: cylinder(bm, (x - 7.5, y, z + 12.0), 2.7, 8.0, 4, 0.0),
        material("slate", True),
        collection,
    )


def lowpoly_castle(collection, x, y):
    """Square curtain wall, corner towers with cone roofs, crenellated keep."""
    z = height(x, y) - 0.5
    half = 10.0
    corners = [
        (x - half, y - half),
        (x + half, y - half),
        (x + half, y + half),
        (x - half, y + half),
    ]

    def walls(bm):
        for (ax, ay), (bx, by) in zip(corners, corners[1:] + corners[:1], strict=True):
            length = math.hypot(bx - ax, by - ay)
            box(
                bm,
                ((ax + bx) / 2, (ay + by) / 2, z),
                (length, 1.6, 7.0),
                math.atan2(by - ay, bx - ax),
            )
            crenels(bm, (ax, ay), (bx, by), z + 7.0)

    mesh_object("curtain", walls, material("stone", True), collection)

    def towers(bm):
        for cx, cy in corners:
            cylinder(bm, (cx, cy, z), 2.6, 10.0, 8)

    mesh_object("towers", towers, material("stone", True), collection)

    def tower_roofs(bm):
        for cx, cy in corners:
            cylinder(bm, (cx, cy, z + 10.0), 3.1, 5.0, 8, 0.0)

    mesh_object("tower_roofs", tower_roofs, material("slate", True), collection)

    def keep(bm):
        box(bm, (x + 2.0, y + 2.0, z), (8.0, 8.0, 15.0))
        for (ax, ay), (bx, by) in (
            ((x - 2, y - 2), (x + 6, y - 2)),
            ((x + 6, y - 2), (x + 6, y + 6)),
            ((x + 6, y + 6), (x - 2, y + 6)),
            ((x - 2, y + 6), (x - 2, y - 2)),
        ):
            crenels(bm, (ax, ay), (bx, by), z + 15.0)

    mesh_object("keep", keep, material("stone_dark", True), collection)

    def hall(bm):
        box(bm, (x - 4.5, y - 4.5, z), (9.0, 5.0, 5.5))

    mesh_object("hall", hall, material("plaster", True), collection)
    mesh_object(
        "hall_roof",
        lambda bm: gable_roof(bm, (x - 4.5, y - 4.5, z + 5.5), (9.0, 5.0), 3.0),
        material("tile", True),
        collection,
    )


def lowpoly_mill(collection, x, y):
    """Stone mill with a paddle wheel on the river bank."""
    z = height(x, y) + 0.3
    mesh_object(
        "mill",
        lambda bm: box(bm, (x, y + 4.0, z), (6.0, 5.0, 4.5)),
        material("stone", True),
        collection,
    )
    mesh_object(
        "mill_roof",
        lambda bm: gable_roof(bm, (x, y + 4.0, z + 4.5), (6.0, 5.0), 3.2),
        material("thatch", True),
        collection,
    )

    def wheel(bm):
        result = bmesh.ops.create_cone(
            bm, segments=12, radius1=2.2, radius2=2.2, depth=0.8, cap_ends=True
        )
        bmesh.ops.rotate(
            bm, verts=result["verts"], cent=(0, 0, 0), matrix=_rotation_x()
        )
        bmesh.ops.translate(bm, verts=result["verts"], vec=(x, y + 1.0, z + 1.4))

    mesh_object("wheel", wheel, material("timber", True), collection)


def _rotation_x():
    from mathutils import Matrix

    return Matrix.Rotation(math.radians(90), 3, "X")


def lowpoly_windmill(collection, x, y):
    """Tower windmill: tapered body, cone cap, four sails."""
    z = height(x, y) - 0.2
    mesh_object(
        "windmill",
        lambda bm: cylinder(bm, (x, y, z), 2.6, 9.0, 8, 2.0),
        material("plaster", True),
        collection,
    )
    mesh_object(
        "windmill_cap",
        lambda bm: cylinder(bm, (x, y, z + 9.0), 2.4, 2.6, 8, 0.0),
        material("thatch", True),
        collection,
    )

    def sails(bm):
        hub = Vector((x, y - 2.4, z + 8.0))
        for k in range(4):
            angle = math.radians(20 + 90 * k)
            direction = Vector((math.cos(angle), 0, math.sin(angle)))
            centre = hub + direction * 3.4
            result = bmesh.ops.create_cube(bm, size=1.0)
            from mathutils import Matrix

            scale = Matrix.Diagonal((6.0, 0.15, 1.3)).to_3x3()
            bmesh.ops.transform(
                bm, verts=result["verts"], matrix=scale.to_4x4(), space=Matrix()
            )
            bmesh.ops.rotate(
                bm,
                verts=result["verts"],
                cent=(0, 0, 0),
                matrix=Matrix.Rotation(-angle, 3, "Y"),
            )
            bmesh.ops.translate(bm, verts=result["verts"], vec=centre)

    mesh_object("sails", sails, material("sail", True), collection)


def lowpoly_tree(collection, x, y, rng, index):
    """Broadleaf (jittered icosphere) or conifer (stacked cones)."""
    z = height(x, y) - 0.2
    if rng.random() < 0.3:

        def conifer(bm):
            for level in range(3):
                cylinder(
                    bm, (x, y, z + 1.5 + level * 2.0), 2.4 - level * 0.6, 3.2, 7, 0.0
                )

        mesh_object(f"conifer_{index}", conifer, material("conifer", True), collection)
    else:
        radius = rng.uniform(2.2, 3.2)

        def crown(bm):
            result = bmesh.ops.create_icosphere(bm, subdivisions=1, radius=radius)
            for vert in result["verts"]:
                vert.co *= rng.uniform(0.85, 1.12)
                vert.co.z *= 0.85
            bmesh.ops.translate(bm, verts=result["verts"], vec=(x, y, z + 2.2 + radius))

        leaf = "leaf" if rng.random() < 0.6 else "leaf_light"
        mesh_object(f"crown_{index}", crown, material(leaf, True), collection)
    mesh_object(
        f"trunk_{index}",
        lambda bm: cylinder(bm, (x, y, z), 0.35, 2.6, 5),
        material("trunk", True),
        collection,
    )


def wood_positions():
    """Tree spots of the wood plus a few hedgerow trees (same for both sides)."""
    rng = random.Random(7)
    (cx, cy), radius, count = WOOD
    spots = []
    while len(spots) < count:
        x, y = (
            cx + rng.uniform(-radius, radius) * 1.6,
            cy + rng.uniform(-radius, radius),
        )
        if all(math.hypot(x - sx, y - sy) > 4.0 for sx, sy in spots):
            spots.append((x, y))
    spots += [
        (-14.0, -12.0),
        (-10.0, 18.0),
        (26.0, 0.0),
        (30.0, -4.0),
        (22.0, 16.0),
        (-38.0, -6.0),
    ]
    return spots


def import_model(
    path: Path, footprint: float, x: float, y: float, degrees: float, collection
):
    """Import a glb, scale it to a horizontal footprint and set it on the ground."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path))
    added = [obj for obj in bpy.data.objects if obj not in before]
    root = bpy.data.objects.new(f"root_{path.stem}", None)
    collection.objects.link(root)
    for obj in added:
        for user in list(obj.users_collection):
            user.objects.unlink(obj)
        collection.objects.link(obj)
        if obj.parent is None:
            obj.parent = root
    bpy.context.view_layer.update()
    points = [
        obj.matrix_world @ Vector(corner)
        for obj in added
        if obj.type == "MESH"
        for corner in obj.bound_box
    ]
    low = Vector(
        (min(p.x for p in points), min(p.y for p in points), min(p.z for p in points))
    )
    high = Vector(
        (max(p.x for p in points), max(p.y for p in points), max(p.z for p in points))
    )
    scale = footprint / max(high.x - low.x, high.y - low.y)
    root.scale = (scale, scale, scale)
    root.rotation_euler = (0, 0, math.radians(degrees))
    centre = (low + high) / 2
    offset = Vector((centre.x, centre.y, low.z)) * scale
    offset.rotate(root.rotation_euler)
    root.location = Vector((x, y, height(x, y) - 0.15)) - offset


def build_current(models: Path, collection):
    """Same layout with the DN models of the game."""
    rng = random.Random(3)
    import_model(
        models / CURRENT["castle"][0], CURRENT["castle"][1], *CASTLE, 20, collection
    )
    import_model(
        models / CURRENT["church"][0], CURRENT["church"][1], *CHURCH, 0, collection
    )
    for index, (x, y, degrees) in enumerate(HOUSES):
        name, size = CURRENT["houses"][index % len(CURRENT["houses"])]
        import_model(models / name, size, x, y, degrees, collection)
    import_model(
        models / CURRENT["mill"][0],
        CURRENT["mill"][1],
        MILL[0],
        MILL[1] + 4.0,
        0,
        collection,
    )
    import_model(
        models / CURRENT["windmill"][0],
        CURRENT["windmill"][1],
        *WINDMILL,
        0,
        collection,
    )
    for x, y in wood_positions():
        name, size = rng.choice(CURRENT["trees"])
        import_model(
            models / name,
            size * rng.uniform(0.85, 1.15),
            x,
            y,
            rng.uniform(0, 360),
            collection,
        )


def build_lowpoly(collection):
    """Same layout from parametric primitives only."""
    rng = random.Random(3)
    lowpoly_castle(collection, *CASTLE)
    lowpoly_church(collection, *CHURCH)
    for index, (x, y, degrees) in enumerate(HOUSES):
        lowpoly_house(collection, x, y, degrees, rng, index)
    lowpoly_mill(collection, *MILL)
    lowpoly_windmill(collection, *WINDMILL)
    for index, (x, y) in enumerate(wood_positions()):
        lowpoly_tree(collection, x, y, rng, index)


def setup_scene():
    """Camera, sun, sky and Eevee settings shared by both renders."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x, scene.render.resolution_y = 1280, 800
    scene.view_settings.view_transform = "Standard"
    scene.world = bpy.data.worlds.new("sky")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs["Color"].default_value = (
        0.62,
        0.70,
        0.80,
        1,
    )
    scene.world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.45
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
    sun.data.energy = 4.0
    sun.data.angle = math.radians(3)
    sun.rotation_euler = (math.radians(48), 0, math.radians(-35))
    scene.collection.objects.link(sun)
    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    camera.data.lens = 40
    camera.location = (40.0, -78.0, 66.0)
    direction = Vector((-6.0, 4.0, 0.0)) - camera.location
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    scene.collection.objects.link(camera)
    scene.camera = camera
    return scene


def render(scene, target: Path):
    """Render the current scene to a PNG."""
    scene.render.filepath = str(target)
    bpy.ops.render.render(write_still=True)


def main():
    """Build and render both sides."""
    models, out = parse_args()
    out.parent.mkdir(parents=True, exist_ok=True)
    for side in ("current", "lowpoly"):
        scene = setup_scene()
        collection = scene.collection
        flat = side == "lowpoly"
        build_terrain(collection, flat)
        build_water(collection, flat)
        build_fields(collection, flat)
        if flat:
            build_lowpoly(collection)
        else:
            build_current(models, collection)
        render(scene, out.with_name(f"{out.name}_{side}.png"))


main()
