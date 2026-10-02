"""Buildings outside the walls of the campaign map, three levels each (lot TB3, ADR 0162).

No paid generation (ADR 0152): every model is an assembly of the ``low`` detail recipes of the
building kit (``building_kit.py``: cottage, longère, barn, stone house, church, manor, market
hall, windmill, water mill, haystack, well, lychgate, wall run) plus a few procedural pieces
written here with the same primitives (vine rows, salt pans, jetty, treadwheel crane, mine whim
and headframe, cloister wings, stalls, tents, boats, scaffolding).

Level 1 is one building, level 2 a small yard, level 3 a small estate. Conventions of the kit:
metres, Z up, ground at z = 0, foundations below (models sit on gentle slopes), front towards -Y
(Godot: +Z). The front is the *site side*: the water for mills, saltworks and ports, the road for
markets. Every material is a layer of the ``Building`` atlas, so a model is one surface and one
draw call per MultiMesh.

Run headless:
    blender --background --python tb3_outbuildings.py -- export <out_dir>
    python3 tb3_outbuildings.py stats          (triangle counts and footprints, no Blender)

``export`` writes ``<family>_<level>.glb``, ``worksite_1.glb`` and ``manifest.json``.
"""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import building_kit as kit  # noqa: E402
import kit_geometry as k  # noqa: E402
from kit_geometry import Geometry  # noqa: E402

# Layers of the ``Building`` atlas (same list as ``kit_export.ATLAS_LAYERS``).
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
    "TimberFrame",
    "TimberFrameFar",
]
# Triangle caps per level (campaign view: a model is a few dozen pixels wide).
TRIANGLE_CAPS = {1: 800, 2: 1800, 3: 3500}

WOOD = (0.5, 0.4, 0.33)
DARK_WOOD = (0.34, 0.27, 0.22)
STONE = (0.95, 0.92, 0.86)
CLAY = (0.78, 0.68, 0.52)
SPOIL = (0.52, 0.47, 0.42)
ORE = (0.45, 0.33, 0.28)
SALT = (1.0, 1.0, 1.0)
BRINE = (0.42, 0.52, 0.6)
VINE = (0.42, 0.8, 0.26)
HAY = (0.86, 0.72, 0.32)
CLOTHS = [(0.86, 0.3, 0.24), (0.3, 0.42, 0.7), (0.9, 0.8, 0.5), (0.4, 0.6, 0.36)]
TILE = kit.ROOF_TINTS["RoofTile"][0]


# --- pieces ---------------------------------------------------------------------------------


def put(
    g: Geometry, kind: str, seed: int, x: float, y: float, yaw: float = 0.0, **dims
):
    """Merge one ``low`` detail kit building at (x, y), turned by ``yaw`` degrees."""
    part, info = kit.build(kind, seed, "low", False, **dims)
    g.merge(part.transformed((1.0, 1.0, 1.0), (x, y, 0.0), math.radians(yaw)))
    return info


def run(g, points, height, thick, mat, color, depth=0.8, closed=False) -> None:
    """Fence or wall along a polyline, sunk ``depth`` m into the ground."""
    count = len(points)
    for i in range(count if closed else count - 1):
        a, b = points[i], points[(i + 1) % count]
        dx, dy = b[0] - a[0], b[1] - a[1]
        k.box(
            g,
            ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (height - depth) / 2),
            (math.hypot(dx, dy) + thick, thick, height + depth),
            mat,
            math.atan2(dy, dx),
            color=color,
        )


def rect(x0, y0, x1, y1) -> list:
    """Corners of a rectangle, counter-clockwise."""
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]


def extrude(g, points, z0, z1, mat, color) -> None:
    """Prism over a counter-clockwise outline: sides and top."""
    count = len(points)
    for i in range(count):
        a, b = points[i], points[(i + 1) % count]
        g.quad((*a, z0), (*b, z0), (*b, z1), (*a, z1), mat, color=color)
    g.poly([(*p, z1) for p in points], mat, color=color, occlude=False)


def wing(g, x, y, yaw, length, width, height, wall="Ashlar", roof="RoofTile") -> None:
    """Plain range of a cloister or a storehouse: walls and a gabled roof."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        k.box(
            g,
            (0, 0, (height - 1.5) / 2),
            (length, width, height + 1.5),
            wall,
            color=STONE,
        )
        k.prism_x(
            g,
            -length / 2 - 0.3,
            length / 2 + 0.3,
            0.0,
            height,
            width / 2 + 0.35,
            width * 0.55,
            roof,
            color=kit.ROOF_TINTS[roof][0],
        )


def stall(g, x, y, yaw, tint) -> None:
    """Market stall: four posts, a trestle table, a canvas awning."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        for sx in (-1.3, 1.3):
            for sy in (-0.8, 0.8):
                k.box(g, (sx, sy, 0.85), (0.1, 0.1, 2.7), "Timber", color=WOOD)
        k.box(g, (0, 0, 0.85), (2.4, 1.0, 0.1), "Planks", color=WOOD, bottom=True)
        awning = [
            (-1.6, -1.1, 1.9),
            (1.6, -1.1, 1.9),
            (1.6, 1.0, 2.5),
            (-1.6, 1.0, 2.5),
        ]
        g.poly(awning, "Canvas", color=tint, occlude=False)
        g.poly(awning[::-1], "Canvas", color=tint, occlude=False)


def tent(g, x, y, yaw, length, width, height, tint) -> None:
    """Ridge tent of a fair."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        k.prism_x(
            g,
            -length / 2,
            length / 2,
            0.0,
            -0.3,
            width / 2,
            height + 0.3,
            "Canvas",
            color=tint,
        )


def barrel(g, x, y) -> None:
    """Standing barrel."""
    k.cylinder(g, (x, y, -0.2), 0.36, 1.1, "Planks", sides=6, color=DARK_WOOD)


def heap(g, x, y, radius, height, mat, color, sides=7) -> None:
    """Conical heap (spoil, ore, salt, stone)."""
    k.cone(
        g,
        (x, y, -0.8),
        radius * (1.0 + 0.8 / height),
        height + 0.8,
        mat,
        sides=sides,
        color=color,
    )


def cart(g, x, y, yaw) -> None:
    """Two-wheeled cart, shafts resting on the ground."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        k.box(g, (0, 0, 1.0), (3.0, 1.4, 0.5), "Planks", color=WOOD, bottom=True)
        for sy in (-0.85, 0.85):
            k.tube(
                g,
                (0.2, sy - 0.06, 0.65),
                (0.2, sy + 0.06, 0.65),
                0.65,
                "Timber",
                8,
                color=DARK_WOOD,
            )
            k.beam(
                g,
                (-1.5, sy * 0.7, 0.8),
                (-3.4, sy * 0.7, 0.05),
                0.08,
                0.08,
                "Timber",
                (0, 0, 1),
                color=WOOD,
            )


def vines(g, x0, y0, rows, length, spacing=2.2) -> None:
    """Rows of vines along X: a leafy hedge on stakes."""
    for i in range(rows):
        y = y0 + i * spacing
        k.prism_x(g, x0, x0 + length, y, 0.35, 0.42, 1.15, "Canvas", color=VINE)
        for x in (x0 - 0.2, x0 + length + 0.2):
            k.box(g, (x, y, 0.5), (0.1, 0.1, 2.0), "Timber", color=DARK_WOOD)


def pan(g, x, y, width, depth, salted: bool) -> None:
    """Evaporation pan: clay bund, brine or salt crust."""
    x0, y0, x1, y1 = x - width / 2, y - depth / 2, x + width / 2, y + depth / 2
    run(g, rect(x0, y0, x1, y1), 0.45, 0.7, "Rubble", CLAY, depth=1.6, closed=True)
    g.quad(
        (x0, y0, 0.28),
        (x1, y0, 0.28),
        (x1, y1, 0.28),
        (x0, y1, 0.28),
        "Plaster" if salted else "Canvas",
        color=SALT if salted else BRINE,
        occlude=False,
    )


def jetty(g, x, y0, y1, width) -> None:
    """Timber jetty along Y from the shore (y0) out to y1, deck 1.1 m above the anchor."""
    span = abs(y1 - y0)
    k.box(
        g,
        (x, (y0 + y1) / 2, 1.1),
        (width, span, 0.3),
        "Planks",
        color=WOOD,
        bottom=True,
    )
    piles = max(2, int(span / 5.0))
    for i in range(piles + 1):
        y = y0 + (y1 - y0) * i / piles
        for sx in (-width / 2, width / 2):
            k.box(g, (x + sx, y, -1.6), (0.35, 0.35, 6.2), "Timber", color=DARK_WOOD)


def crane(g, x, y, yaw) -> None:
    """Treadwheel crane: wheel house, mast and jib."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        k.box(g, (0, 0, 1.2), (3.0, 3.0, 3.6), "Planks", color=WOOD)
        k.prism_x(
            g,
            -1.8,
            1.8,
            0.0,
            3.0,
            1.9,
            1.5,
            "RoofFlat",
            color=kit.ROOF_TINTS["RoofFlat"][1],
        )
        k.box(g, (0, 0, 6.0), (0.4, 0.4, 5.0), "Timber", color=DARK_WOOD)
        k.beam(
            g,
            (0, 0, 7.6),
            (0, -5.5, 9.8),
            0.3,
            0.3,
            "Timber",
            (1, 0, 0),
            color=DARK_WOOD,
        )
        k.beam(
            g,
            (0, 0, 8.4),
            (0, 2.2, 6.4),
            0.25,
            0.25,
            "Timber",
            (1, 0, 0),
            color=DARK_WOOD,
        )


def boat(g, x, y, yaw, length, beam_m, mast=0.0) -> None:
    """Open boat (or a cog with a mast), hull resting 0.3 m in the water or the sand."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        half, w = length / 2, beam_m / 2
        outline = [
            (half, 0.0),
            (half * 0.45, w),
            (-half, w * 0.8),
            (-half, -w * 0.8),
            (half * 0.45, -w),
        ]
        extrude(g, outline, -0.4, 0.25 * beam_m + 0.4, "Planks", DARK_WOOD)
        if mast > 0.0:
            k.box(g, (0, 0, mast / 2), (0.3, 0.3, mast), "Timber", color=WOOD)
            k.tube(
                g,
                (0, -beam_m * 1.3, mast * 0.82),
                (0, beam_m * 1.3, mast * 0.82),
                0.25,
                "Canvas",
                5,
                color=(1.0, 0.98, 0.92),
            )


def dovecote(g, x, y) -> None:
    """Round stone dovecote under a conical tile roof."""
    k.cylinder(g, (x, y, -1.5), 2.7, 8.5, "Rubble", sides=8, top=False, color=STONE)
    k.cone(g, (x, y, 7.0), 3.1, 3.4, "RoofTile", sides=8, color=TILE)


def whim(g, x, y) -> None:
    """Horse whim of a mine: winding drum under a thatched round roof."""
    for i in range(6):
        a = math.tau * i / 6
        k.box(
            g,
            (x + 4.0 * math.cos(a), y + 4.0 * math.sin(a), 1.2),
            (0.25, 0.25, 4.0),
            "Timber",
            color=WOOD,
        )
    k.cone(g, (x, y, 3.0), 4.9, 2.8, "Thatch", sides=8, color=HAY)
    k.cylinder(g, (x, y, -0.5), 0.8, 3.2, "Timber", sides=6, color=DARK_WOOD)


def headframe(g, x, y) -> None:
    """Timber headframe over a shaft, with its pulley."""
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.tube(
                g,
                (x + sx * 2.2, y + sy * 2.2, -1.0),
                (x + sx * 0.5, y + sy * 0.5, 9.0),
                0.16,
                "Timber",
                4,
                color=DARK_WOOD,
            )
    k.tube(g, (x - 0.15, y, 9.3), (x + 0.15, y, 9.3), 1.0, "Timber", 8, color=WOOD)
    k.box(g, (x, y, 0.9), (3.6, 3.6, 2.6), "Planks", color=WOOD)


def adit(g, x, y, yaw) -> None:
    """Mine entrance: spoil bank, timber portal, dark mouth."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        heap(g, 0, 4.0, 8.0, 5.5, "Rubble", SPOIL, sides=9)
        for sx in (-1.1, 1.1):
            k.box(g, (sx, -2.6, 0.9), (0.3, 0.3, 2.8), "Timber", color=DARK_WOOD)
        k.box(g, (0, -2.6, 2.45), (2.9, 0.4, 0.3), "Timber", color=DARK_WOOD)
        g.quad(
            (-1.0, -2.5, 0.0),
            (1.0, -2.5, 0.0),
            (1.0, -2.5, 2.3),
            (-1.0, -2.5, 2.3),
            "Window",
        )


def chimney_stack(g, x, y, height) -> None:
    """Tall stone stack of a furnace or a boiling house."""
    k.cylinder(
        g,
        (x, y, -1.0),
        0.9,
        height + 1.0,
        "Rubble",
        sides=6,
        radius_top=0.6,
        color=STONE,
    )


def market_cross(g, x, y) -> None:
    """Stepped market cross."""
    k.box(g, (x, y, 0.1), (3.0, 3.0, 1.0), "Ashlar", color=STONE)
    k.box(g, (x, y, 0.85), (1.8, 1.8, 0.5), "Ashlar", color=STONE)
    k.box(g, (x, y, 2.9), (0.35, 0.35, 3.6), "Ashlar", color=STONE)
    k.box(g, (x, y, 4.2), (1.3, 0.3, 0.3), "Ashlar", color=STONE)


def scaffold(g, x, y, yaw, length, height) -> None:
    """Pole scaffolding with two plank lifts and a ladder, in front of a wall along X."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        bays = max(2, int(length / 2.6))
        for i in range(bays + 1):
            px = -length / 2 + length * i / bays
            for py in (-1.3, -0.2):
                k.box(
                    g,
                    (px, py, height / 2 - 0.3),
                    (0.12, 0.12, height + 0.6),
                    "Timber",
                    color=WOOD,
                )
        for lift in (height * 0.45, height * 0.85):
            k.box(
                g,
                (0, -0.75, lift),
                (length + 0.4, 1.3, 0.08),
                "Planks",
                color=WOOD,
                bottom=True,
            )
        k.beam(
            g,
            (-length / 2, -1.5, 0.0),
            (-length / 2 + 1.6, -1.5, height * 0.85),
            0.5,
            0.08,
            "Timber",
            (0, -1, 0),
            color=WOOD,
        )


# --- models ---------------------------------------------------------------------------------


def farm_1(g) -> None:
    """A longère and its rick."""
    put(g, "longere", 511, 0, 0)
    put(g, "haystack", 512, 12, 5)
    run(g, [(-11, -9), (9, -9), (9, -4)], 1.1, 0.12, "Planks", WOOD)


def farm_2(g) -> None:
    """House, barn, well and ricks around a yard."""
    put(g, "longere", 521, 0, 9)
    put(g, "barn", 522, -16, -4, 90)
    put(g, "well", 523, 2, -3)
    put(g, "haystack", 524, 14, -6)
    put(g, "haystack", 525, 19, 1)
    cart(g, -6, -9, 20)
    run(g, [(-22, -14), (10, -14), (10, -9)], 1.1, 0.12, "Planks", WOOD)


def farm_3(g) -> None:
    """Courtyard farm: stone house, two barns, byre, dovecote, walled yard."""
    put(g, "stonehouse", 531, 0, 22, length=14.0, depth=8.0)
    put(g, "barn", 532, -24, 4, 90, length=20.0, depth=9.0)
    put(g, "barn", 533, 24, 4, 90, length=18.0, depth=9.0)
    put(g, "longere", 534, 0, -20, 180)
    put(g, "cottage", 535, 34, -22, 30)
    put(g, "well", 536, 4, 4)
    dovecote(g, -9, 3)
    for n, (x, y) in enumerate([(38, 10), (44, 3), (40, -6)]):
        put(g, "haystack", 537 + n, x, y)
    cart(g, 12, -6, -30)
    cart(g, -14, -10, 70)
    run(
        g,
        [(-30, -26), (-30, 28), (30, 28), (30, -26), (12, -26)],
        1.6,
        0.6,
        "Rubble",
        STONE,
    )


def mill_1(g) -> None:
    """Post windmill and the miller's cottage."""
    put(g, "windmill", 611, 0, 0)
    put(g, "cottage", 612, 13, 5, -20)
    for x, y in [(4.5, -3), (5.3, -2.2)]:
        barrel(g, x, y)


def mill_2(g) -> None:
    """Water mill on its race, house and store."""
    put(g, "watermill", 621, 0, 0, length=11.0, depth=6.8)
    put(g, "cottage", 622, 15, 6, -15)
    put(g, "barn", 623, -15, 9, 90, length=12.0, depth=7.5)
    k.box(g, (-2.2, -9.0, -0.3), (1.4, 12.0, 0.9), "Planks", color=DARK_WOOD)
    cart(g, 8, 10, 10)
    for x, y in [(5, 4.5), (5.8, 5.2), (4.3, 5.4)]:
        barrel(g, x, y)


def mill_3(g) -> None:
    """Mill hamlet: two wheels (grain, fulling or hammer), windmill, houses, store."""
    put(g, "watermill", 631, -9, 0, length=12.0, depth=7.0)
    put(g, "watermill", 632, 11, 0, length=11.0, depth=6.8)
    chimney_stack(g, 17, 2.5, 10.0)
    put(g, "windmill", 633, 30, 22)
    put(g, "barn", 634, -26, 14, 90, length=16.0, depth=9.0)
    put(g, "stonehouse", 635, 2, 20)
    put(g, "cottage", 636, -12, 24, 10)
    put(g, "cottage", 637, 18, 26, -12)
    k.box(g, (1.0, -8.5, -0.3), (34.0, 1.6, 0.9), "Planks", color=DARK_WOOD)
    cart(g, -4, 11, 15)
    cart(g, 22, 10, -40)
    for x, y in [(-17, 6), (-16.2, 6.8), (5, 9), (5.8, 9.5), (6, 8.6)]:
        barrel(g, x, y)


def vineyard_1(g) -> None:
    """Press shed and a few rows."""
    put(g, "barn", 711, 0, 9, length=10.0, depth=7.0)
    vines(g, -14, -12, 6, 28)
    barrel(g, 6.5, 6)
    barrel(g, 7.4, 6.4)


def vineyard_2(g) -> None:
    """Press house, cellar and a plot of vines."""
    put(g, "stonehouse", 721, -8, 18)
    put(g, "longere", 722, 12, 20)
    vines(g, -26, -22, 13, 52)
    cart(g, 2, 10, 0)
    for x, y in [(-1, 14), (-0.2, 14.7), (0.6, 14), (20, 14)]:
        barrel(g, x, y)


def vineyard_3(g) -> None:
    """Walled clos: house, press house, cellar, two plots."""
    put(g, "manor", 731, 0, 34, length=20.0, depth=7.5)
    put(g, "stonehouse", 732, -24, 30)
    put(g, "longere", 733, 26, 30)
    put(g, "barn", 734, 40, 14, 90, length=13.0, depth=8.0)
    vines(g, -42, -40, 14, 38)
    vines(g, 4, -40, 14, 38)
    vines(g, -42, -4, 9, 30)
    cart(g, 8, 18, 0)
    cart(g, -10, 20, 40)
    for x, y in [(-16, 24), (-15.2, 24.6), (-14.4, 24), (18, 24), (18.8, 24.6)]:
        barrel(g, x, y)
    run(
        g,
        [(-4, -44), (-46, -44), (-46, 42), (46, 42), (46, -44), (4, -44)],
        1.7,
        0.6,
        "Rubble",
        STONE,
    )


def mine_1(g) -> None:
    """Adit, spoil heap and a miner's hut."""
    adit(g, 0, 4, 0)
    put(g, "cottage", 811, 14, -4, -25)
    heap(g, -9, -5, 2.6, 1.6, "Rubble", ORE)


def mine_2(g) -> None:
    """Adit, horse whim, washing trough, smithy and huts."""
    adit(g, -10, 10, 0)
    whim(g, 8, 6)
    put(g, "stonehouse", 821, 22, -10, -20, length=10.0, depth=7.0)
    chimney_stack(g, 27, -7, 9.0)
    put(g, "cottage", 822, -20, -12, 15)
    k.box(g, (0, -10, 0.1), (14.0, 1.2, 0.9), "Planks", color=DARK_WOOD)
    for x, y, r in [(-4, -3, 3.0), (2, -16, 2.2), (30, 4, 3.4)]:
        heap(g, x, y, r, r * 0.6, "Rubble", ORE)
    cart(g, 12, -16, 30)


def mine_3(g) -> None:
    """Mining works: two adits, headframe, whim, furnace hall, stores, miners' row."""
    adit(g, -24, 22, 10)
    adit(g, 4, 28, -8)
    headframe(g, 22, 14)
    whim(g, -6, 4)
    put(g, "barn", 831, 30, -12, 0, length=18.0, depth=9.0)
    chimney_stack(g, 24, -5, 13.0)
    chimney_stack(g, 36, -5, 11.0)
    put(g, "stonehouse", 832, -30, -8, 10)
    for n, x in enumerate((-34, -22, -10)):
        put(g, "cottage", 833 + n, x, -26, 4 * n)
    put(g, "longere", 836, 8, -30)
    k.box(g, (6, -12, 0.1), (20.0, 1.2, 0.9), "Planks", color=DARK_WOOD)
    for x, y, r in [
        (-14, -8, 3.4),
        (12, -2, 2.6),
        (40, 8, 4.2),
        (44, -2, 3.0),
        (-40, 8, 4.8),
    ]:
        heap(g, x, y, r, r * 0.6, "Rubble", ORE if r < 4.0 else SPOIL)
    cart(g, 16, -20, 20)
    cart(g, -2, -18, -35)


def saltworks_1(g) -> None:
    """Boiling hut and four pans."""
    put(g, "cottage", 911, 0, 12)
    for i in range(4):
        pan(g, -13.5 + 9 * i, -6, 8, 12, i % 2 == 0)
    heap(g, 9, 9, 1.8, 1.3, "Plaster", SALT)


def saltworks_2(g) -> None:
    """Boiling house with its stack, store, a dozen pans."""
    put(g, "longere", 921, -8, 22)
    chimney_stack(g, -2, 25, 9.0)
    put(g, "barn", 922, 18, 22, 0, length=12.0, depth=8.0)
    for row in range(2):
        for i in range(6):
            pan(g, -25 + 10 * i, -22 + 15 * row, 9, 14, (i + row) % 3 != 0)
    for x, y in [(4, 15), (8, 16.5), (-20, 16)]:
        heap(g, x, y, 2.0, 1.5, "Plaster", SALT)
    cart(g, 14, 13, 10)


def saltworks_3(g) -> None:
    """Great saltworks: two boiling houses, salt store, a field of pans, carts."""
    put(g, "longere", 931, -22, 40)
    put(g, "longere", 932, 4, 42, 4)
    chimney_stack(g, -15, 43, 11.0)
    chimney_stack(g, 11, 45, 10.0)
    put(g, "barn", 933, 34, 38, 0, length=20.0, depth=9.5)
    put(g, "stonehouse", 934, -46, 36, 10)
    for row in range(4):
        for i in range(8):
            pan(g, -38.5 + 11 * i, -42 + 17 * row, 10, 16, (i * 3 + row * 5) % 4 != 0)
    for x, y, r in [
        (18, 30, 2.8),
        (24, 31, 2.2),
        (-8, 31, 2.4),
        (-34, 30, 2.0),
        (46, 28, 2.6),
    ]:
        heap(g, x, y, r, r * 0.75, "Plaster", SALT)
    cart(g, 12, 29, 0)
    cart(g, -26, 29, 20)


def abbey_1(g) -> None:
    """Priory: chapel and one range of cells."""
    put(g, "church", 1011, 0, 0, length=19.0, depth=8.0)
    put(g, "longere", 1012, 2, -14)
    run(g, [(-14, -20), (-14, 8)], 1.6, 0.6, "Rubble", STONE)


def _cloister(g, x, y, side) -> None:
    """Three ranges around a garth, south of the church."""
    wing(g, x - side / 2, y, 90, side, 7.0, 5.5)
    wing(g, x + side / 2, y, 90, side, 7.0, 5.5)
    wing(g, x, y - side / 2, 0, side + 7.0, 7.5, 5.5)


def abbey_2(g) -> None:
    """Abbey: church, cloister, barn, precinct wall and gate."""
    put(g, "church", 1021, 0, 14, length=28.0, depth=10.0)
    _cloister(g, 0, -6, 22.0)
    put(g, "barn", 1022, 32, -14, 90, length=18.0, depth=9.0)
    put(g, "lychgate", 1023, -34, -10, 90)
    run(
        g,
        [(-34, -8), (-34, 30), (44, 30), (44, -34), (-34, -34), (-34, -12)],
        2.4,
        0.7,
        "Rubble",
        STONE,
    )


def abbey_3(g) -> None:
    """Great abbey: large church, cloister, guest house, infirmary, tithe barn, farm court."""
    put(g, "church", 1031, 0, 24, length=40.0, depth=13.0)
    _cloister(g, -4, -2, 28.0)
    put(g, "manor", 1032, -44, 10, 90, length=20.0, depth=7.5)
    put(g, "longere", 1033, 34, -4, 90)
    put(g, "stonehouse", 1034, 34, 22, 90)
    put(g, "barn", 1035, 30, -40, 0, length=26.0, depth=10.0)
    put(g, "barn", 1036, -6, -44, 0, length=16.0, depth=9.0)
    put(g, "longere", 1037, -40, -38, 10)
    dovecote(g, 52, -24)
    put(g, "well", 1038, -4, -2)
    put(g, "lychgate", 1039, -60, -14, 90)
    run(
        g,
        [(-60, -12), (-60, 46), (62, 46), (62, -56), (-60, -56), (-60, -16)],
        2.6,
        0.8,
        "Rubble",
        STONE,
    )


def market_1(g) -> None:
    """Market cross and a few stalls."""
    market_cross(g, 0, 0)
    for n, (x, y, yaw) in enumerate(
        [(-7, 3, 10), (-6, -5, -15), (6, 5, 170), (7, -4, 195)]
    ):
        stall(g, x, y, yaw, CLOTHS[n % len(CLOTHS)])
    cart(g, 0, -10, 80)


def market_2(g) -> None:
    """Market hall, cross and two rows of stalls."""
    put(g, "hall", 1121, 0, 12)
    market_cross(g, 0, -6)
    for i in range(5):
        stall(g, -14 + 7 * i, -14, 0, CLOTHS[i % len(CLOTHS)])
        stall(g, -14 + 7 * i, -22, 180, CLOTHS[(i + 2) % len(CLOTHS)])
    cart(g, 20, -4, 60)
    cart(g, -20, 0, 120)
    for x, y in [(-12, 4), (-11.2, 4.6), (13, 4)]:
        barrel(g, x, y)


def market_3(g) -> None:
    """Fair ground: hall, guild house, rows of stalls and tents, carts."""
    put(g, "hall", 1131, -14, 30)
    put(g, "stonehouse", 1132, 18, 32, length=14.0, depth=8.5)
    market_cross(g, 0, 12)
    for row in range(3):
        for i in range(7):
            stall(
                g,
                -24 + 8 * i,
                -2 - 10 * row,
                180 * (row % 2),
                CLOTHS[(i + row) % len(CLOTHS)],
            )
    for n, (x, y, yaw) in enumerate(
        [
            (-38, -30, 15),
            (-28, -36, -10),
            (30, -34, 20),
            (40, -26, 80),
            (40, 6, 95),
            (-40, 8, 85),
        ]
    ):
        tent(g, x, y, yaw, 7.0, 5.0, 3.2, CLOTHS[n % len(CLOTHS)])
    for x, y, yaw in [(-34, 18, 30), (36, 18, 150), (0, -36, 5), (12, -38, -20)]:
        cart(g, x, y, yaw)
    for x, y in [(-6, 20), (-5.2, 20.6), (6, 20), (26, 22), (26.8, 22.5)]:
        barrel(g, x, y)


def port_1(g) -> None:
    """Jetty, a store and two boats."""
    jetty(g, 0, 0, -22, 3.5)
    put(g, "barn", 1211, -10, 12, length=11.0, depth=7.0)
    boat(g, 5, -12, 80, 6.0, 1.8)
    boat(g, 9, 2, 20, 5.5, 1.7)
    barrel(g, 3, 3)
    barrel(g, 3.8, 3.6)


def port_2(g) -> None:
    """Two jetties, crane, warehouses, a cog."""
    jetty(g, -10, 0, -30, 4.0)
    jetty(g, 16, 0, -24, 3.5)
    k.box(g, (3, 1.0, 0.2), (44.0, 4.0, 2.0), "Rubble", color=STONE)
    crane(g, -4, 2, 0)
    put(g, "barn", 1221, -18, 16, length=14.0, depth=8.0)
    put(g, "stonehouse", 1222, 6, 18)
    put(g, "cottage", 1223, 24, 16, -10)
    boat(g, -2, -18, 90, 17.0, 5.0, mast=13.0)
    boat(g, 22, -12, 85, 6.0, 1.8)
    boat(g, 28, -3, 30, 5.5, 1.7)
    for x, y in [(-12, 5), (-11.2, 5.6), (10, 5), (10.8, 5.4)]:
        barrel(g, x, y)


def port_3(g) -> None:
    """Harbour: stone quay, three jetties, crane, hall, warehouses, two cogs and boats."""
    k.box(g, (0, 1.5, 0.2), (86.0, 5.0, 2.0), "Rubble", color=STONE)
    jetty(g, -28, -1, -34, 4.5)
    jetty(g, 0, -1, -40, 4.5)
    jetty(g, 28, -1, -30, 4.0)
    crane(g, -12, 2.5, 0)
    crane(g, 16, 2.5, 0)
    put(g, "hall", 1231, 0, 24)
    put(g, "barn", 1232, -30, 20, length=18.0, depth=9.0)
    put(g, "barn", 1233, 30, 20, length=16.0, depth=9.0)
    put(g, "stonehouse", 1234, -22, 40)
    put(g, "stonehouse", 1235, 20, 42)
    put(g, "cottage", 1236, 40, 40, -15)
    put(g, "cottage", 1237, -40, 38, 12)
    boat(g, -14, -20, 90, 18.0, 5.2, mast=14.0)
    boat(g, 14, -24, 92, 17.0, 5.0, mast=13.0)
    for x, y, yaw in [(36, -14, 85), (40, -2, 20), (-38, -10, 100), (-42, -1, 160)]:
        boat(g, x, y, yaw, 6.0, 1.8)
    cart(g, -4, 10, 0)
    cart(g, 24, 10, 180)
    for x, y in [(-20, 6), (-19.2, 6.6), (-18.4, 6), (6, 6), (6.8, 6.6), (30, 6)]:
        barrel(g, x, y)


def worksite_1(g) -> None:
    """Building site: a wall going up under scaffolding, crane, stone and timber stacks."""
    k.box(g, (-3, 0, 1.0), (12.0, 1.2, 5.0), "Ashlar", color=STONE)
    k.box(g, (6, 0, 0.0), (6.0, 1.2, 3.0), "Ashlar", color=STONE)
    k.box(g, (10, 5, -0.3), (1.2, 10.0, 2.4), "Ashlar", color=STONE)
    scaffold(g, -3, -0.7, 0, 12.0, 4.6)
    crane(g, 4, -9, 180)
    for x, y, r in [(-10, -8, 2.0), (-6, -10, 1.5), (-12, -4, 1.3)]:
        heap(g, x, y, r, r * 0.7, "Ashlar", STONE, sides=6)
    for i in range(4):
        k.box(
            g,
            (12 + 0.1 * i, -9 - 0.6 * i, 0.25 + 0.3 * (i % 2)),
            (5.0, 0.5, 0.5),
            "Timber",
            color=WOOD,
        )
    cart(g, -2, -14, 15)


MODELS = {
    "farm": [farm_1, farm_2, farm_3],
    "mill": [mill_1, mill_2, mill_3],
    "vineyard": [vineyard_1, vineyard_2, vineyard_3],
    "mine": [mine_1, mine_2, mine_3],
    "saltworks": [saltworks_1, saltworks_2, saltworks_3],
    "abbey": [abbey_1, abbey_2, abbey_3],
    "market": [market_1, market_2, market_3],
    "port": [port_1, port_2, port_3],
    "worksite": [worksite_1],
}


def build(family: str, level: int) -> tuple[Geometry, dict]:
    """Geometry and manifest entry of one model."""
    g = Geometry()
    MODELS[family][level - 1](g)
    unknown = sorted(set(g.polys) - set(ATLAS_LAYERS))
    if unknown:
        raise SystemExit(f"{family}_{level}: materials outside the atlas: {unknown}")
    points = [p for polys in g.polys.values() for pts, _, _ in polys for p in pts]
    xs, ys, zs = ([p[i] for p in points] for i in range(3))
    info = {
        "family": family,
        "level": level,
        "radius": round(max(math.hypot(p[0], p[1]) for p in points), 1),
        "length": round(max(xs) - min(xs), 1),
        "depth": round(max(ys) - min(ys), 1),
        "height": round(max(zs), 1),
        "triangles": g.triangle_count(),
    }
    return g, info


def manifest() -> dict:
    """Manifest entries of every model (no Blender needed)."""
    return {
        f"{family}_{level}": build(family, level)[1]
        for family, recipes in MODELS.items()
        for level in range(1, len(recipes) + 1)
    }


def export(out_dir: Path) -> None:
    """Write every model as a GLB (one ``Building`` surface) and the manifest."""
    import kit_export  # noqa: PLC0415  (needs bpy)

    out_dir.mkdir(parents=True, exist_ok=True)
    entries = {}
    for family, recipes in MODELS.items():
        for level in range(1, len(recipes) + 1):
            name = f"{family}_{level}"
            kit_export.reset_scene()
            g, info = build(family, level)
            obj = kit_export.to_object(g, name)
            kit_export.atlas(obj)
            kit_export.export_glb(obj, out_dir / f"{name}.glb")
            entries[name] = info
            print("MODEL", name, info["triangles"])
    (out_dir / "manifest.json").write_text(
        json.dumps(entries, indent=1, sort_keys=True) + "\n"
    )
    print("OK")


def main() -> None:
    """Command line entry point."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else sys.argv[1:]
    if argv[:1] == ["export"] and len(argv) == 2:
        export(Path(argv[1]))
    elif argv[:1] == ["stats"]:
        for name, info in manifest().items():
            cap = TRIANGLE_CAPS[info["level"]]
            flag = "" if info["triangles"] <= cap else "  OVER"
            print(
                f"{name:14} {info['triangles']:5} tri  r {info['radius']:5} m  h {info['height']:5} m{flag}"
            )
    else:
        raise SystemExit("usage: -- export <out_dir> | stats")


if __name__ == "__main__":
    main()
