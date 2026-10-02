"""Western settlement family of the campaign map (lot GC3c, ADR 0158).

``west``: France, England, the Empire, the Low Countries, Scandinavia, Poland, Hungary. Same
maquette style and helpers as ``settlements_east.py`` (few but large elements, light flat
colours read from far away), registered next to the five other families as
``<kind>_west_<a|b>``. It replaces, on the campaign map, the dark and heavy ``city_a``...
models of ``settlements.py`` (BR1 building kit), which stay in use in battles.

Signature: pale limestone walls with round towers under blue slate cones, steep gabled houses
with red-orange tile roofs (a few slate ones), gothic churches with spires, blond thatch in
the countryside.
"""

import math
import random

import models as m
import settlements_east as east
from settlements_east import (
    TAU,
    F,
    block,
    cloister,
    curtain,
    flag,
    gable_house,
    gatehouse,
    local_frame,
    mound,
    plots,
    round_tower,
    scatter,
    shrink,
    suburb,
    tower,
)

FAMILY = "west"

# Same rule as ``settlements_east.PALETTE``: names outside the kit atlas layers.
PALETTE = {
    "PaleStone": ((0.74, 0.68, 0.55), 0.88, 0.0),
    "Cream": ((0.84, 0.74, 0.54), 0.92, 0.0),
    "TileRed": ((0.68, 0.19, 0.07), 0.8, 0.0),
    "SlateBlue": ((0.20, 0.29, 0.42), 0.6, 0.0),
    "Straw": ((0.74, 0.58, 0.26), 0.97, 0.0),
    "Turf": ((0.24, 0.36, 0.12), 1.0, 0.0),
    "Street": ((0.48, 0.42, 0.30), 1.0, 0.0),
    "Orchard": ((0.12, 0.30, 0.08), 0.95, 0.0),
    "Pond": ((0.16, 0.38, 0.55), 0.7, 0.0),
}
for _name, _spec in PALETTE.items():
    m.PALETTE.setdefault(_name, _spec)

STONE = "PaleStone"
SLATE = "SlateBlue"
TILE = "TileRed"
HOUSE_WALLS = ("Cream", "Cream", STONE, "Whitewash")
FIELDS = ("Turf", "Straw", "Field")


# --- Buildings -----------------------------------------------------------------------


def town_house(x, y, w, d, angle, rng, z=0.0):
    """Gabled town house: rendered or stone walls, steep tile roof (one in five in slate)."""
    roof = SLATE if rng.random() < 0.2 else TILE
    wall = rng.choice(HOUSE_WALLS)
    return gable_house(x, y, w, d, rng.uniform(0.12, 0.18), angle, wall, roof, 0.7, z)


def cottage(x, y, w, d, angle, rng, z=0.0):
    """Low rendered cottage under blond thatch (one in three under tile)."""
    roof = TILE if rng.random() < 0.33 else "Straw"
    wall = rng.choice(("Cream", "Whitewash"))
    return gable_house(x, y, w, d, rng.uniform(0.08, 0.11), angle, wall, roof, 0.8, z)


def barn(x, y, angle, s=1.0):
    """Timber barn under a large thatch roof."""
    return gable_house(x, y, 0.34 * s, 0.19 * s, 0.1 * s, angle, "Log", "Straw", 0.85)


def market_hall(x, y, angle, s=1.0):
    """Open market hall: wide tile roof on low timber posts."""
    return gable_house(x, y, 0.42 * s, 0.22 * s, 0.07 * s, angle, "Log", TILE, 0.6)


def pepper_pot(x, y, radius, height, z=0.0):
    """Round stone tower under a blue slate cone."""
    return round_tower(x, y, radius, height, STONE, SLATE, z)


def belfry(x, y, s=1.0):
    """Town belfry: tall square shaft, jutting gallery, bell stage under a slate spire."""
    parts = tower(x, y, 0.17 * s, 0.7 * s, STONE, "cap")
    return parts + tower(
        x, y, 0.13 * s, 0.14 * s, STONE, "tent", SLATE, z=0.7 * s + 0.045
    )


def belfry_top(s=1.0):
    """Height of the spire apex of :func:`belfry`."""
    return 0.7 * s + 0.045 + 0.14 * s + 0.13 * s * 1.35


def parish_church(x, y, angle, s=1.0, roof=SLATE, z=0.0):
    """Parish church (west tower at local -X): nave, lower choir, bell tower under a spire."""
    at = local_frame(x, y, angle, s)
    parts = gable_house(x, y, 0.5 * s, 0.2 * s, 0.2 * s, angle, STONE, roof, 0.75, z)
    parts += gable_house(
        *at(0.33, 0.0), 0.2 * s, 0.15 * s, 0.15 * s, angle, STONE, roof, 0.75, z
    )
    return parts + tower(
        *at(-0.32, 0.0), 0.17 * s, 0.42 * s, STONE, "tent", roof, angle, z
    )


def great_church(x, y, angle, s, nave, crossing):
    """Nave (ridge along local X), transept at ``crossing`` and round apse of a large church.

    ``nave`` is ``(length, width, wall height, roof material)``; returns the parts and the
    ridge height.
    """
    length, width, height, roof = nave
    at = local_frame(x, y, angle, s)
    pitch = 0.75
    parts = gable_house(
        x, y, length * s, width * s, height * s, angle, STONE, roof, pitch
    )
    parts += gable_house(
        *at(crossing, 0.0),
        width * 2.9 * s,
        width * 0.92 * s,
        height * s,
        angle + math.pi / 2,
        STONE,
        roof,
        pitch,
    )
    ax, ay = at(length / 2, 0.0)
    radius = width * 0.5 * s
    parts.append(
        m.cylinder(radius, height * s + F, (ax, ay, (height * s - F) / 2), STONE, 8)
    )
    rise = radius * 1.3
    parts.append(m.cone(radius * 1.12, rise, (ax, ay, height * s + rise / 2), roof, 8))
    return parts, (height + width * pitch) * s


def cathedral(x, y, angle, s=1.0, spire=False):
    """Gothic cathedral (west front at local -X).

    Tall nave between aisles and buttress pinnacles, transept, chevet with its ambulatory;
    two front towers, or (``spire``) a crossing spire between front turrets.
    """
    at = local_frame(x, y, angle, s)
    parts, ridge = great_church(x, y, angle, s, (1.0, 0.24, 0.46, SLATE), 0.2)
    for side in (-1, 1):
        parts += gable_house(
            *at(-0.06, side * 0.19),
            0.86 * s,
            0.16 * s,
            0.22 * s,
            angle,
            STONE,
            SLATE,
            0.3,
        )
        for dx in (-0.4, -0.24, -0.08):
            bx, by = at(dx, side * 0.29)
            parts.append(block(bx, by, 0.04 * s, 0.05 * s, 0.36 * s, STONE, angle))
    # Ambulatory around the apse.
    ax, ay = at(0.5, 0.0)
    parts.append(
        m.cylinder(0.2 * s, 0.2 * s + F, (ax, ay, (0.2 * s - F) / 2), STONE, 8)
    )
    parts.append(
        m.cone(0.215 * s, 0.07 * s, (ax, ay, 0.235 * s), SLATE, 8, radius_top=0.11 * s)
    )
    if spire:
        cx, cy = at(0.2, 0.0)
        parts.append(m.cone(0.065 * s, 0.62 * s, (cx, cy, ridge + 0.29 * s), SLATE, 8))
        for side in (-1, 1):
            parts += tower(
                *at(-0.5, side * 0.15), 0.1 * s, 0.6 * s, STONE, "tent", SLATE, angle
            )
    else:
        for side in (-1, 1):
            parts += tower(
                *at(-0.52, side * 0.17), 0.2 * s, 0.84 * s, STONE, "crenel", angle=angle
            )
    return parts


def abbey_church(x, y, angle, s=1.0, roof=SLATE, crossing_tower=True):
    """Abbey church: long nave, transept, apse, crossing tower (or a slender bell spire)."""
    at = local_frame(x, y, angle, s)
    parts, ridge = great_church(x, y, angle, s, (1.0, 0.24, 0.32, roof), 0.22)
    cx, cy = at(0.22, 0.0)
    if crossing_tower:
        parts += tower(cx, cy, 0.22 * s, 0.6 * s, STONE, "tent", roof, angle)
    else:
        parts.append(m.cone(0.045 * s, 0.3 * s, (cx, cy, ridge + 0.13 * s), SLATE, 6))
    return parts


def orchard(x, y, rows, columns, step=0.14, radius=0.05):
    """Fruit trees on a grid: small round crowns."""
    return [
        m.sphere(
            radius,
            (x + (c - (columns - 1) / 2) * step, y + (r - (rows - 1) / 2) * step, 0.06),
            "Orchard",
        )
        for r in range(rows)
        for c in range(columns)
    ]


def flat_patch(x, y, radius, mat, sides=10):
    """Flat disc slightly above the ground (yard, green, pond)."""
    return m.cylinder(radius, 0.012 + F * 0.3, (x, y, 0.006 - F * 0.15), mat, sides)


# --- Walls ---------------------------------------------------------------------------


def twin_gate(x, y, angle, height, z=0.0, radius=0.07):
    """Gate between two round towers under slate cones."""
    at = local_frame(x, y, angle)
    parts = gatehouse(x, y, angle, height * 0.85, STONE, width=0.2, z=z)
    for side in (-1, 1):
        parts += pepper_pot(*at(side * 0.14, 0.0), radius, height, z)
    return parts


def west_walls(points, height, gates, z=0.0, radius=0.075, top=None):
    """Pale stone curtain wall with round towers under slate cones and twin-towered gates."""
    return curtain(
        points,
        height,
        STONE,
        tower_at=lambda x, y: pepper_pot(x, y, radius, height * 1.45, z),
        gates=gates,
        gate_at=lambda x, y, a: twin_gate(x, y, a, height * 1.55, z, radius * 0.95),
        top=top,
        z=z,
    )


def side_middle(points, index):
    """Middle of side ``index`` of a wall polygon (where :func:`curtain` puts its gate)."""
    x0, y0 = points[index]
    x1, y1 = points[(index + 1) % len(points)]
    return (x0 + x1) / 2, (y0 + y1) / 2


# --- Models --------------------------------------------------------------------------


def build_west_city(variant):
    """Episcopal city: stone enceinte, gothic cathedral, belfry or hall, gabled houses."""
    first = variant == "a"
    rng = random.Random(1163 if first else 1220)
    radius = 1.42 if first else 1.38
    points = m.ring_points(radius, 11 if first else 10, rng, 0.08)
    gates = (0, 4, 8) if first else (1, 6)
    parts = m.ground_patch(radius * 0.97, "Street")
    parts += west_walls(points, 0.2, gates)
    cath = (-0.08, 0.14, 0.25) if first else (0.08, 0.02, -0.4)
    parts += cathedral(*cath, 0.95, spire=not first)
    at = local_frame(*cath, 0.95)
    keep_out = [(*at(-0.3, 0.0), 0.36), (*at(0.26, 0.0), 0.42)]
    hx, hy = (0.42, -0.62) if first else (-0.5, 0.62)
    parts += market_hall(hx, hy, 0.2)
    keep_out.append((hx, hy, 0.24))
    if first:
        bx, by = hx + 0.34, hy + 0.12
        parts += belfry(bx, by)
        parts += flag(bx, by, belfry_top() - 0.06, 0.3)
    else:
        # Royal keep against the wall.
        bx, by = points[4][0] * 0.78, points[4][1] * 0.78
        parts += tower(bx, by, 0.26, 0.6, STONE, "crenel", angle=0.3)
        parts += flag(bx, by, 0.6 + 0.09)
    keep_out.append((bx, by, 0.18))
    parts += scatter(
        rng,
        120,
        radius * 0.95,
        town_house,
        keep_out,
        shrink(points, 0.9),
        (0.17, 0.25),
        0.98,
    )
    return parts + suburb(points, gates[0], rng, cottage)


def build_west_town(variant):
    """Market town inside a curtain wall (a) or square bastide around its market place (b)."""
    first = variant == "a"
    rng = random.Random(1247 if first else 1283)
    if first:
        points = m.ring_points(0.9, 9, rng, 0.1)
        gates = (0, 5)
        parts = m.ground_patch(0.88, "Street")
        parts += west_walls(points, 0.17, gates, radius=0.065)
        parts += parish_church(0.08, 0.14, 0.7, 1.15)
        parts += market_hall(-0.3, -0.22, 0.3, 0.85)
        gx, gy = side_middle(points, gates[0])
        parts += flag(gx, gy, 0.17 * 1.55 * 0.85 + 0.04, 0.26)
        keep_out = [(0.08, 0.14, 0.38), (-0.3, -0.22, 0.2)]
        return parts + scatter(
            rng, 30, 0.82, town_house, keep_out, shrink(points, 0.88), (0.18, 0.26)
        )
    half = 0.86
    points = [(-half, -half), (half, -half), (half, half), (-half, half)]
    parts = [block(0.0, 0.0, 2 * half, 2 * half, 0.02, "Street")]
    parts += curtain(
        points,
        0.17,
        STONE,
        tower_at=lambda x, y: pepper_pot(x, y, 0.08, 0.3),
        gates=(0, 1, 2, 3),
        gate_at=lambda x, y, a: gatehouse(x, y, a, 0.3, STONE, TILE),
    )
    parts += market_hall(0.0, 0.0, 0.0, 0.8)
    parts += flag(0.0, -0.17, 0.0, 0.42)
    step = 0.3
    church_cell = (1, 1)
    parts += parish_church(step, step + 0.02, 0.0, 0.72, TILE)
    for ix in range(-2, 3):
        for iy in range(-2, 3):
            if (ix, iy) in ((0, 0), church_cell):
                continue  # market place, church
            for dx in (-0.062, 0.062):
                if rng.random() < 0.9:
                    parts += town_house(
                        ix * step + dx, iy * step, 0.21, 0.115, math.pi / 2, rng
                    )
    return parts


def build_west_castle(variant):
    """Keep on a motte with its bailey (a) or stone castle with pepper-pot towers (b)."""
    first = variant == "a"
    rng = random.Random(1066 if first else 1360)
    if first:
        mx, my, height = 0.42, 0.18, 0.26
        bx, by = -0.3, -0.08
        parts = [flat_patch(bx, by, 0.56, "Street", 12)]
        bailey = [
            (bx + px, by + py) for px, py in m.ring_points(0.6, 8, rng, 0.06, phase=0.3)
        ]
        parts += curtain(
            bailey,
            0.14,
            "Log",
            gates=(4,),
            gate_at=lambda x, y, a: gatehouse(x, y, a, 0.26, "Log", "Straw"),
            thickness=0.045,
            top="stakes",
        )
        parts += mound(mx, my, 0.5, height, "Turf", 0.56)
        fence = [(mx + px, my + py) for px, py in m.ring_points(0.24, 7, rng, 0.03)]
        parts += curtain(fence, 0.08, "Log", thickness=0.03, z=height)
        parts += tower(mx, my, 0.24, 0.58, STONE, "crenel", angle=0.2, z=height)
        parts += flag(mx, my, height + 0.58 + 0.09)
        parts += gable_house(
            bx - 0.1, by + 0.2, 0.36, 0.17, 0.14, 0.5, "Cream", TILE, 0.75
        )
        parts += barn(bx - 0.18, by - 0.22, -0.3, 0.85)
        parts += cottage(bx + 0.18, by - 0.3, 0.2, 0.14, 1.2, rng)
        parts += cottage(bx - 0.36, by - 0.02, 0.18, 0.13, 1.5, rng)
        return parts
    height = 0.14
    parts = mound(0.0, 0.0, 0.82, height, "Turf", 0.8, 14)
    parts.append(flat_patch(0.0, 0.0, 0.5, "Street", 12))
    parts[-1].location.z += height
    shell = m.ring_points(0.5, 6, rng, 0.05, phase=0.5)
    parts += curtain(
        shell,
        0.26,
        STONE,
        tower_at=lambda x, y: pepper_pot(x, y, 0.1, 0.46, height),
        gates=(4,),
        gate_at=lambda x, y, a: twin_gate(x, y, a, 0.42, height, 0.075),
        top="merlons",
        z=height,
    )
    kx, ky = 0.12, 0.1
    parts += pepper_pot(kx, ky, 0.15, 0.78, height)
    parts += flag(kx, ky, height + 0.78 + 0.28, 0.26)
    parts += gable_house(-0.16, -0.1, 0.4, 0.16, 0.24, 0.5, STONE, SLATE, 0.75, height)
    for k in range(5):
        ang = 3.5 + k * 0.36 + rng.uniform(-0.06, 0.06)
        r = 0.92 + rng.uniform(-0.03, 0.05)
        parts += cottage(
            r * math.cos(ang), r * math.sin(ang), 0.2, 0.14, ang + math.pi / 2, rng
        )
    return parts


def build_west_abbey(variant):
    """Abbey: church, cloister, monastic ranges, precinct wall, gardens and orchard."""
    first = variant == "a"
    rng = random.Random(1098 if first else 1115)
    flip = 1 if first else -1
    roof = SLATE if first else TILE
    half = 0.84
    parts = [block(0.0, 0.0, 2 * half, 2 * half, 0.02, "Turf")]
    parts += abbey_church(-0.02, 0.34 * flip, 0.0, 1.0, roof, crossing_tower=first)
    parts += cloister(0.0, -0.08 * flip, 0.48, STONE, TILE, "Turf")
    parts += gable_house(0.35, -0.1 * flip, 0.15, 0.5, 0.18, 0.0, STONE, roof, 0.7)
    parts += gable_house(0.0, -0.42 * flip, 0.56, 0.15, 0.16, 0.0, "Cream", TILE, 0.7)
    parts += gable_house(-0.35, -0.1 * flip, 0.15, 0.44, 0.14, 0.0, STONE, TILE, 0.7)
    precinct = [(-half, -half), (half, -half), (half, half), (-half, half)]
    parts += curtain(
        precinct,
        0.08,
        STONE,
        gates=(3,),
        gate_at=lambda x, y, a: gatehouse(x, y, a, 0.2, STONE, TILE, 0.2),
        thickness=0.045,
    )
    parts += flag(-0.62, 0.05 * flip, 0.0, 0.42)
    parts += plots(0.56, -0.64 * flip, 0.0, 3, 0.4, 0.08, ("Field", "Straw", "Field"))
    parts += orchard(-0.56, -0.62 * flip, 2, 3)
    if not first:
        # Cistercian house: mill and fishpond.
        parts.append(flat_patch(0.6, 0.2, 0.15, "Pond"))
        parts += gable_house(0.6, 0.42, 0.2, 0.14, 0.12, 0.2, "Cream", "Straw", 0.75)
    for k in range(3):
        parts += cottage(
            -half - 0.17, (-0.45 + 0.34 * k) * flip, 0.2, 0.14, 1.5 + 0.1 * k, rng
        )
    return parts


def build_west_village(variant):
    """Open village around its church: thatched cottages, barns, field strips."""
    first = variant == "a"
    rng = random.Random(99 if first else 314)
    road = 0.3 if first else -0.2
    parts = [
        m.box(
            (1.9, 0.1, 0.01 + F), (0, 0, 0.005 - F / 2), "Street", rotation=(0, 0, road)
        )
    ]
    church = (0.05, 0.26)
    parts += parish_church(*church, road + (0.0 if first else 0.5), 0.85, TILE)
    parts += flag(church[0] - 0.08, church[1] - 0.2, 0.0, 0.36)
    keep_out = [(*church, 0.34)]
    if not first:
        # Village green and its pond.
        parts.append(flat_patch(-0.12, -0.3, 0.2, "Turf"))
        parts.append(flat_patch(-0.14, -0.32, 0.09, "Pond", 8))
        keep_out.append((-0.12, -0.3, 0.2))
    barns = ((-0.74, -0.3, 0.3), (0.72, 0.44, 1.9)) if first else ((0.66, -0.5, 1.2),)
    for bx, by, angle in barns:
        parts += barn(bx, by, angle)
        keep_out.append((bx, by, 0.2))
    parts += scatter(
        rng, 14 if first else 12, 0.8, cottage, keep_out, None, (0.19, 0.25), 1.2
    )
    parts += plots(-0.15, -0.98, road, 5, 0.6, 0.075, FIELDS)
    parts += plots(0.93, -0.15, road + TAU / 4, 4, 0.5, 0.075, FIELDS)
    return parts


east.register_family(
    FAMILY,
    (
        build_west_city,
        build_west_town,
        build_west_castle,
        build_west_abbey,
        build_west_village,
    ),
)
