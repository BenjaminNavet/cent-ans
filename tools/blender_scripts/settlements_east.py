"""Eastern and southern settlement families of the campaign map (lot GC3, ADR 0158).

Imported by ``settlements.py``, which exports the models of :data:`MODELS` next to the western
kit. Five architecture families around 1340, five settlement kinds, two variants each, named
``<kind>_<family>_<a|b>``:

* ``med``: Italy, Christian Iberia, Midi, Dalmatia (pale stone, low tile roofs, campaniles);
* ``byz``: Greeks, Bulgarians, Serbs, Georgians, Armenians (banded walls, domes on drums);
* ``rus``: Rus', Novgorod, Ruthenia, Lithuania (log walls, tent roofs, onion and helmet domes);
* ``isl``: Maghreb, Arabs, Turks, al-Andalus (flat roofs, courtyard mosques, minarets);
* ``steppe``: Kipchaks and the Horde (mud brick, yurts, earth forts, kurgans).

The models are meant to be read from far away (stylised maquettes at a constant world size):
few but large elements, bold silhouettes, light and contrasted flat colours. Unlike the western
kit, houses are plain primitives, not ``building_kit`` houses; the new palette materials are not
atlas layers, so Godot keeps their flat colour. Same contract as the western kit: one joined
mesh, a ``Banner`` material, foundations below ``z = 0``, same ground footprint per kind.
"""

import math
import random
from functools import partial

import models as m

F = m.FOUNDATION
TAU = 2 * math.pi

FAMILIES = ("med", "byz", "rus", "isl", "steppe")
KINDS = ("city", "town", "castle", "abbey", "village")
VARIANTS = ("a", "b")

# name: (base colour (linear RGB), roughness, metallic). Names must stay out of the kit atlas
# layers (``kit_export.ATLAS_LAYERS``), otherwise Godot would texture them.
PALETTE = {
    "Whitewash": ((0.80, 0.77, 0.68), 0.9, 0.0),
    "Limestone": ((0.66, 0.59, 0.46), 0.88, 0.0),
    "Ochre": ((0.60, 0.40, 0.19), 0.92, 0.0),
    "MudBrick": ((0.50, 0.36, 0.22), 0.95, 0.0),
    "Brick": ((0.45, 0.17, 0.10), 0.88, 0.0),
    "TileOrange": ((0.62, 0.21, 0.07), 0.8, 0.0),
    "Lead": ((0.36, 0.39, 0.43), 0.5, 0.0),
    "Log": ((0.28, 0.18, 0.10), 0.9, 0.0),
    "Shingle": ((0.42, 0.41, 0.38), 0.85, 0.0),
    "CopperGreen": ((0.10, 0.40, 0.27), 0.5, 0.0),
    "Gilt": ((0.85, 0.58, 0.12), 0.4, 0.3),
    "Turquoise": ((0.05, 0.42, 0.48), 0.4, 0.0),
    "Felt": ((0.74, 0.70, 0.60), 0.95, 0.0),
    "Sand": ((0.50, 0.40, 0.25), 1.0, 0.0),
    "Rock": ((0.40, 0.36, 0.29), 0.95, 0.0),
    "Steppe": ((0.30, 0.31, 0.13), 1.0, 0.0),
    "Cypress": ((0.04, 0.10, 0.04), 0.95, 0.0),
    "Palm": ((0.08, 0.24, 0.06), 0.9, 0.0),
    # Same values as ``settlements.py`` (``setdefault`` keeps whichever came first).
    "Garden": ((0.12, 0.16, 0.05), 0.95, 0.0),
    "Field": ((0.26, 0.22, 0.10), 1.0, 0.0),
    "Earth": ((0.20, 0.17, 0.11), 1.0, 0.0),
}
for _name, _spec in PALETTE.items():
    m.PALETTE.setdefault(_name, _spec)


# --- Primitives ----------------------------------------------------------------------


def block(x, y, w, d, h, mat, angle=0.0, z=0.0, deep=True):
    """Box of footprint ``w`` x ``d`` standing on ``z``, with foundations when ``deep``."""
    depth = F if deep else 0.0
    return m.box(
        (w, d, h + depth), (x, y, z + (h - depth) / 2), mat, rotation=(0, 0, angle)
    )


def lathe(profile, mat, location, segments=8, phase=0.0):
    """Surface of revolution from ``(radius, z)`` pairs, bottom to top (zero radius = apex)."""
    verts, rings = [], []
    for radius, z in profile:
        if radius <= 1e-6:
            rings.append([len(verts)])
            verts.append((0.0, 0.0, z))
            continue
        rings.append(list(range(len(verts), len(verts) + segments)))
        for k in range(segments):
            a = phase + TAU * k / segments
            verts.append((radius * math.cos(a), radius * math.sin(a), z))
    faces = []
    for lower, upper in zip(rings, rings[1:], strict=False):
        for k in range(segments):
            k1 = (k + 1) % segments
            if len(lower) > 1 and len(upper) > 1:
                faces.append((lower[k], lower[k1], upper[k1], upper[k]))
            elif len(lower) > 1:
                faces.append((lower[k], lower[k1], upper[0]))
            elif len(upper) > 1:
                faces.append((lower[0], upper[k1], upper[k]))
    return m.mesh_object("lathe", verts, faces, mat, location)


# Unit profiles (radius 1 at the springing), scaled by :func:`cupola`.
DOME_PROFILES = {
    "round": ((1.0, 0.0), (0.87, 0.5), (0.5, 0.87), (0.0, 1.0)),
    "shallow": ((1.0, 0.0), (0.85, 0.3), (0.45, 0.52), (0.0, 0.6)),
    "pointed": ((1.0, 0.0), (0.96, 0.45), (0.72, 0.95), (0.36, 1.3), (0.0, 1.55)),
    "helmet": ((1.05, 0.0), (0.98, 0.35), (0.7, 0.75), (0.28, 1.05), (0.0, 1.4)),
    "onion": (
        (0.6, 0.0),
        (1.08, 0.45),
        (1.0, 0.85),
        (0.52, 1.3),
        (0.14, 1.65),
        (0.0, 2.1),
    ),
}


def cupola(x, y, z, radius, mat, shape="round", segments=8, finial=None):
    """Dome of the given ``shape`` springing at ``z``; ``finial`` adds a spike of that material."""
    profile = [(r * radius, h * radius) for r, h in DOME_PROFILES[shape]]
    parts = [lathe(profile, mat, (x, y, z), segments)]
    if finial:
        top = z + profile[-1][1]
        spike = radius * 0.5
        parts.append(
            m.cylinder(radius * 0.05, spike, (x, y, top + spike / 2), finial, 4)
        )
    return parts


def drum_dome(x, y, z, radius, drum, wall, mat, shape="round", segments=8, finial=None):
    """Cylindrical drum of height ``drum`` standing on ``z`` and its dome."""
    parts = [
        m.cylinder(radius, drum + 0.02, (x, y, z + drum / 2 - 0.01), wall, segments)
    ]
    if shape == "cone":
        height = radius * 1.7
        parts.append(
            m.cone(radius * 1.12, height, (x, y, z + drum + height / 2), mat, segments)
        )
        return parts
    return parts + cupola(x, y, z + drum, radius * 1.04, mat, shape, segments, finial)


def flag(x, y, z, height=0.32):
    """Flag pole with a controller-coloured banner (``Banner`` material)."""
    return [
        m.cylinder(0.007, height, (x, y, z + height / 2), "Wood", 4),
        m.box((0.006, 0.16, 0.1), (x, y + 0.08, z + height - 0.06), "Banner"),
    ]


def mound(x, y, radius, height, mat="Earth", top=0.62, sides=12):
    """Hill (frustum) of ``height`` whose flat top has radius ``radius * top``."""
    return [
        m.cone(
            radius,
            height + F,
            (x, y, (height - F) / 2),
            mat,
            sides,
            radius_top=radius * top,
        )
    ]


def mound_height(distance, radius, height, top=0.62):
    """Ground height of a :func:`mound` at ``distance`` from its axis."""
    if distance <= radius * top:
        return height
    if distance >= radius:
        return 0.0
    return height * (radius - distance) / (radius * (1.0 - top))


def tower(x, y, side, height, mat, top="cap", roof=None, angle=0.0, z=0.0, taper=1.0):
    """Square tower. ``top``: ``cap`` (jutting parapet), ``crenel`` (parapet and corner merlons),
    ``pyramid`` (low tiled roof), ``tent`` (tall wooden roof) or ``flat``; ``taper`` < 1 batters
    the walls (pisé towers).
    """  # noqa: D205
    if taper < 1.0:
        half = side * math.sqrt(0.5)
        parts = [
            m.cone(
                half,
                height + F,
                (x, y, z + (height - F) / 2),
                mat,
                4,
                radius_top=half * taper,
                rotation=(0, 0, angle + math.pi / 4),
            )
        ]
    else:
        parts = [block(x, y, side, side, height, mat, angle, z)]
    crown = side * taper
    zt = z + height
    if top in ("cap", "crenel"):
        parts.append(
            block(x, y, crown * 1.18, crown * 1.18, 0.045, mat, angle, zt, deep=False)
        )
    if top == "crenel":
        ca, sa = math.cos(angle), math.sin(angle)
        off = crown * 0.43
        for sx in (-1, 1):
            for sy in (-1, 1):
                dx, dy = sx * off, sy * off
                parts.append(
                    block(
                        x + dx * ca - dy * sa,
                        y + dx * sa + dy * ca,
                        crown * 0.32,
                        crown * 0.32,
                        0.05,
                        mat,
                        angle,
                        zt + 0.045,
                        deep=False,
                    )
                )
    if top in ("pyramid", "tent"):
        rise = crown * (0.45 if top == "pyramid" else 1.35)
        parts.append(
            m.cone(
                crown * 0.82,
                rise,
                (x, y, zt + rise / 2),
                roof or "TileOrange",
                4,
                rotation=(0, 0, angle + math.pi / 4),
            )
        )
    return parts


def banded(x, y, w, d, h, bands, angle=0.0, z=0.0):
    """Wall block made of horizontal courses ``[(material, fraction), ...]`` (bottom to top)."""
    parts = []
    for index, (mat, fraction) in enumerate(bands):
        parts.append(block(x, y, w, d, h * fraction, mat, angle, z, deep=index == 0))
        z += h * fraction
    return parts


def merlon_row(x0, y0, x1, y1, z, mat, size=0.05, spacing=0.16, thickness=0.06):
    """Merlons of material ``mat`` along a wall top (``models.merlons`` is stone only)."""
    length = math.hypot(x1 - x0, y1 - y0)
    count = max(1, int(length / spacing))
    angle = math.atan2(y1 - y0, x1 - x0)
    parts = []
    for index in range(count):
        t = (index + 0.5) / count
        parts.append(
            m.box(
                (size, thickness, size),
                (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z + size / 2),
                mat,
                rotation=(0, 0, angle),
            )
        )
    return parts


def stakes(x0, y0, x1, y1, z, mat="Log", spacing=0.1, size=0.035):
    """Pointed stake tips along a palisade top."""
    length = math.hypot(x1 - x0, y1 - y0)
    count = max(1, int(length / spacing))
    return [
        m.cone(
            size,
            size * 2.2,
            (
                x0 + (x1 - x0) * (i + 0.5) / count,
                y0 + (y1 - y0) * (i + 0.5) / count,
                z + size * 1.1,
            ),
            mat,
            4,
        )
        for i in range(count)
    ]


def curtain(
    points,
    height,
    mat,
    tower_at=None,
    gates=(),
    gate_at=None,
    thickness=0.07,
    bands=None,
    top=None,
    z=0.0,
):
    """Closed enceinte along ``points``.

    ``tower_at(x, y)`` builds a tower at each corner, ``gate_at(x, y, angle)`` a gatehouse in
    the middle of the sides listed in ``gates``. ``bands`` makes banded masonry, ``top`` is
    ``"merlons"`` or ``"stakes"`` (palisade).
    """
    parts = []
    count = len(points)
    for index in range(count):
        x0, y0 = points[index]
        x1, y1 = points[(index + 1) % count]
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        length = math.hypot(x1 - x0, y1 - y0)
        angle = math.atan2(y1 - y0, x1 - x0)
        if bands:
            parts += banded(mx, my, length, thickness, height, bands, angle, z)
        else:
            parts.append(block(mx, my, length, thickness, height, mat, angle, z))
        if top == "merlons":
            parts += merlon_row(x0, y0, x1, y1, z + height, mat)
        elif top == "stakes":
            parts += stakes(x0, y0, x1, y1, z + height, mat)
        if tower_at is not None:
            parts += tower_at(x0, y0)
        if gate_at is not None and index in gates:
            parts += gate_at(mx, my, angle)
    return parts


def gatehouse(x, y, angle, height, mat, roof=None, width=0.22, z=0.0):
    """Gate tower across a wall: parapet (or hipped ``roof``) and a dark doorway."""
    parts = [block(x, y, width, 0.16, height, mat, angle, z)]
    if roof:
        parts.append(m.hip_roof(width, 0.16, 0.08, (x, y, z + height), roof, angle))
    else:
        parts.append(
            block(x, y, width * 1.12, 0.185, 0.04, mat, angle, z + height, deep=False)
        )
    parts.append(
        m.box(
            (width * 0.32, 0.175, height * 0.45),
            (x, y, z + height * 0.225),
            "Wood",
            rotation=(0, 0, angle),
        )
    )
    return parts


def gable_house(x, y, w, d, h, angle, wall, roof, pitch=0.32, z=0.0):
    """Plain house: wall block and a gable roof of rise ``d * pitch`` (ridge along ``w``)."""
    return [
        block(x, y, w, d, h, wall, angle, z),
        m.gable_roof(w, d, d * pitch, (x, y, z + h), roof, angle, overhang=0.015),
    ]


def flat_house(x, y, w, d, h, angle, wall, z=0.0, upper=None):
    """Flat-roofed cubic house; ``upper`` adds a roof room of that material on one corner."""
    parts = [block(x, y, w, d, h, wall, angle, z)]
    if upper:
        ca, sa = math.cos(angle), math.sin(angle)
        dx, dy = w * 0.22, d * 0.2
        parts.append(
            block(
                x + dx * ca - dy * sa,
                y + dx * sa + dy * ca,
                w * 0.5,
                d * 0.55,
                h * 0.55,
                upper,
                angle,
                z + h,
                deep=False,
            )
        )
    return parts


def yurt(x, y, radius, mat="Felt", door=0.0):
    """Felt tent: round wall, conical roof with a smoke ring, door facing ``door``."""
    wall_h = radius * 0.75
    profile = [
        (radius, -F * 0.5),
        (radius, wall_h),
        (radius * 0.28, wall_h + radius * 0.55),
        (0.0, wall_h + radius * 0.6),
    ]
    return [
        lathe(profile, mat, (x, y, 0.0), 8),
        m.box(
            (radius * 0.2, radius * 0.5, wall_h * 0.85),
            (
                x + math.cos(door) * radius * 0.95,
                y + math.sin(door) * radius * 0.95,
                wall_h * 0.42,
            ),
            "Wood",
            rotation=(0, 0, door),
        ),
    ]


def cypress(x, y, height=0.3, z=0.0):
    """Cypress: dark slender cone."""
    return [
        m.cone(
            height * 0.16, height + 0.05, (x, y, z + height / 2 - 0.025), "Cypress", 6
        )
    ]


def palm(x, y, height=0.3, z=0.0):
    """Date palm: thin trunk and a flat crown."""
    return [
        m.cylinder(0.012, height + 0.1, (x, y, z + height / 2 - 0.05), "Log", 4),
        m.cone(0.11, 0.07, (x, y, z + height + 0.02), "Palm", 6),
    ]


def plots(x, y, angle, count, length, width, mats=("Field", "Garden", "Earth")):
    """Flat cultivated strips slightly above the ground (gardens, vineyards, fields)."""
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


def scatter(
    rng,
    count,
    radius,
    builder,
    keep_out=(),
    inside=None,
    size=(0.2, 0.28),
    gap=1.02,
    inner=0.0,
    align=None,
):
    """Place up to ``count`` buildings in a disc, without overlaps.

    ``builder(x, y, w, d, angle, rng)`` returns the parts of one building of footprint
    ``w`` x ``d``. ``keep_out`` lists ``(x, y, radius)`` circles to avoid, ``inside`` a wall
    polygon, ``inner`` a hole radius. Buildings face the centre unless ``align`` gives a fixed
    street direction.
    """
    parts, placed = [], []
    tries = 0
    while len(placed) < count and tries < count * 80:
        tries += 1
        r = math.sqrt(rng.uniform((inner / radius) ** 2, 1.0)) * radius
        a = rng.uniform(0.0, TAU)
        x, y = r * math.cos(a), r * math.sin(a)
        w = rng.uniform(*size)
        d = w * rng.uniform(0.62, 0.85)
        reach = w * 0.58
        if inside is not None and not all(
            m.point_in_polygon(x + dx * reach, y + dy * reach, inside)
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
        ):
            continue
        if any(math.hypot(x - kx, y - ky) < kr + reach for kx, ky, kr in keep_out):
            continue
        if any(
            math.hypot(x - px, y - py) < (reach + pr) * gap for px, py, pr in placed
        ):
            continue
        placed.append((x, y, reach))
        angle = (a + math.pi / 2 if align is None else align) + rng.uniform(-0.12, 0.12)
        parts += builder(x, y, w, d, angle, rng)
    return parts


def shrink(points, factor):
    """Polygon scaled about the origin (inner side of a wall)."""
    return [(x * factor, y * factor) for x, y in points]


def local_frame(x, y, angle, s=1.0):
    """Return ``at(dx, dy)``: local offsets (scaled by ``s``) to model coordinates."""
    ca, sa = math.cos(angle), math.sin(angle)

    def at(dx, dy):
        return x + (dx * ca - dy * sa) * s, y + (dx * sa + dy * ca) * s

    return at


def placeholder(radius):
    """Skeleton stand-in: ground disc and banner."""
    return m.ground_patch(radius) + flag(0.0, 0.0, 0.0)


# --- med: Italy, Christian Iberia, Midi, Dalmatia ------------------------------------

MED_WALLS = ("Limestone", "Whitewash", "Ochre", "Limestone")


def med_house(x, y, w, d, angle, rng, z=0.0):
    """Stone or rendered house under a low orange tile roof."""
    return gable_house(
        x,
        y,
        w,
        d,
        rng.uniform(0.11, 0.17),
        angle,
        rng.choice(MED_WALLS),
        "TileOrange",
        z=z,
    )


def campanile(x, y, side, height, angle=0.0, z=0.0, mat="Limestone"):
    """Bell tower: tall shaft, jutting belfry stage, low pyramidal tile roof."""
    parts = tower(x, y, side, height, mat, "cap", angle=angle, z=z)
    return parts + tower(
        x,
        y,
        side * 0.82,
        side * 0.9,
        mat,
        "pyramid",
        "TileOrange",
        angle,
        z + height + 0.045,
    )


def round_campanile(x, y, radius, height, z=0.0):
    """Round arcaded bell tower (Pisan type): white drum with ring cornices and a lantern."""
    parts = [
        m.cylinder(radius, height + F, (x, y, z + (height - F) / 2), "Whitewash", 10)
    ]
    for level in (0.34, 0.67, 1.0):
        parts.append(
            m.cylinder(radius * 1.16, 0.03, (x, y, z + height * level), "Limestone", 10)
        )
    parts.append(
        m.cylinder(
            radius * 0.72,
            radius * 1.3,
            (x, y, z + height + radius * 0.65),
            "Whitewash",
            8,
        )
    )
    return parts


def med_church(x, y, angle, s=1.0, tower_side=1, bell_gable=False, z=0.0):
    """Romanesque church: nave, apse, and a campanile or (``bell_gable``) a wall belfry."""
    at = local_frame(x, y, angle, s)
    nx, ny = at(0.0, 0.0)
    parts = gable_house(
        nx, ny, 0.6 * s, 0.24 * s, 0.24 * s, angle, "Limestone", "TileOrange", 0.3, z
    )
    ax, ay = at(0.33, 0.0)
    parts.append(
        m.cylinder(
            0.1 * s, 0.2 * s + F, (ax, ay, z + (0.2 * s - F) / 2), "Limestone", 8
        )
    )
    parts.append(m.cone(0.115 * s, 0.09 * s, (ax, ay, z + 0.245 * s), "TileOrange", 8))
    if bell_gable:
        fx, fy = at(-0.31, 0.0)
        parts.append(block(fx, fy, 0.05 * s, 0.3 * s, 0.44 * s, "Limestone", angle, z))
        parts.append(
            block(
                fx,
                fy,
                0.05 * s,
                0.16 * s,
                0.1 * s,
                "Limestone",
                angle,
                z + 0.44 * s,
                False,
            )
        )
    else:
        cx, cy = at(-0.12, tower_side * 0.22)
        parts += campanile(cx, cy, 0.14 * s, 0.56 * s, angle, z)
    return parts


def duomo(x, y, angle, s=1.0, dome=True):
    """Cathedral: marble nave and aisles, screen facade, transept and dome (or lantern)."""
    at = local_frame(x, y, angle, s)
    nx, ny = at(-0.1, 0.0)
    parts = gable_house(
        nx, ny, 1.0 * s, 0.3 * s, 0.4 * s, angle, "Whitewash", "TileOrange", 0.3
    )
    for side in (-1, 1):
        sx, sy = at(-0.1, side * 0.24)
        parts += gable_house(
            sx, sy, 0.96 * s, 0.2 * s, 0.22 * s, angle, "Whitewash", "TileOrange", 0.18
        )
    fx, fy = at(-0.62, 0.0)
    parts.append(block(fx, fy, 0.05 * s, 0.7 * s, 0.34 * s, "Whitewash", angle))
    parts.append(
        block(fx, fy, 0.05 * s, 0.34 * s, 0.24 * s, "Whitewash", angle, 0.34 * s, False)
    )
    tx, ty = at(0.42, 0.0)
    parts += gable_house(
        tx,
        ty,
        0.9 * s,
        0.3 * s,
        0.38 * s,
        angle + math.pi / 2,
        "Whitewash",
        "TileOrange",
        0.3,
    )
    ax, ay = at(0.66, 0.0)
    parts.append(
        m.cylinder(0.15 * s, 0.34 * s + F, (ax, ay, (0.34 * s - F) / 2), "Whitewash", 8)
    )
    parts.append(m.cone(0.17 * s, 0.12 * s, (ax, ay, 0.4 * s), "TileOrange", 8))
    if dome:
        parts += drum_dome(
            tx,
            ty,
            0.38 * s,
            0.24 * s,
            0.2 * s,
            "Whitewash",
            "TileOrange",
            "pointed",
            8,
            "Gilt",
        )
    else:
        parts += tower(
            tx,
            ty,
            0.24 * s,
            0.14 * s,
            "Whitewash",
            "pyramid",
            "TileOrange",
            angle,
            0.44 * s,
        )
    return parts


def palazzo(x, y, angle, s=1.0):
    """Communal palace: crenellated block and a slender tower flying the banner."""
    at = local_frame(x, y, angle, s)
    parts = [block(x, y, 0.5 * s, 0.32 * s, 0.3 * s, "Limestone", angle)]
    corners = [at(-0.25, -0.16), at(0.25, -0.16), at(0.25, 0.16), at(-0.25, 0.16)]
    for index, (x0, y0) in enumerate(corners):
        x1, y1 = corners[(index + 1) % 4]
        parts += merlon_row(x0, y0, x1, y1, 0.3 * s, "Limestone", 0.05, 0.13, 0.04)
    tx, ty = at(-0.1, 0.05)
    parts += tower(tx, ty, 0.13 * s, 0.82 * s, "Limestone", "crenel", angle=angle)
    return parts + flag(tx, ty, 0.82 * s + 0.09)


def med_walls(points, height, gates, mat="Limestone"):
    """Pale curtain wall with square towers and gate towers."""
    return curtain(
        points,
        height,
        mat,
        tower_at=lambda x, y: tower(x, y, 0.14, height * 1.5, mat, "cap"),
        gates=gates,
        gate_at=lambda x, y, a: gatehouse(x, y, a, height * 1.7, mat),
    )


def build_med_city(variant):
    """Commune: pale walls, duomo, campanile, palazzo and tower houses."""
    first = variant == "a"
    rng = random.Random(1340 if first else 1282)
    radius = 1.38 if first else 1.32
    points = m.ring_points(radius, 11 if first else 9, rng, 0.08)
    gates = (0, 4, 8) if first else (1, 5)
    parts = m.ground_patch(radius * 0.97)
    parts += med_walls(points, 0.2, gates)
    duomo_at = (-0.15, 0.2, 0.2) if first else (0.1, -0.1, -0.5)
    parts += duomo(*duomo_at, 0.85, dome=first)
    at = local_frame(*duomo_at, 0.85)
    bx, by = at(-0.45, 0.62)
    parts += (
        campanile(bx, by, 0.17, 0.85, duomo_at[2])
        if first
        else round_campanile(bx, by, 0.11, 0.7)
    )
    px, py = (0.55, -0.55) if first else (-0.6, 0.5)
    parts += palazzo(px, py, 0.4, 1.0)
    keep_out = [(duomo_at[0], duomo_at[1], 0.62), (bx, by, 0.16), (px, py, 0.34)]
    for _ in range(5 if first else 3):
        ang, r = rng.uniform(0, TAU), rng.uniform(0.45, 0.95) * radius
        tx, ty = r * math.cos(ang), r * math.sin(ang)
        if any(math.hypot(tx - kx, ty - ky) < kr + 0.1 for kx, ky, kr in keep_out):
            continue
        parts += tower(
            tx, ty, 0.11, rng.uniform(0.5, 0.7), "Limestone", "cap", angle=ang
        )
        keep_out.append((tx, ty, 0.1))
    parts += scatter(
        rng, 60, radius * 0.95, med_house, keep_out, shrink(points, 0.9), (0.2, 0.3)
    )
    # Borgo outside the first gate.
    gx, gy = (
        (points[gates[0]][0] + points[gates[0] + 1][0]) / 2,
        (points[gates[0]][1] + points[gates[0] + 1][1]) / 2,
    )
    base = math.atan2(gy, gx)
    for k in range(6):
        ang = base + (0.14 if k % 2 else -0.14) + rng.uniform(-0.04, 0.04)
        t = (1.22 + 0.13 * (k // 2)) * math.hypot(gx, gy)
        parts += med_house(
            t * math.cos(ang),
            t * math.sin(ang),
            0.22,
            0.15,
            base + rng.uniform(-0.2, 0.2),
            rng,
        )
    return parts


def build_med_town(variant):
    """Walled hill town with its parish church and a few tower houses."""
    first = variant == "a"
    rng = random.Random(1215 if first else 1248)
    points = m.ring_points(0.9, 8 if first else 7, rng, 0.1, phase=0.2)
    parts = m.ground_patch(0.88)
    parts += med_walls(points, 0.17, (0, 4) if first else (2, 5))
    parts += med_church(0.05, 0.1, 0.5, 1.0, bell_gable=not first)
    keep_out = [(0.05, 0.1, 0.42)]
    if first:
        for tx, ty, h in ((-0.42, -0.3, 0.62), (0.4, -0.38, 0.5), (-0.3, 0.5, 0.55)):
            parts += tower(tx, ty, 0.11, h, "Limestone", "cap", angle=0.3)
            keep_out.append((tx, ty, 0.1))
        parts += flag(-0.42, -0.3, 0.62 + 0.045)
    else:
        parts += tower(-0.45, -0.35, 0.22, 0.55, "Limestone", "crenel", angle=0.2)
        parts += flag(-0.45, -0.35, 0.55 + 0.09)
        keep_out.append((-0.45, -0.35, 0.2))
    parts += scatter(
        rng, 24, 0.82, med_house, keep_out, shrink(points, 0.88), (0.19, 0.27)
    )
    return parts


def build_med_castle(variant):
    """Castello with four corner towers (a) or rocca on a rock (b)."""
    rng = random.Random(1266)
    if variant == "a":
        parts = m.ground_patch(0.95, "Earth")
        half = 0.42
        square = [(-half, -half), (half, -half), (half, half), (-half, half)]
        parts += curtain(
            square,
            0.3,
            "Limestone",
            tower_at=lambda x, y: tower(x, y, 0.2, 0.46, "Limestone", "crenel"),
            gates=(0,),
            gate_at=lambda x, y, a: gatehouse(x, y, a, 0.4, "Limestone"),
            top="merlons",
        )
        parts += tower(0.1, 0.1, 0.28, 0.85, "Limestone", "crenel")
        parts += flag(0.1, 0.1, 0.85 + 0.09)
        parts += gable_house(
            -0.16, -0.12, 0.4, 0.16, 0.2, 0.0, "Limestone", "TileOrange"
        )
        for k in range(6):
            ang = -2.2 + k * 0.32 + rng.uniform(-0.05, 0.05)
            parts += med_house(
                0.8 * math.cos(ang),
                0.8 * math.sin(ang),
                0.2,
                0.14,
                ang + math.pi / 2,
                rng,
            )
        return parts
    height = 0.3
    parts = m.ground_patch(0.95, "Earth")
    parts += mound(0.0, 0.0, 0.62, height, "Rock", 0.66)
    shell = m.ring_points(0.34, 7, rng, 0.08)
    parts += curtain(shell, 0.2, "Limestone", top="merlons", z=height)
    for x, y in shell[::3]:
        parts += tower(x, y, 0.13, 0.32, "Limestone", "cap", z=height)
    parts += tower(0.03, 0.0, 0.2, 0.78, "Limestone", "crenel", z=height)
    parts += flag(0.03, 0.0, height + 0.78 + 0.09)
    parts += gable_house(
        -0.13, 0.12, 0.2, 0.12, 0.13, 0.4, "Limestone", "TileOrange", z=height
    )
    for k in range(8):
        ang = 3.4 + k * 0.3 + rng.uniform(-0.06, 0.06)
        r = 0.8 + rng.uniform(-0.05, 0.08)
        parts += med_house(
            r * math.cos(ang), r * math.sin(ang), 0.2, 0.14, ang + math.pi / 2, rng
        )
    for k in range(3):
        parts += cypress(0.72 + 0.07 * k, 0.3 + 0.12 * k, 0.26)
    return parts


def cloister(x, y, side, wall="Limestone", roof="TileOrange", garth="Garden"):
    """Square cloister: four roofed galleries around a garth with a well."""
    parts = []
    depth = 0.08
    off = side / 2 - depth / 2
    for k in range(4):
        a = k * math.pi / 2
        parts += gable_house(
            x + math.cos(a) * off,
            y + math.sin(a) * off,
            side,
            depth,
            0.07,
            a + math.pi / 2,
            wall,
            roof,
            0.4,
        )
    inner = side - 2 * depth
    parts.append(
        m.box((inner, inner, 0.012 + F * 0.3), (x, y, 0.006 - F * 0.15), garth)
    )
    parts.append(m.cylinder(0.028, 0.05, (x, y, 0.03), wall, 6))
    return parts


def build_med_abbey(variant):
    """Abbey with a cloister, a campanile and cypresses."""
    first = variant == "a"
    rng = random.Random(529 if first else 1098)
    flip = 1 if first else -1
    parts = m.ground_patch(0.98, "Dirt")
    cy = 0.34 * flip
    if first:
        parts += med_church(-0.02, cy, 0.0, 1.35, tower_side=1)
    else:
        parts += gable_house(-0.02, cy, 0.85, 0.3, 0.3, 0.0, "Limestone", "TileOrange")
        parts += drum_dome(
            0.28, cy, 0.3, 0.17, 0.12, "Limestone", "TileOrange", "round"
        )
        parts += round_campanile(-0.55, cy - 0.1, 0.09, 0.6)
    parts += cloister(0.0, -0.1 * flip, 0.5)
    parts += gable_house(
        0.36, -0.1 * flip, 0.14, 0.5, 0.17, 0.0, "Limestone", "TileOrange"
    )
    parts += gable_house(
        0.0, -0.44 * flip, 0.56, 0.14, 0.15, 0.0, "Whitewash", "TileOrange"
    )
    parts += gable_house(
        -0.36, -0.12 * flip, 0.14, 0.44, 0.13, 0.0, "Ochre", "TileOrange"
    )
    precinct = [(-0.82, -0.82), (0.82, -0.82), (0.86, 0.8), (-0.8, 0.84)]
    parts += curtain(precinct, 0.08, "Limestone", thickness=0.045)
    parts += flag(-0.62, 0.5 * flip - 0.4 * (flip < 0), 0.0, 0.4)
    parts += plots(
        0.58, -0.56 * flip, 0.0, 4, 0.36, 0.07, ("Garden", "Field", "Garden")
    )
    parts += plots(-0.58, -0.6 * flip, 0.0, 3, 0.3, 0.07, ("Garden", "Earth", "Garden"))
    for k in range(6):
        parts += cypress(0.66 + rng.uniform(-0.02, 0.02), (-0.2 + k * 0.17) * flip, 0.3)
    for k in range(3):
        parts += med_house(1.0 + 0.04 * k, -0.4 + 0.3 * k, 0.2, 0.14, 1.4, rng)
    return parts


def build_med_village(variant):
    """Perched village around its church."""
    first = variant == "a"
    rng = random.Random(77 if first else 1167)
    radius, height, top = (0.78, 0.3, 0.5) if first else (0.72, 0.4, 0.46)
    parts = m.ground_patch(0.95, "Earth", 14)
    parts += mound(0.0, 0.0, radius, height, "Rock", top)
    if first:
        parts += med_church(0.02, 0.0, 0.3, 0.72, z=height)
        parts += flag(-0.3, 0.0, height, 0.3)
    else:
        parts += tower(0.0, 0.0, 0.18, 0.6, "Limestone", "crenel", z=height)
        parts += flag(0.0, 0.0, height + 0.69)
        ring = m.ring_points(radius * top * 0.92, 7, rng, 0.05)
        parts += curtain(ring, 0.1, "Limestone", thickness=0.05, z=height)
        parts += med_church(0.45, -0.72, 0.9, 0.62, bell_gable=True)
    rings = (
        (
            (radius * top * 0.78, 6, 0.0),
            (radius * 0.78, 9, 0.2),
            (radius * 1.12, 7, 0.5),
        )
        if first
        else (
            (radius * top * 0.6, 4, 0.0),
            (radius * 0.74, 8, 0.3),
            (radius * 1.15, 8, 0.1),
        )
    )
    for ring_radius, count, phase in rings:
        for k in range(count):
            ang = phase + TAU * k / count + rng.uniform(-0.1, 0.1)
            if not first and ring_radius > radius and -1.5 < ang - TAU < -0.6:
                continue
            x, y = ring_radius * math.cos(ang), ring_radius * math.sin(ang)
            z = mound_height(ring_radius + 0.07, radius, height, top)
            w = rng.uniform(0.19, 0.25)
            parts += med_house(x, y, w, w * 0.7, ang + math.pi / 2, rng, z)
    parts += plots(0.25, -1.0, 0.1, 3, 0.5, 0.07, ("Garden", "Field", "Garden"))
    for k in range(4):
        parts += cypress(-0.95 + 0.06 * k, 0.25 + 0.14 * k, 0.26)
    return parts


# --- byz: Greeks, Bulgarians, Serbs, Georgians, Armenians ----------------------------

BYZ_BANDS = (("Limestone", 0.42), ("Brick", 0.12), ("Limestone", 0.34), ("Brick", 0.12))
BYZ_WALLS = ("Limestone", "Whitewash", "Brick", "Limestone")


def byz_house(x, y, w, d, angle, rng, z=0.0):
    """Stone or brick house under a low tile roof."""
    return gable_house(
        x,
        y,
        w,
        d,
        rng.uniform(0.1, 0.15),
        angle,
        rng.choice(BYZ_WALLS),
        "TileOrange",
        z=z,
    )


def byz_tower(x, y, side, height, top="cap", z=0.0):
    """Square tower of banded stone and brick with a jutting parapet."""
    parts = banded(x, y, side, side, height, BYZ_BANDS, z=z)
    return parts + tower(x, y, side, 0.0, "Limestone", top, z=z + height)[1:]


def byz_walls(points, height, gates, z=0.0, tower_side=0.15):
    """Banded curtain wall with tall square towers and gate towers."""
    return curtain(
        points,
        height,
        "Limestone",
        tower_at=lambda x, y: byz_tower(x, y, tower_side, height * 1.55, z=z),
        gates=gates,
        gate_at=lambda x, y, a: gatehouse(x, y, a, height * 1.5, "Limestone", z=z),
        bands=BYZ_BANDS,
        z=z,
    )


def byz_church(x, y, angle, s=1.0, domes=1, conical=False, z=0.0):
    """Cross-in-square church: tiled cross arms, apse, dome(s) on drums.

    Greek type: brick walls, tiled domes. ``conical``: Caucasian type, pale stone, tall drum
    under a conical roof.
    """
    wall = "Limestone" if conical else "Brick"
    roof = "Lead" if conical else "TileOrange"
    at = local_frame(x, y, angle, s)
    parts = [block(x, y, 0.5 * s, 0.5 * s, 0.24 * s, wall, angle, z)]
    parts.append(
        block(
            x,
            y,
            0.53 * s,
            0.53 * s,
            0.025 * s,
            "TileOrange",
            angle,
            z + 0.24 * s,
            False,
        )
    )
    for turn in (0.0, math.pi / 2):
        parts += gable_house(
            x, y, 0.66 * s, 0.2 * s, 0.34 * s, angle + turn, wall, "TileOrange", 0.35, z
        )
    ax, ay = at(0.36, 0.0)
    parts.append(
        m.cylinder(0.1 * s, 0.3 * s + F, (ax, ay, z + (0.3 * s - F) / 2), wall, 8)
    )
    parts.append(m.cone(0.115 * s, 0.08 * s, (ax, ay, z + 0.34 * s), "TileOrange", 8))
    drum = (0.26 if conical else 0.15) * s
    parts += drum_dome(
        x,
        y,
        z + 0.36 * s,
        0.13 * s,
        drum,
        wall if conical else "Limestone",
        roof,
        "cone" if conical else "round",
        8,
        None if conical else "Gilt",
    )
    if domes > 1:
        for dx, dy in ((-0.19, -0.19), (0.19, -0.19), (0.19, 0.19), (-0.19, 0.19)):
            cx, cy = at(dx, dy)
            parts += drum_dome(
                cx, cy, z + 0.26 * s, 0.07 * s, 0.1 * s, "Limestone", roof, "round", 6
            )
    return parts


def great_church(x, y, angle, s=1.0):
    """Imperial domed basilica: massive base, wide lead dome between two half domes."""
    at = local_frame(x, y, angle, s)
    parts = [block(x, y, 0.95 * s, 0.8 * s, 0.38 * s, "Ochre", angle)]
    parts.append(
        block(x, y, 0.56 * s, 0.6 * s, 0.16 * s, "Ochre", angle, 0.38 * s, False)
    )
    parts.append(m.cylinder(0.29 * s, 0.1 * s, (x, y, 0.58 * s), "Limestone", 12))
    parts += cupola(x, y, 0.63 * s, 0.3 * s, "Lead", "shallow", 12, "Gilt")
    for dx in (-0.36, 0.36):
        hx, hy = at(dx, 0.0)
        parts += cupola(hx, hy, 0.38 * s, 0.24 * s, "Lead", "round", 8)
    for dx in (-0.44, 0.44):
        for dy in (-0.42, 0.42):
            bx, by = at(dx, dy)
            parts.append(block(bx, by, 0.13 * s, 0.13 * s, 0.5 * s, "Limestone", angle))
    nx, ny = at(-0.54, 0.0)
    parts += gable_house(
        nx, ny, 0.7 * s, 0.14 * s, 0.2 * s, angle + math.pi / 2, "Ochre", "TileOrange"
    )
    return parts


def build_byz_city(variant):
    """City behind banded walls, domed cathedral and churches."""
    first = variant == "a"
    rng = random.Random(1261 if first else 1185)
    radius = 1.38 if first else 1.32
    points = m.ring_points(radius, 11 if first else 9, rng, 0.07, phase=0.15)
    gates = (0, 5) if first else (2, 6)
    parts = m.ground_patch(radius * 0.97)
    parts += byz_walls(points, 0.22, gates)
    if first:
        parts += great_church(-0.05, 0.1, 0.25, 1.0)
        keep_out = [(-0.05, 0.1, 0.68)]
        churches = ((0.7, -0.5, 0.4, 0.75, 1), (-0.75, -0.45, -0.3, 0.7, 1))
        cx, cy = points[3][0] * 0.8, points[3][1] * 0.8
    else:
        parts += byz_church(0.0, 0.05, 0.3, 1.5, 1, True)
        keep_out = [(0.0, 0.05, 0.62)]
        churches = ((0.65, -0.55, 0.2, 0.75, 1), (-0.7, 0.5, 0.8, 0.7, 1))
        cx, cy = points[4][0] * 0.74, points[4][1] * 0.74
        parts += mound(cx, cy, 0.34, 0.16, "Rock", 0.8, 10)
    for px, py, pa, ps, pd in churches:
        parts += byz_church(px, py, pa, ps, pd, not first)
        keep_out.append((px, py, 0.36 * ps + 0.05))
    # Citadel: great tower and palace hall.
    base = 0.0 if first else 0.16
    parts += byz_tower(cx, cy, 0.3, 0.62, "crenel", z=base)
    parts += flag(cx, cy, base + 0.62 + 0.09)
    parts += gable_house(
        cx * 0.78,
        cy * 0.78,
        0.34,
        0.16,
        0.2,
        math.atan2(cy, cx) + math.pi / 2,
        "Limestone",
        "TileOrange",
        z=0.0,
    )
    keep_out.append((cx, cy, 0.36))
    parts += scatter(
        rng, 60, radius * 0.95, byz_house, keep_out, shrink(points, 0.9), (0.2, 0.29)
    )
    gx, gy = (
        (points[gates[0]][0] + points[gates[0] + 1][0]) / 2,
        (points[gates[0]][1] + points[gates[0] + 1][1]) / 2,
    )
    base_angle = math.atan2(gy, gx)
    for k in range(6):
        ang = base_angle + (0.14 if k % 2 else -0.14) + rng.uniform(-0.04, 0.04)
        t = (1.22 + 0.13 * (k // 2)) * math.hypot(gx, gy)
        parts += byz_house(
            t * math.cos(ang),
            t * math.sin(ang),
            0.22,
            0.15,
            base_angle + rng.uniform(-0.2, 0.2),
            rng,
        )
    return parts


def build_byz_town(variant):
    """Walled town around a cross-in-square church."""
    first = variant == "a"
    rng = random.Random(1204 if first else 1018)
    points = m.ring_points(0.9, 8 if first else 7, rng, 0.09, phase=0.5)
    parts = m.ground_patch(0.88)
    parts += byz_walls(points, 0.18, (1, 5) if first else (0, 4), tower_side=0.14)
    parts += byz_church(0.0, 0.08, 0.4, 1.0, 5 if first else 1, not first)
    tx, ty = points[3][0] * 0.72, points[3][1] * 0.72
    parts += byz_tower(tx, ty, 0.2, 0.52, "crenel")
    parts += flag(tx, ty, 0.52 + 0.09)
    keep_out = [(0.0, 0.08, 0.4), (tx, ty, 0.18)]
    parts += scatter(
        rng, 24, 0.82, byz_house, keep_out, shrink(points, 0.88), (0.19, 0.27)
    )
    return parts


def build_byz_castle(variant):
    """Kastron: hilltop fortress with square towers."""
    first = variant == "a"
    rng = random.Random(1082 if first else 961)
    radius, height, top = (0.74, 0.2, 0.74) if first else (0.66, 0.36, 0.7)
    parts = m.ground_patch(0.95, "Earth")
    parts += mound(0.0, 0.0, radius, height, "Rock", top)
    shell = m.ring_points(radius * top * 0.86, 7 if first else 6, rng, 0.07)
    parts += byz_walls(shell, 0.2, (2,), z=height, tower_side=0.13)
    parts += byz_tower(0.06, 0.04, 0.22, 0.66, "crenel", z=height)
    parts += flag(0.06, 0.04, height + 0.66 + 0.09)
    parts += byz_church(-0.17, -0.12, 0.3, 0.42, 1, not first, z=height)
    for k in range(7):
        ang = 3.3 + k * 0.33 + rng.uniform(-0.06, 0.06)
        r = 0.84 + rng.uniform(-0.04, 0.06)
        parts += byz_house(
            r * math.cos(ang), r * math.sin(ang), 0.2, 0.14, ang + math.pi / 2, rng
        )
    return parts


def build_byz_abbey(variant):
    """Fortified monastery: ranges against the walls, katholikon in the court."""
    first = variant == "a"
    rng = random.Random(963 if first else 1106)
    parts = m.ground_patch(1.0, "Dirt")
    if first:
        walls = [(-0.74, -0.62), (0.74, -0.62), (0.74, 0.62), (-0.74, 0.62)]
    else:
        walls = m.ring_points(0.78, 6, rng, 0.05, phase=0.3)
    height = 0.26
    parts += curtain(
        walls,
        height,
        "Limestone",
        gates=(0,),
        gate_at=lambda x, y, a: gatehouse(
            x, y, a, height * 1.5, "Limestone", "TileOrange"
        ),
        bands=BYZ_BANDS,
    )
    # Cell ranges built against the inner face of the walls, roofs above the parapet.
    for index in range(1, len(walls)):
        x0, y0 = walls[index]
        x1, y1 = walls[(index + 1) % len(walls)]
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        length = math.hypot(x1 - x0, y1 - y0)
        inward = 1.0 - 0.13 / max(math.hypot(mx, my), 0.01)
        parts += gable_house(
            mx * inward,
            my * inward,
            length * 0.66,
            0.16,
            height + 0.05,
            math.atan2(y1 - y0, x1 - x0),
            "Whitewash",
            "TileOrange",
            0.4,
        )
    tx, ty = walls[2]
    parts += byz_tower(tx * 0.9, ty * 0.9, 0.24, 0.74, "crenel")
    parts += flag(tx * 0.9, ty * 0.9, 0.74 + 0.09)
    parts += byz_church(0.0, 0.0, 0.0, 0.95, 5 if first else 1, not first)
    parts += plots(0.0, -0.84, 0.0, 2, 0.6, 0.07, ("Garden", "Field"))
    for k in range(4):
        parts += cypress(-0.92 + rng.uniform(-0.02, 0.02), -0.3 + k * 0.2, 0.3)
    for k in range(3):
        parts += byz_house(1.0 + 0.03 * k, -0.35 + 0.3 * k, 0.2, 0.14, 1.5, rng)
    return parts


def build_byz_village(variant):
    """Open village of tiled houses around a small domed church."""
    first = variant == "a"
    rng = random.Random(412 if first else 733)
    parts = [
        m.box(
            (1.9, 0.1, 0.01 + F), (0, 0, 0.005 - F / 2), "Dirt", rotation=(0, 0, -0.4)
        )
    ]
    parts += byz_church(0.05, 0.22, 0.2, 0.8, 1, not first)
    parts += flag(-0.3, 0.3, 0.0, 0.34)
    keep_out = [(0.05, 0.22, 0.34)]
    if not first:
        parts += tower(-0.5, -0.3, 0.16, 0.5, "Limestone", "pyramid", "TileOrange")
        keep_out.append((-0.5, -0.3, 0.14))
    parts += scatter(rng, 13, 0.85, byz_house, keep_out, None, (0.2, 0.27), 1.25)
    parts.append(
        m.cylinder(0.13, 0.012 + F * 0.3, (0.55, -0.6, 0.006 - F * 0.15), "Sand", 10)
    )
    parts += plots(-0.25, -0.95, -0.1, 4, 0.55, 0.07, ("Garden", "Field", "Garden"))
    parts += plots(0.92, 0.25, 1.4, 4, 0.5, 0.07)
    return parts


# --- rus: Rus', Novgorod, Ruthenia, Lithuania ----------------------------------------


def build_rus_city(variant):
    """Detinets and posad: log (a) or white stone (b) kremlin, cathedral, log houses."""
    return placeholder(1.4)


def build_rus_town(variant):
    """Palisaded gorod with a wooden church and the prince's hall."""
    return placeholder(0.95)


def build_rus_castle(variant):
    """Wooden fort on a mound (a) or white stone fortress (b)."""
    return placeholder(0.95)


def build_rus_abbey(variant):
    """Monastery inside a wooden enclosure."""
    return placeholder(1.0)


def build_rus_village(variant):
    """Street village of log houses with a wooden chapel."""
    return placeholder(0.95)


# --- isl: Maghreb, Arabs, Turks, Berbers, al-Andalus ---------------------------------


def build_isl_city(variant):
    """Medina: flat roofs, great mosque with its court and minaret, kasbah."""
    return placeholder(1.4)


def build_isl_town(variant):
    """Small walled medina around its mosque."""
    return placeholder(0.95)


def build_isl_castle(variant):
    """Kasbah of rammed earth (a) or citadel on a mound (b)."""
    return placeholder(0.95)


def build_isl_abbey(variant):
    """Ribat (a) or caravanserai (b) with the domed tomb of a zawiya."""
    return placeholder(1.0)


def build_isl_village(variant):
    """Village of cubic houses with palms and a small mosque."""
    return placeholder(0.95)


# --- steppe: Kipchaks and the Horde --------------------------------------------------


def build_steppe_city(variant):
    """Sarai: mud-brick city without walls, palaces, mosque, yurts around."""
    return placeholder(1.6)


def build_steppe_town(variant):
    """Large encampment of yurts around the khan's tent, with corrals."""
    return placeholder(0.95)


def build_steppe_castle(variant):
    """Earth fort with a palisade."""
    return placeholder(0.95)


def build_steppe_abbey(variant):
    """Domed mausoleum (a) or kurgan with stone statues (b)."""
    return placeholder(1.0)


def build_steppe_village(variant):
    """Small camp of yurts with a corral."""
    return placeholder(0.95)


BUILDERS = {
    "med": (
        build_med_city,
        build_med_town,
        build_med_castle,
        build_med_abbey,
        build_med_village,
    ),
    "byz": (
        build_byz_city,
        build_byz_town,
        build_byz_castle,
        build_byz_abbey,
        build_byz_village,
    ),
    "rus": (
        build_rus_city,
        build_rus_town,
        build_rus_castle,
        build_rus_abbey,
        build_rus_village,
    ),
    "isl": (
        build_isl_city,
        build_isl_town,
        build_isl_castle,
        build_isl_abbey,
        build_isl_village,
    ),
    "steppe": (
        build_steppe_city,
        build_steppe_town,
        build_steppe_castle,
        build_steppe_abbey,
        build_steppe_village,
    ),
}


def model_names(family):
    """Names of the ten models of ``family``, kind by kind."""
    return [f"{kind}_{family}_{variant}" for kind in KINDS for variant in VARIANTS]


# ``<kind>_<family>_<variant>`` -> parts builder, merged into ``settlements.MODELS``.
MODELS = {
    f"{kind}_{family}_{variant}": partial(build, variant)
    for family in FAMILIES
    for kind, build in zip(KINDS, BUILDERS[family], strict=True)
    for variant in VARIANTS
}


def triangle_budget(name, default):
    """Triangle budget of the GC2 / GC3 contract for the models of this module."""
    if name not in MODELS:
        return default
    return 12000 if name.startswith("city_") else 5000
