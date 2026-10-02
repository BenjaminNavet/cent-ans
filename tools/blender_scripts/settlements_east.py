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


def banded(x, y, w, d, h, bands, angle=0.0):
    """Wall block made of horizontal courses ``[(material, fraction), ...]`` (bottom to top)."""
    parts = []
    z = 0.0
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
            parts += banded(mx, my, length, thickness, height, bands, angle)
        else:
            parts.append(block(mx, my, length, thickness, height, mat, angle))
        if top == "merlons":
            parts += merlon_row(x0, y0, x1, y1, height, mat)
        elif top == "stakes":
            parts += stakes(x0, y0, x1, y1, height, mat)
        if tower_at is not None:
            parts += tower_at(x0, y0)
        if gate_at is not None and index in gates:
            parts += gate_at(mx, my, angle)
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
    gap=1.1,
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


def build_med_city(variant):
    """Commune: pale walls, duomo, campanile, palazzo and tower houses."""
    return placeholder(1.4)


def build_med_town(variant):
    """Walled hill town with its parish church and a few tower houses."""
    return placeholder(0.95)


def build_med_castle(variant):
    """Castello with four corner towers (a) or rocca on a rock (b)."""
    return placeholder(0.95)


def build_med_abbey(variant):
    """Abbey with a cloister, a campanile and cypresses."""
    return placeholder(1.0)


def build_med_village(variant):
    """Perched village around its church."""
    return placeholder(0.95)


# --- byz: Greeks, Bulgarians, Serbs, Georgians, Armenians ----------------------------


def build_byz_city(variant):
    """City behind banded walls, domed cathedral and churches."""
    return placeholder(1.4)


def build_byz_town(variant):
    """Walled town around a cross-in-square church."""
    return placeholder(0.95)


def build_byz_castle(variant):
    """Kastron: hilltop fortress with square towers."""
    return placeholder(0.95)


def build_byz_abbey(variant):
    """Fortified monastery: ranges against the walls, katholikon in the court."""
    return placeholder(1.0)


def build_byz_village(variant):
    """Open village of tiled houses around a small domed church."""
    return placeholder(0.95)


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
