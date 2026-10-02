"""Buildings outside the walls of the campaign map, three levels each (lot TB3, ADR 0162).

No paid generation (ADR 0152): every model is an assembly of the ``low`` detail recipes of the
building kit (``building_kit.py``: cottage, longère, barn, stone house, church, manor, market
hall, windmill, water mill, haystack, well, lychgate, wall run) plus a few procedural pieces
written here with the same primitives (vine rows, salt pans, jetty, treadwheel crane, mine whim
and headframe, cloister wings, stalls, tents, boats, scaffolding).

The models are **map signs in the round** (ADR 0162): they are drawn at a held screen size
(40 to 70 px), seen from the south at about 30 degrees, so each one is a few big, tall pieces
with one signature silhouette (sails, wheel, bell tower, hall, crane, headframe, white pans,
vine rows), kit buildings enlarged, tall pieces at the back (+Y), low ones in front (-Y).
Level 1 is the signature piece, level 2 adds a yard, level 3 is a rich compound. The settlement
signs (``sign_*``: village, town, walled town, city, castle) follow the same rule. Conventions of the kit:
metres, Z up, ground at z = 0, foundations below (models sit on gentle slopes), front towards -Y
(Godot: +Z). The front is the *site side*: the water for mills, saltworks and ports, the road for
markets. Every material is a layer of the ``Building`` atlas, so a model is one surface and one
draw call per MultiMesh.

Run headless:
    blender --background --python tb3_outbuildings.py -- export <out_dir>
    python3 tb3_outbuildings.py stats          (triangle counts and footprints, no Blender)

``export`` writes ``<family>_<level>.glb``, ``worksite_1.glb`` and ``manifest.json``.
"""

import functools
import inspect
import json
import math
import random
import sys
from contextlib import contextmanager
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
SPOIL = (0.55, 0.52, 0.5)
ORE = (0.75, 0.4, 0.3)
SALT = (1.0, 1.0, 1.0)
BRINE = (0.42, 0.52, 0.6)
VINE = (0.16, 0.42, 0.12)
HAY = (0.86, 0.72, 0.32)
EARTH = (0.78, 0.62, 0.42)
PALE = (1.0, 0.97, 0.9)
CLOTHS = [(0.86, 0.3, 0.24), (0.3, 0.42, 0.7), (0.9, 0.8, 0.5), (0.4, 0.6, 0.36)]
TILE = kit.ROOF_TINTS["RoofTile"][0]


# --- pieces ---------------------------------------------------------------------------------

# Rigid pieces of the model being built: plan bounding boxes (x0, y0, x1, y1) of what each
# top-level call added. Far away a model is several kilometres wide on the map: Godot stands
# every piece on the ground under its own centre (``pieces`` of the manifest), so that no
# building is buried in a slope or floats over a valley.
_PIECES: list = []
_STATE = {"root": None, "depth": 0}


@contextmanager
def part(g: Geometry):
    """Record what the body adds to the root geometry as one rigid piece."""
    if g is not _STATE["root"] or _STATE["depth"] > 0:
        yield
        return
    _STATE["depth"] += 1
    before = {mat: len(polys) for mat, polys in g.polys.items()}
    try:
        yield
    finally:
        _STATE["depth"] -= 1
        points = [
            point
            for mat, polys in g.polys.items()
            for poly in polys[before.get(mat, 0) :]
            for point in poly[0]
        ]
        if points:
            xs = [point[0] for point in points]
            ys = [point[1] for point in points]
            _PIECES.append((min(xs), min(ys), max(xs), max(ys)))


def rigid(fn):
    """Decorator: the whole call is one rigid piece."""

    @functools.wraps(fn)
    def wrapper(g, *args, **kwargs):
        with part(g):
            return fn(g, *args, **kwargs)

    return wrapper


@rigid
def put(
    g: Geometry,
    kind: str,
    seed: int,
    x: float,
    y: float,
    yaw: float = 0.0,
    s: float = 1.0,
    **dims,
):
    """Merge one ``low`` detail kit building at (x, y), turned by ``yaw`` degrees, scaled ``s``."""
    part, info = kit.build(kind, seed, "low", False, **dims)
    g.merge(part.transformed((s, s, s), (x, y, 0.0), math.radians(yaw)))
    return info


@rigid
def piece(g: Geometry, fn, x: float, y: float, yaw: float, s: float, *args) -> None:
    """Merge a procedural piece built at the origin by ``fn(g, 0, 0, *args)``, scaled ``s``."""
    part = Geometry()
    if "yaw" in inspect.signature(fn).parameters:
        fn(part, 0.0, 0.0, 0.0, *args)
    else:
        fn(part, 0.0, 0.0, *args)
    g.merge(part.transformed((s, s, s), (x, y, 0.0), math.radians(yaw)))


def run(g, points, height, thick, mat, color, depth=0.8, closed=False) -> None:
    """Fence or wall along a polyline, sunk ``depth`` m into the ground."""
    count = len(points)
    for i in range(count if closed else count - 1):
        a, b = points[i], points[(i + 1) % count]
        dx, dy = b[0] - a[0], b[1] - a[1]
        with part(g):
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


@rigid
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


@rigid
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


@rigid
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


@rigid
def barrel(g, x, y) -> None:
    """Standing barrel."""
    k.cylinder(g, (x, y, -0.2), 0.36, 1.1, "Planks", sides=6, color=DARK_WOOD)


@rigid
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


@rigid
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


@rigid
def vines(g, x0, y0, rows, length, spacing=5.6) -> None:
    """Plot of vines: fat dark rows running away from the viewer (along Y) on pale earth.

    ``x0, y0`` is the front left corner, ``rows`` the number of rows side by side along X,
    ``length`` their length along Y. Seen from the south the rows are vertical stripes.
    """
    width = (rows - 1) * spacing + 4.0
    k.box(
        g,
        (x0 + width / 2 - 2.0, y0 + length / 2, 0.0),
        (width, length + 2.0, 1.0),
        "Plaster",
        color=EARTH,
    )
    for i in range(rows):
        x = x0 + i * spacing
        with kit.frame(g, (x, y0 + length / 2, 0.5), math.radians(90)):
            k.prism_x(
                g, -length / 2, length / 2, 0.0, 0.0, 1.5, 3.4, "Canvas", color=VINE
            )


@rigid
def house(g, x, y, yaw, length, width, height, roof="RoofTile", wall="Plaster") -> None:
    """Plain house of a settlement sign: pale walls under a steep roof."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        k.box(
            g,
            (0, 0, (height - 1.5) / 2),
            (length, width, height + 1.5),
            wall,
            color=PALE,
        )
        k.prism_x(
            g,
            -length / 2 - 0.4,
            length / 2 + 0.4,
            0.0,
            height,
            width / 2 + 0.5,
            width * 0.75,
            roof,
            color=kit.ROOF_TINTS[roof][0],
        )


@rigid
def tower(g, x, y, radius, height, roof="RoofSlate") -> None:
    """Round wall tower under a conical roof."""
    k.cylinder(
        g,
        (x, y, -1.5),
        radius,
        height + 1.5,
        "Masonry",
        sides=8,
        top=False,
        color=STONE,
    )
    k.cone(
        g,
        (x, y, height),
        radius * 1.2,
        radius * 1.5,
        roof,
        sides=8,
        color=kit.ROOF_TINTS[roof][0],
    )


@rigid
def keep(g, x, y, side, height) -> None:
    """Square keep with corner turrets."""
    k.box(
        g,
        (x, y, (height - 1.5) / 2),
        (side, side, height + 1.5),
        "Masonry",
        color=STONE,
    )
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.box(
                g,
                (x + sx * side * 0.45, y + sy * side * 0.45, height + 1.2),
                (side * 0.22, side * 0.22, 2.6),
                "Masonry",
                color=STONE,
            )


def ring_wall(
    g, radius, height, thick, towers, tower_r, tower_h, mat="Masonry"
) -> None:
    """Town wall: a ring of runs, round towers, a gate house on the front (-Y)."""
    sides = 16
    points = [
        (
            radius * math.cos(math.tau * i / sides),
            radius * math.sin(math.tau * i / sides),
        )
        for i in range(sides)
    ]
    run(g, points, height, thick, mat, STONE, depth=1.5, closed=True)
    for i in range(towers):
        a = math.tau * (i + 0.5) / towers - math.pi / 2
        tower(g, radius * math.cos(a), radius * math.sin(a), tower_r, tower_h)
    k.box(
        g,
        (0, -radius, height * 0.7),
        (tower_r * 3.2, thick * 2.2, height * 1.4 + 1.5),
        mat,
        color=STONE,
    )
    g.quad(
        (-tower_r * 0.8, -radius - thick * 1.12, 0.0),
        (tower_r * 0.8, -radius - thick * 1.12, 0.0),
        (tower_r * 0.8, -radius - thick * 1.12, height * 0.75),
        (-tower_r * 0.8, -radius - thick * 1.12, height * 0.75),
        "Window",
    )


def houses(g, seed, count, radius, keep_out, roofs=("RoofTile",), size=1.0) -> None:
    """Tight cluster of houses inside ``radius``, clear of the ``keep_out`` discs (x, y, r)."""
    rng = random.Random(seed)
    placed = []
    tries = 0
    while len(placed) < count and tries < count * 60:
        tries += 1
        a = rng.uniform(0.0, math.tau)
        d = radius * math.sqrt(rng.uniform(0.0, 1.0))
        x, y = d * math.cos(a), d * math.sin(a)
        length = rng.uniform(10.0, 15.0) * size
        if any(math.hypot(x - ox, y - oy) < r + length * 0.5 for ox, oy, r in keep_out):
            continue
        if any(
            math.hypot(x - ox, y - oy) < (length + ol) * 0.52 for ox, oy, ol in placed
        ):
            continue
        placed.append((x, y, length))
        house(
            g,
            x,
            y,
            rng.choice((0, 0, 90, 20, -25)),
            length,
            rng.uniform(6.5, 8.0) * size,
            rng.uniform(5.5, 8.5) * size,
            rng.choice(roofs),
        )


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


@rigid
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


@rigid
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


@rigid
def boat(g, x, y, yaw, length, beam_m, mast=0.0) -> None:
    """Open boat, or a cog with a mast and a square sail, hull resting 0.3 m in the water."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        half, w = length / 2, beam_m / 2
        outline = [
            (half, 0.0),
            (half * 0.45, w),
            (-half, w * 0.8),
            (-half, -w * 0.8),
            (half * 0.45, -w),
        ]
        extrude(g, outline, -0.4, 0.3 * beam_m + 0.6, "Planks", DARK_WOOD)
        if mast > 0.0:
            k.box(g, (0, 0, mast / 2), (0.5, 0.5, mast), "Timber", color=WOOD)
            sail = [
                (-length * 0.3, 0.3, mast * 0.3),
                (length * 0.3, 0.3, mast * 0.3),
                (length * 0.3, 0.3, mast * 0.95),
                (-length * 0.3, 0.3, mast * 0.95),
            ]
            g.poly(sail, "Canvas", color=(1.0, 0.98, 0.92), occlude=False)
            g.poly(sail[::-1], "Canvas", color=(1.0, 0.98, 0.92), occlude=False)


@rigid
def dovecote(g, x, y) -> None:
    """Round stone dovecote under a conical tile roof."""
    k.cylinder(g, (x, y, -1.5), 2.7, 8.5, "Rubble", sides=8, top=False, color=STONE)
    k.cone(g, (x, y, 7.0), 3.1, 3.4, "RoofTile", sides=8, color=TILE)


@rigid
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


@rigid
def headframe(g, x, y) -> None:
    """Timber headframe over a shaft: a boarded tower, a great pulley wheel facing the viewer."""
    k.box(g, (x, y, 5.0), (4.2, 4.2, 12.0), "Planks", color=WOOD)
    k.tube(
        g, (x, y - 0.5, 13.5), (x, y + 0.5, 13.5), 3.4, "Timber", 10, color=DARK_WOOD
    )
    k.beam(
        g,
        (x + 1.0, y, 13.0),
        (x + 9.0, y, 0.0),
        0.8,
        0.8,
        "Timber",
        (0, 1, 0),
        color=DARK_WOOD,
    )


@rigid
def adit(g, x, y, yaw) -> None:
    """Mine entrance: spoil bank, timber portal, dark mouth."""
    with kit.frame(g, (x, y, 0.0), math.radians(yaw)):
        heap(g, 0, 4.0, 8.0, 5.5, "RoofSlate", SPOIL, sides=9)
        for sx in (-1.1, 1.1):
            k.box(g, (sx, -2.6, 0.9), (0.3, 0.3, 2.8), "Timber", color=DARK_WOOD)
        k.box(g, (0, -2.6, 2.45), (2.9, 0.4, 0.3), "Timber", color=DARK_WOOD)
        g.quad(
            (-1.6, -2.7, 0.0),
            (1.6, -2.7, 0.0),
            (1.6, -2.7, 3.2),
            (-1.6, -2.7, 3.2),
            "Window",
        )


@rigid
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


@rigid
def market_cross(g, x, y) -> None:
    """Stepped market cross."""
    k.box(g, (x, y, 0.1), (3.0, 3.0, 1.0), "Ashlar", color=STONE)
    k.box(g, (x, y, 0.85), (1.8, 1.8, 0.5), "Ashlar", color=STONE)
    k.box(g, (x, y, 2.9), (0.35, 0.35, 3.6), "Ashlar", color=STONE)
    k.box(g, (x, y, 4.2), (1.3, 0.3, 0.3), "Ashlar", color=STONE)


@rigid
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
    """A longère, its rick and a fenced yard."""
    put(g, "longere", 511, -2, 3, s=1.35)
    put(g, "haystack", 512, 10, -6, s=2.2)
    run(g, [(-13, -10), (4, -10)], 2.0, 0.5, "Planks", WOOD)


def farm_2(g) -> None:
    """House, barn and ricks around a fenced yard."""
    put(g, "longere", 521, -9, 9, s=1.45)
    put(g, "barn", 522, 13, 4, 90, s=1.45)
    put(g, "haystack", 524, -12, -9, s=2.4)
    put(g, "haystack", 525, -3, -11, s=2.4)
    run(g, [(-20, -4), (-20, -16), (20, -16), (20, -8)], 2.2, 0.5, "Planks", WOOD)


def farm_3(g) -> None:
    """Courtyard farm: stone house, two great barns, dovecote, ricks, walled yard."""
    put(g, "stonehouse", 531, -15, 14, s=1.7)
    put(g, "barn", 532, 13, 15, s=1.8)
    put(g, "barn", 533, 24, -8, 90, s=1.5)
    piece(g, dovecote, -24, -8, 0, 1.7)
    for n, (x, y) in enumerate([(-10, -14), (0, -17), (9, -13)]):
        put(g, "haystack", 537 + n, x, y, s=2.6)
    run(
        g,
        [(-6, -26), (-32, -26), (-32, 28), (34, 28), (34, -26), (8, -26)],
        3.0,
        1.0,
        "Rubble",
        STONE,
    )


def mill_1(g) -> None:
    """Post windmill on its mound."""
    heap(g, 0, 0, 9.0, 2.2, "Plaster", EARTH, sides=9)
    put(g, "windmill", 611, 0, 0, s=1.6)


def mill_2(g) -> None:
    """Windmill and a water mill with its wheel."""
    heap(g, -11, 7, 9.0, 2.2, "Plaster", EARTH, sides=9)
    put(g, "windmill", 621, -11, 7, s=1.6)
    put(g, "watermill", 622, 11, -1, s=1.7)
    k.box(g, (11.0, -15.0, -0.3), (5.0, 8.0, 1.6), "Canvas", color=BRINE)


def mill_3(g) -> None:
    """Mill hamlet: great windmill, two wheels, miller's house and store."""
    heap(g, -18, 12, 11.0, 2.6, "Plaster", EARTH, sides=9)
    put(g, "windmill", 631, -18, 12, s=2.0)
    put(g, "watermill", 632, 8, -6, s=1.9)
    put(g, "watermill", 633, 27, -6, s=1.6)
    put(g, "stonehouse", 634, 14, 16, s=1.5)
    put(g, "barn", 635, -20, -12, s=1.3)
    k.box(g, (16.0, -21.0, -0.3), (36.0, 7.0, 1.6), "Canvas", color=BRINE)


@rigid
def cask(g, x, y) -> None:
    """Great cask lying on its side."""
    k.tube(g, (x - 1.6, y, 1.5), (x + 1.6, y, 1.5), 1.5, "Planks", 8, color=DARK_WOOD)


def vineyard_1(g) -> None:
    """Press house and a plot of vines."""
    put(g, "stonehouse", 711, -9, 5, s=1.3)
    vines(g, 3, -12, 3, 22)
    cask(g, -9, -8)


def vineyard_2(g) -> None:
    """Press house, cellar and a larger plot."""
    put(g, "stonehouse", 721, -13, 9, s=1.5)
    put(g, "barn", 722, -13, -9, s=1.1)
    vines(g, 2, -16, 4, 32)
    cask(g, -1.5, -13)


def vineyard_3(g) -> None:
    """Walled clos: manor, press house, two plots."""
    put(g, "manor", 731, 3, 20, s=1.5)
    put(g, "barn", 733, 0, -14, 90, s=1.3)
    vines(g, -27, -26, 4, 30)
    vines(g, 11, -26, 4, 30)
    cask(g, -5, 2)
    cask(g, 1, 2)
    run(
        g,
        [(-6, -31), (-33, -31), (-33, 32), (33, 32), (33, -31), (6, -31)],
        2.8,
        1.0,
        "Rubble",
        STONE,
    )


def mine_1(g) -> None:
    """Spoil bank with its adit and a headframe."""
    piece(g, adit, -3, 4, 0, 1.6)
    piece(g, headframe, 11, -4, 0, 1.5)
    heap(g, -12, -7, 3.6, 2.6, "RoofSlate", ORE)


def mine_2(g) -> None:
    """Adit, tall headframe, horse whim and ore heaps."""
    piece(g, adit, -12, 8, 0, 1.8)
    piece(g, headframe, 10, 6, 0, 2.2)
    piece(g, whim, 14, -11, 0, 1.3)
    for x, y, r in [(-8, -10, 4.4), (-16, -6, 3.4)]:
        heap(g, x, y, r, r * 0.75, "RoofSlate", ORE)


def mine_3(g) -> None:
    """Mining works: two adits, great headframe, whim, furnace hall with its stacks."""
    piece(g, adit, -22, 14, 8, 2.0)
    piece(g, adit, 4, 20, -6, 1.6)
    piece(g, headframe, 24, 12, 0, 2.6)
    piece(g, whim, -6, -8, 0, 1.5)
    put(g, "barn", 831, 22, -14, s=1.5)
    piece(g, chimney_stack, 12, -8, 0, 2.0, 12.0)
    piece(g, chimney_stack, 33, -8, 0, 2.0, 10.0)
    for x, y, r in [(-26, -12, 5.4), (-18, -18, 4.0), (-32, -4, 3.6)]:
        heap(g, x, y, r, r * 0.75, "RoofSlate", ORE if r < 5.0 else SPOIL)


def pans(g, x0, y0, columns, rows, side) -> None:
    """Chequer of evaporation pans (white salt, pale brine) inside thick clay bunds."""
    for row in range(rows):
        for i in range(columns):
            x = x0 + (i + 0.5) * side
            y = y0 + (row + 0.5) * side
            k.box(g, (x, y, 0.1), (side, side, 1.8), "Rubble", color=CLAY)
            inner = side * 0.4
            salted = (i + row) % 2 == 0
            g.quad(
                (x - inner, y - inner, 1.05),
                (x + inner, y - inner, 1.05),
                (x + inner, y + inner, 1.05),
                (x - inner, y + inner, 1.05),
                "Plaster" if salted else "Canvas",
                color=SALT if salted else BRINE,
                occlude=False,
            )


def saltworks_1(g) -> None:
    """Four pans, a boiling hut and a heap of salt."""
    pans(g, -11, -13, 2, 2, 11)
    put(g, "cottage", 911, -4, 14, s=1.4)
    heap(g, 9, 13, 4.0, 5.0, "Plaster", SALT)


def saltworks_2(g) -> None:
    """Six pans, boiling house with its stack, heaps of salt."""
    pans(g, -18, -17, 3, 2, 12)
    put(g, "longere", 921, -6, 14, s=1.4)
    piece(g, chimney_stack, 4, 17, 0, 1.8, 9.0)
    heap(g, 15, 13, 4.6, 5.6, "Plaster", SALT)
    heap(g, 22, 9, 3.4, 4.2, "Plaster", SALT)


def saltworks_3(g) -> None:
    """Great saltworks: a dozen pans, two boiling houses, salt store, heaps."""
    pans(g, -26, -27, 4, 3, 13)
    put(g, "longere", 931, -18, 20, s=1.5)
    piece(g, chimney_stack, -6, 23, 0, 2.0, 10.0)
    put(g, "barn", 933, 12, 21, s=1.5)
    for x, y, r in [(28, 18, 5.2), (31, 8, 4.0), (-31, 16, 3.6)]:
        heap(g, x, y, r, r * 1.2, "Plaster", SALT)


def _cloister(g, x, y, side, height=6.5) -> None:
    """Three ranges around a garth, south of the church."""
    wing(g, x - side / 2, y, 90, side, 7.5, height)
    wing(g, x + side / 2, y, 90, side, 7.5, height)
    wing(g, x, y - side / 2, 0, side + 7.5, 7.5, height)


def abbey_1(g) -> None:
    """Priory: a chapel with its bell tower and one range of cells."""
    put(g, "church", 1011, 0, 4, s=0.95)
    wing(g, 3, -8, 0, 18, 6.5, 5.0)


def abbey_2(g) -> None:
    """Abbey: church, cloister and a precinct wall."""
    put(g, "church", 1021, 0, 12, s=1.25)
    _cloister(g, 2, -4, 20.0)
    run(
        g,
        [(-4, -22), (-24, -22), (-24, 22), (24, 22), (24, -22), (8, -22)],
        3.0,
        1.0,
        "Rubble",
        STONE,
    )


def abbey_3(g) -> None:
    """Great abbey: large church, cloister, guest house, tithe barn, precinct wall."""
    put(g, "church", 1031, 0, 16, s=1.65)
    _cloister(g, 4, -6, 24.0, 7.5)
    put(g, "manor", 1032, -24, -14, 90, s=1.0)
    put(g, "barn", 1035, 28, -14, 90, s=1.4)
    run(
        g,
        [(-6, -32), (-36, -32), (-36, 30), (38, 30), (38, -32), (10, -32)],
        3.4,
        1.2,
        "Rubble",
        STONE,
    )


def market_1(g) -> None:
    """Market hall and two stalls."""
    put(g, "hall", 1111, 0, 4, s=0.9)
    for n, x in enumerate((-6, 6)):
        piece(g, stall, x, -8, 0, 2.2, CLOTHS[n])


def market_2(g) -> None:
    """Market hall, cross and a row of stalls."""
    put(g, "hall", 1121, 0, 9, s=1.3)
    piece(g, market_cross, 0, -6, 0, 1.8)
    for n, x in enumerate((-17, -9, 9, 17)):
        piece(g, stall, x, -11, 0, 2.3, CLOTHS[n % len(CLOTHS)])


def market_3(g) -> None:
    """Fair ground: great hall, guild house, rows of stalls and tents."""
    put(g, "hall", 1131, -9, 16, s=1.7)
    put(g, "stonehouse", 1132, 23, 17, s=1.6)
    piece(g, market_cross, 0, -4, 0, 2.2)
    for n, x in enumerate((-26, -17, -8, 8, 17, 26)):
        piece(g, stall, x, -13, 0, 2.4, CLOTHS[n % len(CLOTHS)])
    for n, x in enumerate((-22, -8, 8, 22)):
        tent(g, x, -25, 0, 10.0, 8.0, 6.0, CLOTHS[(n + 1) % len(CLOTHS)])


@rigid
def quay(g, width) -> None:
    """Stone quay along X at y = 0, water in front (-Y)."""
    k.box(g, (0, 1.0, 0.0), (width, 6.0, 4.0), "Masonry", color=STONE)
    k.box(g, (0, -8.0, -0.6), (width, 12.0, 1.6), "Canvas", color=BRINE)


def port_1(g) -> None:
    """Quay, crane, store and a cog."""
    quay(g, 28.0)
    piece(g, crane, -8, 2, 0, 1.9)
    put(g, "barn", 1211, 5, 12, s=1.2)
    boat(g, 3, -8, 0, 16.0, 5.0, mast=14.0)


def port_2(g) -> None:
    """Quay, jetty, crane, warehouses, a cog and boats."""
    quay(g, 42.0)
    jetty(g, 15, 0, -18, 5.0)
    piece(g, crane, -4, 2, 0, 2.1)
    put(g, "barn", 1221, -12, 15, s=1.5)
    put(g, "stonehouse", 1222, 12, 14, s=1.4)
    boat(g, -6, -9, 0, 19.0, 5.6, mast=17.0)
    boat(g, 23, -6, 80, 9.0, 3.0)


def port_3(g) -> None:
    """Harbour: long quay, two jetties, two cranes, hall, warehouses, two cogs."""
    quay(g, 60.0)
    jetty(g, -8, 0, -20, 5.0)
    jetty(g, 22, 0, -20, 5.0)
    piece(g, crane, -22, 2, 0, 2.2)
    piece(g, crane, 8, 2, 0, 2.2)
    put(g, "hall", 1231, 0, 20, s=1.5)
    put(g, "barn", 1232, -24, 17, s=1.5)
    put(g, "barn", 1233, 25, 17, s=1.4)
    boat(g, -20, -10, 0, 20.0, 6.0, mast=18.0)
    boat(g, 7, -11, 0, 19.0, 5.6, mast=17.0)


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


# --- settlement signs -----------------------------------------------------------------------


def sign_village(g) -> None:
    """Village: a chapel and a handful of thatched houses."""
    put(g, "church", 1311, 0, 6, s=0.7)
    houses(g, 1312, 7, 30.0, [(0, 6, 13.0)], roofs=("Thatch", "Thatch", "RoofTile"))


def sign_town(g) -> None:
    """Open town: church and tight tiled roofs."""
    put(g, "church", 1321, 2, 8, s=1.05)
    houses(
        g, 1322, 20, 44.0, [(2, 8, 17.0)], roofs=("RoofTile", "RoofTile", "RoofFlat")
    )


def sign_walled(g) -> None:
    """Walled town: stone wall, towers, gate, church and tiled roofs."""
    put(g, "church", 1331, 2, 8, s=1.05)
    houses(
        g, 1332, 18, 38.0, [(2, 8, 17.0)], roofs=("RoofTile", "RoofTile", "RoofFlat")
    )
    ring_wall(g, 48.0, 8.0, 3.0, 6, 4.5, 13.0)


def sign_city(g) -> None:
    """City: wall, towers and gate, great church, keep, tight tiled and slated roofs."""
    put(g, "church", 1341, 6, 14, s=1.5)
    keep(g, -30, 22, 15.0, 26.0)
    houses(
        g,
        1342,
        30,
        50.0,
        [(6, 14, 25.0), (-30, 22, 13.0)],
        roofs=("RoofTile", "RoofTile", "RoofSlate"),
        size=1.1,
    )
    ring_wall(g, 62.0, 10.0, 3.6, 8, 5.5, 16.0)


def sign_castle(g) -> None:
    """Castle: square curtain, corner towers, gate and a tall keep."""
    half = 22.0
    run(
        g,
        rect(-half, -half, half, half),
        11.0,
        3.0,
        "Masonry",
        STONE,
        depth=1.5,
        closed=True,
    )
    for sx in (-1, 1):
        for sy in (-1, 1):
            tower(g, sx * half, sy * half, 5.5, 17.0)
    k.box(g, (0, -half, 7.5), (11.0, 6.0, 16.5), "Masonry", color=STONE)
    g.quad(
        (-2.5, -half - 3.1, 0.0),
        (2.5, -half - 3.1, 0.0),
        (2.5, -half - 3.1, 7.0),
        (-2.5, -half - 3.1, 7.0),
        "Window",
    )
    keep(g, 2, 6, 15.0, 28.0)
    house(g, -9, -8, 0, 13.0, 7.0, 6.0, "RoofSlate", "Ashlar")


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
# Settlement signs (ADR 0162): one model per kind, named ``sign_<kind>``.
SIGNS = {
    "village": sign_village,
    "town": sign_town,
    "walled": sign_walled,
    "city": sign_city,
    "castle": sign_castle,
}
SIGN_TRIANGLE_CAP = 6000


def build(family: str, level: int) -> tuple[Geometry, dict]:
    """Geometry and manifest entry of one model."""
    g = Geometry()
    _PIECES.clear()
    _STATE["root"] = g
    _STATE["depth"] = 0
    if family == "sign":
        SIGNS[level](g)
    else:
        MODELS[family][level - 1](g)
    unknown = sorted(set(g.polys) - set(ATLAS_LAYERS))
    if unknown:
        raise SystemExit(f"{family}_{level}: materials outside the atlas: {unknown}")
    if family == "sign":
        level = 0
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
        # Rigid pieces in Godot's frame (x, z = -y): centre x, centre z, then the plan box.
        "pieces": [
            [
                round((x0 + x1) / 2, 1),
                round(-(y0 + y1) / 2, 1),
                round(x0, 1),
                round(-y1, 1),
                round(x1, 1),
                round(-y0, 1),
            ]
            for x0, y0, x1, y1 in _PIECES
        ],
    }
    return g, info


def manifest() -> dict:
    """Manifest entries of every model (no Blender needed)."""
    entries = {
        f"{family}_{level}": build(family, level)[1]
        for family, recipes in MODELS.items()
        for level in range(1, len(recipes) + 1)
    }
    for kind in SIGNS:
        entries[f"sign_{kind}"] = build("sign", kind)[1]
    return entries


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
    for kind in SIGNS:
        name = f"sign_{kind}"
        kit_export.reset_scene()
        g, info = build("sign", kind)
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
            cap = TRIANGLE_CAPS.get(info["level"], SIGN_TRIANGLE_CAP)
            flag = "" if info["triangles"] <= cap else "  OVER"
            print(
                f"{name:14} {info['triangles']:5} tri  r {info['radius']:5} m  h {info['height']:5} m{flag}"
            )
    else:
        raise SystemExit("usage: -- export <out_dir> | stats")


if __name__ == "__main__":
    main()
