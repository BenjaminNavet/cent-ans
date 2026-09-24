"""Settlement models for the campaign map county view (lot C6), exported as glTF binaries.

Run headless:
    blender --background --python settlements.py -- <out_dir> [name ...]

Five settlement kinds, two variants each, plus three hamlet house groups (semi-realistic
14th-century style, ADR 0004), reusing the building helpers of ``models.py``:

* ``city_a`` / ``city_b``: walled episcopal city with its cathedral;
* ``town_a`` / ``town_b``: market town inside a curtain wall (``b`` is a square bastide);
* ``castle_a`` / ``castle_b``: keep on a motte with a bailey (``b`` with a village below);
* ``abbey_a`` / ``abbey_b``: abbey church, cloister, monastic ranges, precinct wall, gardens;
* ``village_a`` / ``village_b``: open village of cottages and barns around a parish church;
* ``hamlet_a`` / ``hamlet_b`` / ``hamlet_c``: 2 to 5 farm buildings (instanced by Godot).

Each model is one joined mesh (< 5 000 triangles) whose ``Banner`` material is tinted by Godot
with the controller's colour. Buildings extend below ``z = 0`` so that they sit on slopes.
A line ``MODEL <name> <triangles>`` is printed per model and ``OK`` at the end.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import bpy  # noqa: E402
import models as m  # noqa: E402

m.PALETTE.setdefault("Garden", ((0.12, 0.16, 0.05), 0.95, 0.0))
m.PALETTE.setdefault("Field", ((0.26, 0.22, 0.10), 1.0, 0.0))
m.PALETTE.setdefault("Earth", ((0.20, 0.17, 0.11), 1.0, 0.0))

MAX_TRIANGLES = 60000
F = m.FOUNDATION


def banner(x, y, z, height=0.3):
    """Flag pole with a controller-coloured banner (``Banner`` material)."""
    return [
        m.cylinder(0.006, height, (x, y, z + height / 2), "Wood", 4),
        m.box((0.005, 0.13, 0.08), (x, y + 0.065, z + height - 0.05), "Banner"),
    ]


def low_wall(points, height=0.07, thickness=0.04, mat="Stone"):
    """Closed precinct wall without crenellations."""
    parts = []
    for index, (x0, y0) in enumerate(points):
        x1, y1 = points[(index + 1) % len(points)]
        parts += m.wall_segment(x0, y0, x1, y1, height, thickness, mat, crenel=False)
    return parts


def lean_to(length, depth, height, location, angle, mat="Tile"):
    """Single-pitch gallery roof (cloister walk) rising towards local -Y."""
    hl, hd = length / 2, depth / 2
    verts = [
        (-hl, -hd, height),
        (hl, -hd, height),
        (hl, hd, 0.0),
        (-hl, hd, 0.0),
        (-hl, -hd, height - 0.02),
        (hl, -hd, height - 0.02),
    ]
    faces = [(0, 1, 2, 3), (4, 0, 3), (1, 5, 2), (0, 4, 5, 1)]
    return m.mesh_object("lean_to", verts, faces, mat, location, angle)


def field_strips(x, y, angle, count, length, width, mats=("Field", "Garden", "Earth")):
    """Flat cultivated strips (open-field furlong) slightly above the ground."""
    parts = []
    ca, sa = math.cos(angle), math.sin(angle)
    for index in range(count):
        off = (index - (count - 1) / 2) * width * 1.05
        parts.append(
            m.box(
                (length, width, 0.012 + F * 0.3),
                (x - off * sa, y + off * ca, 0.006 - F * 0.15),
                mats[index % len(mats)],
                rotation=(0, 0, angle),
            )
        )
    return parts


# --- Cities and towns ----------------------------------------------------------------


def build_city(seed, sides, radius, cathedral_angle, gates):
    """Walled episcopal city dominated by its cathedral, a keep on one side."""
    rng = random.Random(seed)
    points = m.ring_points(radius, sides, rng, 0.1)
    parts = m.ground_patch(radius * 0.96)
    parts += m.town_walls(points, 0.22, 0.075, gates=gates)
    parts += m.cathedral_building(0.0, 0.05, cathedral_angle, 0.55)
    keep_x, keep_y = radius * 0.55, -radius * 0.45
    parts += m.square_tower(keep_x, keep_y, 0.2, 0.55, "Slate")
    parts += banner(keep_x, keep_y, 0.55 + 0.2)
    parts += m.scatter_houses(
        rng,
        52,
        radius * 0.9,
        [(0.0, 0.05, 0.58), (keep_x, keep_y, 0.22)],
        1.3,
        inside=[(x * 0.92, y * 0.92) for x, y in points],
    )
    # Faubourg hors les murs devant une porte.
    gx, gy = points[gates[0]]
    for k in range(6):
        t = 1.12 + 0.12 * k
        ang = math.atan2(gy, gx) + rng.uniform(-0.25, 0.25)
        parts += m.house(
            (t * radius * math.cos(ang), t * radius * math.sin(ang), 0.0),
            (0.18, 0.11, 0.1),
            rng.choice(("Thatch", "TileOld")),
            rng.choice(("Plaster", "Timber")),
            angle=ang + math.pi / 2,
        )
    return parts


def build_city_a():
    """Large walled city, cathedral near the centre."""
    return build_city(4242, 11, 1.3, 0.25, (0, 4, 8))


def build_city_b():
    """Oval city along a river bend, cathedral set east-west, keep on the south side."""
    parts = build_city(911, 10, 1.2, -0.1, (1, 6))
    return parts


def build_town_a():
    """Market town: irregular curtain wall, parish church, market hall."""
    rng = random.Random(7)
    points = m.ring_points(0.95, 9, rng, 0.12)
    parts = m.ground_patch(0.9)
    parts += m.town_walls(points, 0.19, 0.065, gates=(0, 5))
    parts += m.church(0.1, 0.12, 0.8, 1.0)
    parts += m.house((-0.25, -0.15, 0.0), (0.26, 0.14, 0.07), "Tile", "Timber", 0.3)
    parts += banner(points[0][0], points[0][1], 0.19 * 1.45 + 0.1)
    parts += m.scatter_houses(
        rng,
        40,
        0.85,
        [(0.1, 0.12, 0.34), (-0.25, -0.15, 0.2)],
        1.3,
        inside=[(x * 0.9, y * 0.9) for x, y in points],
    )
    return parts


def build_town_b():
    """Bastide: square walls, grid streets, arcaded square with its church."""
    rng = random.Random(1283)
    half = 0.78
    points = [(-half, -half), (half, -half), (half, half), (-half, half)]
    parts = m.ground_patch(0.95)
    parts += m.town_walls(points, 0.17, 0.065, gates=(0, 1, 2, 3))
    parts += m.church(0.28, 0.28, 0.0, 0.9, roof="Tile")
    parts += banner(-half, -half, 0.17 * 1.45 + 0.1)
    step = 0.26
    for ix in range(-2, 3):
        for iy in range(-2, 3):
            if abs(ix) <= 0 and abs(iy) <= 0:
                continue  # place du marché
            cx, cy = ix * step, iy * step
            if math.hypot(cx - 0.28, cy - 0.28) < 0.3:
                continue
            for dx in (-0.055, 0.055):
                if rng.random() < 0.85:
                    parts += m.house(
                        (cx + dx, cy, 0.0),
                        (0.1, 0.17, rng.uniform(0.09, 0.13)),
                        rng.choice(("Tile", "TileOld")),
                        rng.choice(("Plaster", "Stone", "Timber")),
                        angle=math.pi / 2,
                    )
    return parts


# --- Castles -----------------------------------------------------------------------------


def motte(x, y, radius, height):
    """Earth mound (frustum) under a keep."""
    return [
        m.cone(
            radius,
            height + F,
            (x, y, (height - F) / 2),
            "Earth",
            12,
            radius_top=radius * 0.62,
        )
    ]


def build_castle_a():
    """Stone keep on a motte, bailey with curtain wall and towers, ditch."""
    parts = m.ground_patch(0.95, "Earth")
    parts += motte(0.2, 0.1, 0.42, 0.14)
    parts += m.castle_keep(0.22, 0.12, 0.85)
    rng = random.Random(51)
    bailey = m.ring_points(0.78, 7, rng, 0.08, phase=0.4)
    parts += m.town_walls(bailey, 0.16, 0.06, gates=(3,))
    parts += m.house((-0.4, -0.2, 0.0), (0.3, 0.12, 0.1), "Slate", "Stone", 0.6)
    parts += m.house((-0.3, 0.3, 0.0), (0.22, 0.12, 0.09), "Thatch", "Timber", -0.4)
    parts += m.house((-0.05, -0.5, 0.0), (0.24, 0.11, 0.08), "Thatch", "Wood", 1.2)
    return parts


def build_castle_b():
    """Rock-top castle: tall square donjon, shell wall, village at the foot of the hill."""
    parts = m.ground_patch(1.05, "Earth")
    parts += motte(0.0, 0.0, 0.55, 0.22)
    rng = random.Random(1360)
    shell = m.ring_points(0.42, 8, rng, 0.06)
    parts += [
        p
        for seg in range(len(shell))
        for p in m.wall_segment(
            shell[seg][0],
            shell[seg][1],
            shell[(seg + 1) % len(shell)][0],
            shell[(seg + 1) % len(shell)][1],
            0.36,
            0.06,
        )
    ]
    for x, y in shell[::2]:
        parts += m.round_tower(x, y, 0.065, 0.46, "Slate", 8)
    parts += m.square_tower(0.05, 0.02, 0.26, 0.95, "Slate")
    parts += banner(0.05, 0.02, 0.95 + 0.22)
    parts += m.house((-0.15, 0.18, 0.2), (0.24, 0.1, 0.1), "Slate", "Stone", 0.2)
    for k in range(9):
        ang = 3.6 + k * 0.26 + rng.uniform(-0.08, 0.08)
        r = 0.9 + rng.uniform(-0.08, 0.1)
        parts += m.house(
            (r * math.cos(ang), r * math.sin(ang), 0.0),
            (0.17, 0.11, 0.09),
            rng.choice(("Thatch", "TileOld", "Thatch")),
            rng.choice(("Plaster", "Timber")),
            angle=ang + math.pi / 2,
        )
    return parts


# --- Abbeys ------------------------------------------------------------------------------


def abbey_church(x, y, angle, s):
    """Romanesque-gothic abbey church: long nave, transept, crossing tower, apse."""
    ca, sa = math.cos(angle), math.sin(angle)

    def at(dx, dy):
        return x + dx * ca - dy * sa, y + dx * sa + dy * ca

    parts = []
    nx, ny = at(0.0, 0.0)
    parts.append(
        m.box(
            (1.0 * s, 0.26 * s, 0.34 * s + F),
            (nx, ny, (0.34 * s - F) / 2),
            "StoneLight",
            (0, 0, angle),
        )
    )
    parts.append(
        m.gable_roof(1.0 * s, 0.26 * s, 0.18 * s, (nx, ny, 0.34 * s), "Slate", angle)
    )
    tx, ty = at(0.22 * s, 0.0)
    parts.append(
        m.box(
            (0.24 * s, 0.7 * s, 0.3 * s + F),
            (tx, ty, (0.3 * s - F) / 2),
            "StoneLight",
            (0, 0, angle),
        )
    )
    parts.append(
        m.gable_roof(
            0.7 * s, 0.24 * s, 0.16 * s, (tx, ty, 0.3 * s), "Slate", angle + math.pi / 2
        )
    )
    parts += m.square_tower(
        tx, ty, 0.2 * s, 0.62 * s, "Slate", spire=0.3 * s, angle=angle
    )
    axp, ayp = at(0.56 * s, 0.0)
    parts.append(
        m.cylinder(
            0.13 * s, 0.28 * s + F, (axp, ayp, (0.28 * s - F) / 2), "StoneLight", 8
        )
    )
    parts.append(
        m.cone(0.15 * s, 0.14 * s, (axp, ayp, 0.28 * s + 0.07 * s), "Slate", 8)
    )
    return parts


def cloister(x, y, side, angle):
    """Square cloister: four roofed galleries around a garth with a well."""
    parts = []
    depth = 0.07
    for k in range(4):
        a = angle + k * math.pi / 2
        off = side / 2 - depth / 2
        dx, dy = math.cos(a), math.sin(a)
        gx, gy = x + dx * off, y + dy * off
        parts.append(
            m.box(
                (depth, side, 0.06 + F),
                (gx, gy, (0.06 - F) / 2),
                "StoneLight",
                (0, 0, a),
            )
        )
        parts.append(
            lean_to(side, depth, 0.05, (gx, gy, 0.06), a + math.pi / 2, "Tile")
        )
    parts.append(
        m.box(
            (side - 2 * depth, side - 2 * depth, 0.01 + F * 0.3),
            (x, y, 0.005 - F * 0.15),
            "Garden",
            (0, 0, angle),
        )
    )
    parts.append(m.cylinder(0.025, 0.04, (x, y, 0.02), "Stone", 6))
    return parts


def build_abbey(seed, s, with_mill):
    """Abbey precinct: church to the north of the cloister, ranges, gardens, wall."""
    rng = random.Random(seed)
    parts = m.ground_patch(0.95, "Dirt")
    parts += abbey_church(0.0, 0.32, 0.0, s)
    parts += cloister(0.02, -0.06, 0.46, 0.0)
    # Bâtiments conventuels : dortoir (est), réfectoire (sud), cellier (ouest).
    parts += m.house((0.33, -0.06, 0.0), (0.12, 0.46, 0.16), "Slate", "StoneLight", 0.0)
    parts += m.house((0.02, -0.36, 0.0), (0.46, 0.12, 0.14), "Tile", "StoneLight", 0.0)
    parts += m.house((-0.3, -0.06, 0.0), (0.12, 0.4, 0.12), "TileOld", "Stone", 0.0)
    parts += m.house((-0.5, 0.4, 0.0), (0.2, 0.12, 0.1), "Tile", "Plaster", 0.2)
    parts += banner(-0.45, 0.2, 0.0, 0.35)
    parts += field_strips(
        0.45, -0.62, 0.0, 5, 0.34, 0.06, ("Garden", "Field", "Garden")
    )
    parts += field_strips(-0.5, -0.55, 0.3, 4, 0.3, 0.06)
    precinct = [(-0.85, -0.85), (0.85, -0.85), (0.9, 0.8), (-0.8, 0.85)]
    parts += low_wall(precinct)
    if with_mill:
        parts += m.house((0.72, 0.55, 0.0), (0.14, 0.12, 0.12), "Thatch", "Timber", 0.5)
        parts.append(
            m.cylinder(
                0.07,
                0.015,
                (0.78, 0.47, 0.07),
                "Wood",
                8,
                rotation=(math.pi / 2, 0, 0.5),
            )
        )
    for _ in range(3):
        parts += m.house(
            (rng.uniform(0.95, 1.2), rng.uniform(-0.6, 0.3), 0.0),
            (0.16, 0.1, 0.08),
            "Thatch",
            rng.choice(("Timber", "Plaster")),
            angle=rng.uniform(0, math.pi),
        )
    return parts


def build_abbey_a():
    """Benedictine abbey with its precinct and gardens."""
    return build_abbey(1098, 1.0, False)


def build_abbey_b():
    """Cistercian abbey with a water mill and a lay-brothers' range."""
    return build_abbey(1115, 0.9, True)


# --- Villages and hamlets ------------------------------------------------------------


def build_village(seed, count, spread, church_angle):
    """Open village: parish church, cottages and barns, field strips around."""
    rng = random.Random(seed)
    parts = [
        m.box((1.9, 0.1, 0.01 + F), (0, 0, 0.005 - F / 2), "Dirt", rotation=(0, 0, 0.3))
    ]
    parts += m.church(0.05, 0.25, church_angle, 0.75, roof="TileOld")
    parts += m.scatter_houses(
        rng,
        count,
        spread,
        [(0.05, 0.25, 0.28)],
        1.35,
        roofs=("Thatch", "Thatch", "TileOld", "Tile"),
        walls=("Plaster", "Timber"),
        size_scale=1.05,
    )
    for dx, dy, angle in ((-0.72, -0.34, 0.3), (0.7, 0.42, 1.9)):
        parts += m.house(
            (dx, dy, 0.0), (0.28, 0.15, 0.11), "Thatch", "Wood", angle=angle
        )
    parts += field_strips(-0.2, -0.95, 0.3, 6, 0.6, 0.07)
    parts += field_strips(0.9, -0.2, 1.6, 5, 0.5, 0.07)
    parts += banner(0.05 - 0.19, 0.25, 0.42 * 0.75 + 0.3 * 0.75, 0.2)
    return parts


def build_village_a():
    """Street village of about fifteen cottages."""
    return build_village(99, 15, 0.85, 0.3)


def build_village_b():
    """Clustered village around the church and its green."""
    return build_village(314, 12, 0.7, -0.9)


def build_hamlet(seed, count):
    """Farmstead group: a few cottages and a barn around a yard."""
    rng = random.Random(seed)
    parts = [m.cylinder(0.26, 0.01 + F * 0.3, (0, 0, 0.005 - F * 0.15), "Dirt", 10)]
    for index in range(count):
        ang = index * 2 * math.pi / count + rng.uniform(-0.3, 0.3)
        r = rng.uniform(0.14, 0.22)
        barn = index == 0
        parts += m.house(
            (r * math.cos(ang), r * math.sin(ang), 0.0),
            (0.24, 0.13, 0.1)
            if barn
            else (
                rng.uniform(0.14, 0.19),
                rng.uniform(0.09, 0.12),
                rng.uniform(0.07, 0.09),
            ),
            "Thatch" if barn or rng.random() < 0.7 else "TileOld",
            "Wood" if barn else rng.choice(("Plaster", "Timber")),
            angle=ang + math.pi / 2 + rng.uniform(-0.2, 0.2),
        )
    return parts


MODELS = {
    "city_a": build_city_a,
    "city_b": build_city_b,
    "town_a": build_town_a,
    "town_b": build_town_b,
    "castle_a": build_castle_a,
    "castle_b": build_castle_b,
    "abbey_a": build_abbey_a,
    "abbey_b": build_abbey_b,
    "village_a": build_village_a,
    "village_b": build_village_b,
    "hamlet_a": lambda: build_hamlet(11, 3),
    "hamlet_b": lambda: build_hamlet(23, 4),
    "hamlet_c": lambda: build_hamlet(37, 5),
}


def export_model(name: str, out_dir: Path) -> int:
    """Build one model, join it into a single mesh, export ``<name>.glb``; return triangles."""
    m.reset_scene()
    parts = MODELS[name]()
    if m.KIT:
        import kit_campaign

        kit_campaign.finish_parts(parts)
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    bpy.ops.object.join()
    obj = bpy.context.active_object
    obj.name = name
    if m.KIT:
        import kit_campaign

        kit_campaign.atlas(obj)
    bpy.ops.object.shade_flat()
    triangles = m.triangle_count(obj)
    if triangles >= MAX_TRIANGLES:
        raise RuntimeError(f"{name}: {triangles} triangles >= {MAX_TRIANGLES}")
    out_dir.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(out_dir / f"{name}.glb"),
        export_format="GLB",
        export_vertex_color="ACTIVE",
        export_yup=True,
        export_apply=True,
        use_selection=False,
    )
    return triangles


def main() -> None:
    """Parse ``-- <out_dir> [names...]`` and export the requested models."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(args[0]) if args else Path.cwd() / "settlements"
    names = args[1:] or list(MODELS)
    for name in names:
        print(f"MODEL {name} {export_model(name, out_dir)}")
    print("OK")


if __name__ == "__main__":
    main()
