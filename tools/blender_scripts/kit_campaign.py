"""Campaign-map bridge of the building kit (lot BR1, ADR 0021).

``models.py`` and ``settlements.py`` build their settlement models in *model units* (a house
is about 0.2 unit long). With ``models.KIT`` on, their ``house()`` and ``church()`` delegate
here: the building is made by ``building_kit`` at the ``low`` level of detail in real metres,
then scaled down by :data:`METERS_PER_UNIT`. :func:`finish_parts` then gives the remaining
legacy parts (walls, towers, keeps, cathedral) metric UVs, vertex colours with baked
occlusion, and the kit's material names, so that Godot textures everything the same way
(``building_materials.gd``).
"""

import math
import random

import building_kit as kit
import kit_export
import kit_geometry as kg
import models as m

# One model unit ~ 60 m: a 0.2-unit house is a 12 m house, a 0.22-unit wall is 13 m high.
METERS_PER_UNIT = 60.0

ROOFS = {
    "Thatch": "Thatch",
    "Tile": "RoofTile",
    "TileOld": "RoofFlat",
    "Slate": "RoofSlate",
}
# Legacy palette material -> (kit material, vertex tint)
ALIAS = {
    "Stone": ("Masonry", (1.0, 0.98, 0.94)),
    "StoneLight": ("Masonry", (1.12, 1.1, 1.04)),
    "DarkStone": ("Rubble", (0.62, 0.6, 0.58)),
    "Slate": ("RoofSlate", (1.0, 1.0, 1.0)),
    "Tile": ("RoofTile", (1.0, 1.0, 1.0)),
    "TileOld": ("RoofFlat", (1.0, 1.0, 1.0)),
    "Wood": ("Planks", (0.8, 0.78, 0.75)),
    "Plaster": ("Plaster", (0.95, 0.9, 0.8)),
    "Timber": ("Timber", (1.0, 1.0, 1.0)),
    "Thatch": ("Thatch", (1.0, 1.0, 1.0)),
}


LAYERS = kit_export.ATLAS_LAYERS


def atlas(obj) -> None:
    """Merge the kit materials of a joined campaign model into one ``Building`` material."""
    kit_export.atlas(obj)


def _seed(x: float, y: float, salt: int = 0) -> int:
    return int((x * 7919.0 + y * 104729.0) * 1000.0) ^ salt


def _place(g: kg.Geometry, name: str, location, angle: float):
    obj = kit_export.to_object(g, name)
    s = 1.0 / METERS_PER_UNIT
    obj.scale = (s, s, s)
    obj.rotation_euler = (0.0, 0.0, angle)
    obj.location = location
    return obj


def house(location, size, roof_mat, wall_mat, angle=0.0, hip=False):
    """Kit replacement for ``models.house``: same footprint, real storeys and roof."""
    x, y, z = location
    w, d, h = size
    rng = random.Random(_seed(x, y, 17))
    length, depth, height = (
        w * METERS_PER_UNIT,
        d * METERS_PER_UNIT,
        h * METERS_PER_UNIT,
    )
    floors = 1 if height < 5.5 else (2 if height < 8.0 else 3)
    roof = ROOFS.get(roof_mat, "RoofTile")
    wall = {"Stone": "Rubble", "StoneLight": "Ashlar", "Wood": "Planks"}.get(
        wall_mat, "Plaster"
    )
    framed = wall_mat == "Timber" or (wall_mat == "Plaster" and rng.random() < 0.45)
    style = kit.Style(
        wall=wall,
        frame=("rural" if floors == 1 else rng.choice(["cross", "close", "rural"]))
        if framed
        else None,
        roof=roof,
        shape="hip"
        if hip
        else ("half_hip" if roof == "Thatch" and rng.random() < 0.4 else "gable"),
        pitch=rng.uniform(50, 58) if roof != "RoofTile" else rng.uniform(44, 52),
        floors=floors,
        jetty=0.45 if framed and floors > 1 else 0.0,
        ground_stone=framed and floors > 1 and rng.random() < 0.4,
        chimneys=0 if wall == "Planks" else 1,
        wall_tint=kit.pick(
            rng,
            kit.PLASTER_TINTS if floors > 1 else kit.DAUB_TINTS + kit.PLASTER_TINTS[:2],
        ),
        timber_tint=kit.pick(rng, kit.TIMBER_TINTS),
        roof_tint=kit.pick(rng, kit.ROOF_TINTS[roof]),
        stone_tint=kit.pick(rng, kit.STONE_TINTS),
    )
    g = kg.Geometry()
    g.uv_shift = (rng.uniform(0, 4), rng.uniform(0, 4))
    g.seed_shift = rng.uniform(0, 1000)
    info = kit.house(g, rng, length, depth, style, "low")
    if "eave" in info:
        kit._settle(g, info, 0.12 * rng.uniform(0.5, 1.2), rng)
    parts = [_place(g, "kit_house", location, angle)]
    # Deep plinth (the kit's own foundations stop 2 m below ground): the house sits on slopes.
    parts.append(
        m.box(
            (w, d, m.FOUNDATION),
            (x, y, z - m.FOUNDATION / 2 - 0.02),
            "DarkStone",
            rotation=(0, 0, angle),
        )
    )
    return parts


def church(x, y, angle, scale=1.0):
    """Kit replacement for ``models.church``: nave, apse, buttresses, west tower and spire."""
    g = kg.Geometry()
    rng = random.Random(_seed(x, y, 71))
    g.seed_shift = rng.uniform(0, 1000)
    length, depth = 0.66 * scale * METERS_PER_UNIT, 0.2 * scale * METERS_PER_UNIT
    kit.church(g, rng, "low", length=length, depth=depth)
    parts = [_place(g, "kit_church", (x, y, 0.0), angle)]
    parts.append(
        m.box(
            (0.66 * scale, 0.2 * scale, m.FOUNDATION),
            (x, y, -m.FOUNDATION / 2 - 0.02),
            "DarkStone",
            rotation=(0, 0, angle),
        )
    )
    return parts


def cathedral(x, y, angle, s=1.0):
    """Kit replacement for ``models.cathedral_building`` (same footprint, gothic cathedral)."""
    g = kg.Geometry()
    rng = random.Random(_seed(x, y, 113))
    g.seed_shift = rng.uniform(0, 1000)
    length, depth = 1.75 * s * METERS_PER_UNIT, 0.52 * s * METERS_PER_UNIT
    kit.cathedral(g, rng, "low", length=length, depth=depth)
    parts = [_place(g, "kit_cathedral", (x, y, 0.0), angle)]
    parts.append(
        m.box(
            (1.75 * s, 0.6 * s, m.FOUNDATION),
            (x, y, -m.FOUNDATION / 2 - 0.02),
            "DarkStone",
            rotation=(0, 0, angle),
        )
    )
    return parts


def finish_parts(parts) -> None:
    """Metric UVs, vertex colours and kit material names on the legacy (non-kit) parts."""
    for obj in parts:
        mesh = obj.data
        if mesh is None or "Color" in mesh.color_attributes:
            continue
        mw = obj.matrix_world
        rot = mw.to_3x3()
        uv = mesh.uv_layers.get("UVMap") or mesh.uv_layers.new(name="UVMap")
        colors = mesh.color_attributes.new(
            name="Color", type="FLOAT_COLOR", domain="CORNER"
        )
        mesh.color_attributes.active_color = colors
        tints = []
        for slot in obj.material_slots:
            name = slot.material.name.split(".")[0] if slot.material else ""
            tints.append(ALIAS.get(name, (None, (1.0, 1.0, 1.0)))[1])
        for poly in mesh.polygons:
            n = (rot @ poly.normal).normalized()
            normal = (n.x, n.y, n.z)
            tangent, bitangent = kg.tangent_frame(normal)
            tint = (
                tints[poly.material_index]
                if poly.material_index < len(tints)
                else (1.0, 1.0, 1.0)
            )
            for li in poly.loop_indices:
                co = mw @ mesh.vertices[mesh.loops[li].vertex_index].co
                p = (
                    co.x * METERS_PER_UNIT,
                    co.y * METERS_PER_UNIT,
                    co.z * METERS_PER_UNIT,
                )
                uv.data[li].uv = (kg.dot(p, tangent), kg.dot(p, bitangent))
                ao = (
                    0.6 + 0.4 * kg.smoothstep(-0.2, 1.6, p[2])
                    if abs(normal[2]) < 0.5
                    else 1.0
                )
                colors.data[li].color = (tint[0] * ao, tint[1] * ao, tint[2] * ao, 1.0)
        for slot in obj.material_slots:
            if slot.material is None:
                continue
            alias = ALIAS.get(slot.material.name.split(".")[0])
            if alias is not None:
                slot.material = kit_export.material(alias[0], False)


def row_house(
    location, frontage, depth, height, roof_mat, wall_mat, angle, gable_front
):
    """Town house of a street row: ``frontage`` along local X, street on local -Y."""
    x, y, z = location
    rng = random.Random(_seed(x, y, 29))
    length, dep = frontage * METERS_PER_UNIT, depth * METERS_PER_UNIT
    floors = 2 if height * METERS_PER_UNIT < 8.0 else 3
    roof = ROOFS.get(roof_mat, "RoofTile")
    stone = wall_mat in ("Stone", "StoneLight")
    style = kit.Style(
        wall="Ashlar" if stone else "Plaster",
        frame=None if stone else rng.choice(["close", "cross", "close", "rural"]),
        roof=roof,
        shape="gable",
        pitch=rng.uniform(52, 60),
        floors=floors,
        jetty=0.0 if stone else rng.uniform(0.3, 0.5),
        ground_stone=not stone and rng.random() < 0.5,
        gable_front=gable_front,
        shop=rng.random() < 0.5,
        window="mullion" if stone else "wide",
        chimneys=1,
        wall_tint=kit.pick(rng, kit.PLASTER_TINTS),
        timber_tint=kit.pick(rng, kit.TIMBER_TINTS),
        roof_tint=kit.pick(rng, kit.ROOF_TINTS[roof]),
        stone_tint=kit.pick(rng, kit.STONE_TINTS),
    )
    g = kg.Geometry()
    g.uv_shift = (rng.uniform(0, 4), rng.uniform(0, 4))
    g.seed_shift = rng.uniform(0, 1000)
    info = kit.house(g, rng, length, dep, style, "low")
    if "eave" in info:
        kit._settle(g, info, 0.08 * rng.uniform(0.5, 1.2), rng)
    parts = [_place(g, "kit_row", location, angle)]
    parts.append(
        m.box(
            (frontage, depth, m.FOUNDATION),
            (x, y, z - m.FOUNDATION / 2 - 0.02),
            "DarkStone",
            rotation=(0, 0, angle),
        )
    )
    return parts


def _overlaps(a, b) -> bool:
    """Separating-axis test between two oriented rectangles (cx, cy, ux, uy, hw, hd)."""
    for ux, uy in ((a[2], a[3]), (-a[3], a[2]), (b[2], b[3]), (-b[3], b[2])):

        def extent(r, ux=ux, uy=uy):
            c = r[0] * ux + r[1] * uy
            e = abs(r[2] * ux + r[3] * uy) * r[4] + abs(-r[3] * ux + r[2] * uy) * r[5]
            return c - e, c + e

        a0, a1 = extent(a)
        b0, b1 = extent(b)
        if a1 <= b0 or b1 <= a0:
            return False
    return True


def street_houses(rng, count, radius, keep_out, inside, roofs, walls, size_scale=1.0):
    """Walled-town layout: radial and ring streets lined with continuous rows of houses.

    Houses face their street (gable-front narrow town houses, some eave-front), plots are
    ``depth`` deep behind the street front; ``keep_out`` circles (cathedral, keep, square) and
    the wall polygon ``inside`` are respected. Returns Blender parts like ``scatter_houses``.
    """
    street_half = 0.03 * size_scale
    placed = []
    parts = []
    center_keep = [(kx, ky, kr) for kx, ky, kr in keep_out]

    def ok(rect):
        cx, cy = rect[0], rect[1]
        ux, uy, hw, hd = rect[2], rect[3], rect[4], rect[5]
        corners = [
            (cx + sx * ux * hw - sy * uy * hd, cy + sx * uy * hw + sy * ux * hd)
            for sx in (-1, 1)
            for sy in (-1, 1)
        ]
        if any(not m.point_in_polygon(px, py, inside) for px, py in corners):
            return False
        if any(
            math.hypot(px - kx, py - ky) < kr
            for px, py in corners + [(cx, cy)]
            for kx, ky, kr in center_keep
        ):
            return False
        shrunk = (cx, cy, ux, uy, hw * 0.97, hd * 0.97)
        return not any(_overlaps(shrunk, other) for other in placed)

    def line_street(x0, y0, x1, y1):
        seg = math.hypot(x1 - x0, y1 - y0)
        if seg < 1e-6:
            return
        ux, uy = (x1 - x0) / seg, (y1 - y0) / seg
        for side in (1, -1):
            nx, ny = -uy * side, ux * side
            t = rng.uniform(0.0, 0.04)
            while t < seg and len(placed) < count:
                gable = rng.random() < 0.65
                front = (
                    rng.uniform(0.085, 0.12) * size_scale
                    if gable
                    else rng.uniform(0.14, 0.2) * size_scale
                )
                depth = (
                    rng.uniform(0.15, 0.2) * size_scale
                    if gable
                    else rng.uniform(0.1, 0.13) * size_scale
                )
                if t + front > seg:
                    break
                sx, sy = x0 + ux * (t + front / 2), y0 + uy * (t + front / 2)
                cx, cy = (
                    sx + nx * (street_half + depth / 2),
                    sy + ny * (street_half + depth / 2),
                )
                # Local frame: +X along the street, -Y towards it -> local +Y = (nx, ny).
                ax, ay = ny, -nx  # local X such that rotate(X, 90 deg) = (nx, ny)
                rect = (cx, cy, ax, ay, front / 2, depth / 2)
                if ok(rect):
                    placed.append(rect)
                    angle = math.atan2(ay, ax)
                    h = rng.uniform(0.1, 0.15) * size_scale
                    parts.extend(
                        row_house(
                            (cx, cy, 0.0),
                            front,
                            depth,
                            h,
                            rng.choice(roofs),
                            rng.choice(walls),
                            angle,
                            gable,
                        )
                    )
                    t += front + rng.uniform(0.0, 0.006)
                else:
                    t += 0.03 * size_scale

    streets = rng.randint(5, 7)
    base = rng.uniform(0, 2 * math.pi)
    for k in range(streets):
        a = base + k * 2 * math.pi / streets + rng.uniform(-0.2, 0.2)
        line_street(
            math.cos(a) * radius * 0.16,
            math.sin(a) * radius * 0.16,
            math.cos(a) * radius * 1.05,
            math.sin(a) * radius * 1.05,
        )
    for frac in (0.42, 0.7, 0.93):
        sides = 14
        ring = [
            (
                math.cos(base + 2 * math.pi * i / sides) * radius * frac,
                math.sin(base + 2 * math.pi * i / sides) * radius * frac,
            )
            for i in range(sides)
        ]
        for i in range(sides):
            (x0, y0), (x1, y1) = ring[i], ring[(i + 1) % sides]
            line_street(x0, y0, x1, y1)
    return parts
