"""Dedicated monument meshes of the landmark cities (lot L1), in real metres.

Each builder returns a list of ``(material, (verts, faces))`` in a local frame: origin at the
monument centre, +x along the main axis (churches: towards the choir), Z up, metres. The city
generator scales, rotates and places them. Proportions after Viollet-le-Duc's Dictionnaire
raisonné (public domain) and the plan de Bâle.
"""

import math

import landmark_geometry as g

# --- Notre-Dame de Paris (c. 1345: towers finished, 13th-century spire, flying buttresses) ----

ND_WEST = -64.0  # facade plane
ND_FACADE_DEPTH = 15.0
ND_APSE_X = 40.0  # centre of the chevet half-circles
ND_INNER = 6.25  # half width of the central vessel
ND_OUTER = 24.0  # half width with double aisles and chapels
ND_VAULT = 33.0  # top of the clerestory walls
ND_RIDGE = 43.0
ND_AISLE_WALL = 12.0
ND_AISLE_TOP = 22.0
ND_TOWER_TOP = 69.0


def _outline(radius, apse_x, west, bays, apse_steps):
    """Chevet outline of half width ``radius``: south side west->east, half circle, north side."""
    points = []
    for i in range(bays + 1):
        points.append((west + (apse_x - west) * i / bays, -radius))
    for i in range(1, apse_steps):
        a = -math.pi / 2 + math.pi * i / apse_steps
        points.append((apse_x + radius * math.cos(a), radius * math.sin(a)))
    for i in range(bays, -1, -1):
        points.append((west + (apse_x - west) * i / bays, radius))
    return points


def notre_dame():
    """Notre-Dame, the centrepiece: two flat-topped towers, rose, spire, flying buttresses."""
    parts = []
    stone, lead, glass = "NDStone", "Lead", "Glass"
    west_body = ND_WEST + ND_FACADE_DEPTH - 1.0
    bays, apse_steps = 12, 10

    # Central vessel (nave + choir + apse), walls up to the vault.
    inner = _outline(ND_INNER, ND_APSE_X, west_body, bays, apse_steps)
    parts.append((stone, g.prism(inner, -2.0, ND_VAULT, top=False)))
    # Double aisles and chapels, with their lean-to roofs rising to the clerestory.
    outer = _outline(ND_OUTER, ND_APSE_X, west_body, bays, apse_steps)
    parts.append((stone, g.prism(outer, -2.0, ND_AISLE_WALL, top=False)))
    ring_verts = [(x, y, ND_AISLE_WALL) for x, y in outer] + [
        (x, y, ND_AISLE_TOP) for x, y in inner
    ]
    n = len(outer)
    ring_faces = [(i, i + 1, n + i + 1, n + i) for i in range(n - 1)]
    parts.append((lead, (ring_verts, ring_faces)))
    # Main roof: gable over the straight part, half cone over the apse.
    length = ND_APSE_X - west_body
    parts.append(
        (
            lead,
            g.gable_roof(
                west_body + length / 2,
                0,
                ND_VAULT,
                length,
                ND_INNER * 2,
                ND_RIDGE - ND_VAULT,
            ),
        )
    )
    apse_verts = [(ND_APSE_X, 0.0, ND_RIDGE)]
    for i in range(apse_steps + 1):
        a = -math.pi / 2 + math.pi * i / apse_steps
        apse_verts.append(
            (ND_APSE_X + ND_INNER * math.cos(a), ND_INNER * math.sin(a), ND_VAULT)
        )
    parts.append((lead, (apse_verts, [(i + 1, i + 2, 0) for i in range(apse_steps)])))
    # Transept (barely protruding) with gabled roof and the north and south roses.
    tx0, tx1 = -5.0, 9.0
    tcx = (tx0 + tx1) / 2
    parts.append((stone, g.box(tcx, 0, -2.0, tx1 - tx0, 50.0, ND_VAULT + 2.0)))
    parts.append(
        (
            lead,
            g.gable_roof(
                tcx, 0, ND_VAULT, 50.0, tx1 - tx0, ND_RIDGE - ND_VAULT, math.pi / 2
            ),
        )
    )
    for side in (-1, 1):
        parts.append(
            (
                glass,
                g.disc_vertical(
                    tcx, side * 25.0, 24.0, 5.2, side * math.pi / 2, 14, 0.3
                ),
            )
        )
        parts.append(
            (
                stone,
                g.disc_vertical(
                    tcx, side * 25.0, 24.0, 1.4, side * math.pi / 2, 8, 0.5
                ),
            )
        )
        parts.append(
            (glass, g.panel(tcx, side * 25.0, 0.0, 5.0, 8.0, side * math.pi / 2, 0.3))
        )
        parts.append(
            (stone, g.pyramid(tcx, side * 25.3, ND_RIDGE - 1.0, 1.6, 1.2, 6.0))
        )

    # Clerestory lancets, aisle windows and the flying buttresses of each bay.
    nave_bays = [west_body + 3.5 + 7.2 * i for i in range(6)]
    choir_bays = [12.5 + 6.8 * i for i in range(4)]
    for x in nave_bays + choir_bays:
        for side in (-1, 1):
            normal = side * math.pi / 2
            parts.append(
                (glass, g.panel(x + 3.4, side * ND_INNER, 23.5, 3.2, 6.0, normal, 0.25))
            )
            parts.append(
                (glass, g.panel(x + 3.4, side * ND_OUTER, 2.5, 3.4, 6.0, normal, 0.25))
            )
            parts += _flying_buttress(x, side)
    for i in range(1, apse_steps, 2):
        a = -math.pi / 2 + math.pi * i / apse_steps
        parts += _radial_buttress(a)
        mid = a + math.pi / apse_steps
        parts.append(
            (
                glass,
                g.panel(
                    ND_APSE_X + (ND_INNER + 0.25) * math.cos(mid),
                    (ND_INNER + 0.25) * math.sin(mid),
                    23.5,
                    2.6,
                    6.0,
                    mid,
                ),
            )
        )

    # West facade and towers.
    parts += _nd_facade()
    # Spire at the crossing (13th century, lead over timber).
    parts += _nd_spire(tcx)
    return parts


def _flying_buttress(x, side):
    """Pier with pinnacle on the aisle wall and a two-segment flyer to the clerestory."""
    parts = []
    pier_y = side * (ND_OUTER + 1.2)
    parts.append(("NDStone", g.box(x, pier_y, -2.0, 1.7, 3.0, 25.0)))
    parts.append(("NDStone", g.pyramid(x, pier_y, 23.0, 1.7, 3.0, 5.5)))
    a = (x, side * (ND_OUTER + 0.2), 22.5)
    m = (x, side * 15.0, 28.5)
    b = (x, side * (ND_INNER + 0.3), 31.0)
    parts.append(("NDStone", g.beam(a, m, 1.1, 1.6)))
    parts.append(("NDStone", g.beam(m, b, 1.1, 1.6)))
    # Lower flyer.
    parts.append(
        (
            "NDStone",
            g.beam(
                (x, side * (ND_OUTER + 0.2), 18.0), (x, side * 15.0, 21.0), 0.9, 1.2
            ),
        )
    )
    return parts


def _radial_buttress(angle):
    """Flying buttress of the chevet along the radius ``angle``."""
    c, s = math.cos(angle), math.sin(angle)

    def at(r, z):
        return (ND_APSE_X + r * c, r * s, z)

    px, py, _ = at(ND_OUTER + 1.2, 0)
    parts = [
        ("NDStone", g.box(px, py, -2.0, 3.0, 1.7, 25.0, angle)),
        ("NDStone", g.pyramid(px, py, 23.0, 3.0, 1.7, 5.5, angle)),
        ("NDStone", g.beam(at(ND_OUTER + 0.2, 22.5), at(15.0, 28.5), 1.1, 1.6)),
        ("NDStone", g.beam(at(15.0, 28.5), at(ND_INNER + 0.3, 31.0), 1.1, 1.6)),
    ]
    return parts


def _nd_facade():
    """Harmonic facade: three portals, gallery of kings, rose, grand gallery, twin towers."""
    parts = []
    stone, glass, dark = "NDStone", "Glass", "Dark"
    x0 = ND_WEST
    depth = ND_FACADE_DEPTH
    cx = x0 + depth / 2
    west = math.pi  # facade normal: -x
    # Central block up to the grand gallery and the two towers.
    parts.append((stone, g.box(cx, 0, -2.0, depth, 11.0, 47.0)))
    for side in (-1, 1):
        ty = side * 12.75
        parts.append((stone, g.box(cx, ty, -2.0, depth, 15.5, ND_TOWER_TOP + 2.0)))
        # Cornice and parapet of the flat tower tops.
        parts.append((stone, g.box(cx, ty, ND_TOWER_TOP - 0.8, depth + 1.2, 16.7, 1.6)))
        # Corner buttresses up to the gallery.
        for by in (side * 5.2, side * 20.3):
            parts.append((stone, g.box(x0 - 0.6, by, -2.0, 2.2, 1.8, 46.0)))
        # Twin belfry lancets on every face.
        for k in (-1, 1):
            parts.append((glass, g.panel(x0, ty + k * 3.4, 50.5, 2.6, 12.0, west, 0.2)))
            parts.append(
                (glass, g.panel(x0 + depth, ty + k * 3.4, 50.5, 2.6, 12.0, 0.0, 0.2))
            )
            parts.append(
                (
                    glass,
                    g.panel(
                        cx + k * 3.4,
                        ty + side * 7.75,
                        50.5,
                        2.6,
                        12.0,
                        side * math.pi / 2,
                        0.2,
                    ),
                )
            )
            # Paired lancets of the rose storey on the towers.
            parts.append((glass, g.panel(x0, ty + k * 2.6, 27.0, 1.9, 7.0, west, 0.2)))
        # Side portals.
        parts.append((dark, g.panel(x0, ty, 0.0, 6.2, 9.0, west, 0.25)))
    # Central portal.
    parts.append((dark, g.panel(x0, 0.0, 0.0, 7.6, 11.0, west, 0.25)))
    # Gallery of kings: a band of niches across the facade.
    for i in range(14):
        y = -19.5 + 3.0 * i
        parts.append((dark, g.panel(x0, y, 17.2, 1.9, 2.6, west, 0.3, pointed=False)))
    parts.append((stone, g.box(x0 - 0.5, 0, 16.4, 1.2, 41.5, 0.8)))
    parts.append((stone, g.box(x0 - 0.5, 0, 20.4, 1.2, 41.5, 0.8)))
    # West rose.
    parts.append((glass, g.disc_vertical(x0, 0.0, 30.5, 5.0, west, 16, 0.3)))
    parts.append((stone, g.disc_vertical(x0, 0.0, 30.5, 1.5, west, 8, 0.45)))
    # Grand gallery: openwork band between and across the towers.
    for i in range(16):
        y = -19.7 + 2.63 * i
        parts.append((dark, g.panel(x0, y, 41.0, 1.3, 4.2, west, 0.3)))
    parts.append((stone, g.box(x0 - 0.5, 0, 46.2, 1.2, 41.5, 0.8)))
    # Nave gable behind the gallery.
    parts.append(("Lead", g.gable_roof(x0 + depth - 1.0, 0, 47.0, 2.0, 10.0, 7.0)))
    return parts


def _nd_spire(x):
    """Octagonal lantern and needle with pinnacles, gilded cross."""
    parts = []
    base = ND_RIDGE - 2.0
    parts.append(("Lead", g.cylinder(x, 0, base, 3.4, 12.0, 8)))
    for i in range(8):
        a = 2 * math.pi * (i + 0.5) / 8
        parts.append(
            (
                "Glass",
                g.panel(
                    3.45 * math.cos(a) + x, 3.45 * math.sin(a), base + 3.0, 1.6, 6.0, a
                ),
            )
        )
    parts.append(("Lead", g.cone(x, 0, base + 12.0, 3.6, 43.0, 8)))
    for i in range(4):
        a = 2 * math.pi * i / 4 + math.pi / 4
        parts.append(
            (
                "Lead",
                g.cone(x + 4.6 * math.cos(a), 4.6 * math.sin(a), base, 0.9, 16.0, 6),
            )
        )
    top = base + 55.0
    parts.append(("Gold", g.box(x, 0, top - 1.0, 0.5, 0.5, 4.0)))
    parts.append(("Gold", g.box(x, 0, top + 1.5, 0.5, 2.4, 0.5)))
    return parts


# --- Sainte-Chapelle ------------------------------------------------------------------


def sainte_chapelle():
    """Palatine chapel: tall glass walls between pinnacled buttresses, steep roof, slender spire."""
    parts = []
    stone, lead, glass = "NDStone", "Lead", "Glass"
    x0, x1, half, wall, ridge = -18.0, 12.0, 5.5, 30.0, 42.0
    outline = [(x0, -half), (x1, -half)]
    for i in range(1, 5):
        a = -math.pi / 2 + math.pi * i / 5
        outline.append((x1 + half * math.cos(a), half * math.sin(a)))
    outline += [(x1, half), (x0, half)]
    parts.append((stone, g.prism(outline, -2.0, wall, top=False)))
    parts.append(
        (lead, g.gable_roof((x0 + x1) / 2, 0, wall, x1 - x0, half * 2, ridge - wall))
    )
    apse = [(x1, 0.0, ridge)]
    for i in range(6):
        a = -math.pi / 2 + math.pi * i / 5
        apse.append((x1 + half * math.cos(a), half * math.sin(a), wall))
    parts.append((lead, (apse, [(i + 1, i + 2, 0) for i in range(5)])))
    for i in range(6):
        x = x0 + 1.0 + 5.0 * i
        for side in (-1, 1):
            parts.append(
                (stone, g.box(x, side * (half + 1.3), -2.0, 1.4, 2.6, wall + 1.0))
            )
            parts.append(
                (stone, g.pyramid(x, side * (half + 1.3), wall - 1.0, 1.4, 2.6, 7.0))
            )
            if i < 5:
                normal = side * math.pi / 2
                parts.append(
                    (glass, g.panel(x + 2.5, side * half, 12.0, 3.4, 13.0, normal, 0.2))
                )
                parts.append(
                    (glass, g.panel(x + 2.5, side * half, 2.5, 2.2, 4.0, normal, 0.2))
                )
    # West front: rose, two turrets with spirelets, porch.
    parts.append((glass, g.disc_vertical(x0, 0, 22.0, 3.8, math.pi, 14, -0.2)))
    for side in (-1, 1):
        parts.append(
            (stone, g.cylinder(x0 - 0.5, side * (half + 0.4), -2.0, 1.6, 36.0, 8))
        )
        parts.append((lead, g.cone(x0 - 0.5, side * (half + 0.4), 34.0, 1.7, 9.0, 8)))
    parts.append((stone, g.box(x0 - 2.5, 0, -2.0, 5.0, 10.0, 12.0)))
    parts.append((lead, g.gable_roof(x0 - 2.5, 0, 10.0, 10.0, 5.0, 4.0, math.pi / 2)))
    parts.append((stone, g.gable_roof(x0 - 0.2, 0, wall, 0.8, half * 2, ridge - wall)))
    # Spire over the middle of the roof.
    sx = (x0 + x1) / 2 - 2.0
    parts.append((lead, g.cylinder(sx, 0, ridge - 3.0, 1.8, 7.0, 8)))
    parts.append((lead, g.cone(sx, 0, ridge + 4.0, 2.0, 29.0, 8)))
    parts.append(("Gold", g.box(sx, 0, ridge + 32.5, 0.4, 0.4, 2.5)))
    return parts


# --- Palais de la Cité ----------------------------------------------------------------


def palais_cite():
    """Royal palace: double-naved great hall, Conciergerie towers on the quay, clock tower."""
    parts = []
    stone, slate = "Stone", "Slate"
    # Great hall: two parallel naves.
    for y in (0.0, 13.0):
        parts.append((stone, g.box(-5.0, y, -2.0, 70.0, 13.0, 18.0)))
        parts.append((slate, g.gable_roof(-5.0, y, 16.0, 70.0, 13.0, 13.0)))
    # Royal lodgings and galleries.
    parts.append((stone, g.box(48.0, 8.0, -2.0, 34.0, 16.0, 16.0)))
    parts.append((slate, g.gable_roof(48.0, 8.0, 14.0, 34.0, 16.0, 12.0)))
    parts.append((stone, g.box(-52.0, 10.0, -2.0, 22.0, 30.0, 14.0)))
    parts.append(
        (slate, g.gable_roof(-52.0, 10.0, 12.0, 30.0, 22.0, 10.0, math.pi / 2))
    )
    # Conciergerie towers on the north quay: Bonbec, Argent, César.
    for x, r, h in ((-40.0, 5.5, 26.0), (-14.0, 6.5, 30.0), (2.0, 6.5, 30.0)):
        parts.append((stone, g.cylinder(x, 30.0, -2.0, r, h + 2.0, 10)))
        parts.append((slate, g.cone(x, 30.0, h, r + 0.4, r * 2.4, 10)))
    parts.append((stone, g.box(-20.0, 27.0, -2.0, 50.0, 8.0, 14.0)))
    parts.append((slate, g.gable_roof(-20.0, 27.0, 12.0, 50.0, 8.0, 6.0)))
    # Clock tower (1350-1370) at the corner.
    parts.append((stone, g.box(72.0, 28.0, -2.0, 11.0, 11.0, 42.0)))
    parts.append((slate, g.pyramid(72.0, 28.0, 40.0, 12.0, 12.0, 16.0)))
    for dx, dy in ((-5.5, -5.5), (5.5, -5.5), (5.5, 5.5), (-5.5, 5.5)):
        parts.append((slate, g.cone(72.0 + dx, 28.0 + dy, 38.0, 1.4, 8.0, 6)))
    parts.append(
        ("Gold", g.panel(72.0, 22.4, 30.0, 4.0, 4.0, -math.pi / 2, 0.1, pointed=False))
    )
    # Precinct wall with its courtyard (the Sainte-Chapelle stands in the south-east part).
    wall = [(-68.0, -78.0), (92.0, -78.0), (92.0, 36.0), (-68.0, 36.0)]
    for i in range(4):
        (ax, ay), (bx, by) = wall[i], wall[(i + 1) % 4]
        if i == 2:
            continue  # the quay side is closed by the buildings
        length = math.hypot(bx - ax, by - ay)
        parts.append(
            (
                stone,
                g.box(
                    (ax + bx) / 2,
                    (ay + by) / 2,
                    -2.0,
                    length,
                    2.0,
                    9.0,
                    math.atan2(by - ay, bx - ax),
                ),
            )
        )
    parts.append(("Paving", g.flat(wall, 0.15)))
    return parts


# --- Louvre ---------------------------------------------------------------------------


def louvre(variant=""):
    """Philippe Auguste's fortress around its great tower; ``charles_v``: royal residence."""
    parts = []
    stone, slate, water = "Stone", "Slate", "Water"
    hx, hy = 39.0, 36.0
    residence = variant == "charles_v"
    parts.append(
        (
            water,
            g.flat(
                [
                    (-hx - 12, -hy - 12),
                    (hx + 12, -hy - 12),
                    (hx + 12, hy + 12),
                    (-hx - 12, hy + 12),
                ],
                0.1,
            ),
        )
    )
    parts.append(
        (
            stone,
            g.flat(
                [
                    (-hx - 3, -hy - 3),
                    (hx + 3, -hy - 3),
                    (hx + 3, hy + 3),
                    (-hx - 3, hy + 3),
                ],
                0.2,
            ),
        )
    )
    corners = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
    for i in range(4):
        (ax, ay), (bx, by) = corners[i], corners[(i + 1) % 4]
        parts.append(
            (
                stone,
                g.box(
                    (ax + bx) / 2,
                    (ay + by) / 2,
                    -2.0,
                    math.hypot(bx - ax, by - ay),
                    3.0,
                    15.0,
                    math.atan2(by - ay, bx - ax),
                ),
            )
        )
        mx, my = (ax + bx) / 2, (ay + by) / 2
        for t in (-0.18, 0.18):
            tx, ty = mx + (bx - ax) * t, my + (by - ay) * t
            parts.append((stone, g.cylinder(tx, ty, -2.0, 4.0, 20.0, 10)))
            parts.append(
                (slate, g.cone(tx, ty, 18.0, 4.4, 9.0 if not residence else 13.0, 10))
            )
    for x, y in corners:
        parts.append((stone, g.cylinder(x, y, -2.0, 5.2, 24.0, 12)))
        parts.append(
            (slate, g.cone(x, y, 22.0, 5.6, 11.0 if not residence else 16.0, 12))
        )
    # Great tower (grosse tour) with its own moat.
    parts.append(
        (
            water,
            g.flat(
                [
                    (9.5 * math.cos(a), 9.5 * math.sin(a))
                    for a in [2 * math.pi * i / 16 for i in range(16)]
                ],
                0.25,
            ),
        )
    )
    parts.append((stone, g.cylinder(0, 0, -2.0, 7.5, 33.0, 16)))
    parts.append((slate, g.cone(0, 0, 31.0, 8.0, 16.0, 16)))
    parts.append(("Gold", g.box(0, 0, 46.5, 0.5, 0.5, 3.0)))
    if residence:
        # Charles V's lodgings along the four sides, tall roofs, stair tower and turrets.
        for cx, cy, length, angle in (
            (0.0, -hy + 7.0, 2 * hx - 12, 0.0),
            (0.0, hy - 7.0, 2 * hx - 12, 0.0),
            (-hx + 7.0, 0.0, 2 * hy - 26, math.pi / 2),
            (hx - 7.0, 0.0, 2 * hy - 26, math.pi / 2),
        ):
            parts.append((stone, g.box(cx, cy, -2.0, length, 10.0, 22.0, angle)))
            parts.append((slate, g.gable_roof(cx, cy, 20.0, length, 10.0, 12.0, angle)))
        for x, y in (
            (-hx + 12, -hy + 12),
            (hx - 12, hy - 12),
            (-hx + 12, hy - 12),
            (hx - 12, -hy + 12),
        ):
            parts.append((stone, g.cylinder(x, y, 18.0, 2.2, 16.0, 8)))
            parts.append((slate, g.cone(x, y, 34.0, 2.5, 8.0, 8)))
    else:
        for cx, cy, length, angle in (
            (0.0, -hy + 6.0, 2 * hx - 14, 0.0),
            (-hx + 6.0, 0.0, 2 * hy - 26, math.pi / 2),
        ):
            parts.append((stone, g.box(cx, cy, -2.0, length, 8.0, 12.0, angle)))
            parts.append((slate, g.gable_roof(cx, cy, 10.0, length, 8.0, 7.0, angle)))
    return parts


# --- Châtelet, Bastille, Halles -------------------------------------------------------


def chatelet(size=1.0):
    """Bridgehead fortress: block with round towers and a dark gate passage."""
    s = size
    parts = [
        ("Stone", g.box(0, 0, -2.0, 30 * s, 24 * s, 17 * s)),
        ("Slate", g.gable_roof(0, 0, 15 * s, 30 * s, 24 * s, 9 * s)),
        ("Dark", g.panel(0, -12 * s, 0.0, 5 * s, 7 * s, -math.pi / 2, 0.2)),
        ("Dark", g.panel(0, 12 * s, 0.0, 5 * s, 7 * s, math.pi / 2, 0.2)),
    ]
    for x in (-15 * s, 15 * s):
        for y in (-12 * s, 12 * s):
            parts.append(("Stone", g.cylinder(x, y, -2.0, 5 * s, 24 * s, 10)))
            parts.append(("Slate", g.cone(x, y, 22 * s, 5.4 * s, 11 * s, 10)))
    return parts


def bastille():
    """Eight round towers joined by curtain walls of the same height, flat crenellated tops."""
    parts = []
    hx, hy, h = 33.0, 15.0, 24.0
    parts.append(
        (
            "Water",
            g.flat(
                [
                    (-hx - 16, -hy - 16),
                    (hx + 16, -hy - 16),
                    (hx + 16, hy + 16),
                    (-hx - 16, hy + 16),
                ],
                0.1,
            ),
        )
    )
    parts.append(("DarkStone", g.box(0, 0, -2.0, 2 * hx, 2 * hy, h + 2.0)))
    parts.append(
        (
            "Paving",
            g.flat(
                [
                    (-hx + 4, -hy + 4),
                    (hx - 4, -hy + 4),
                    (hx - 4, hy - 4),
                    (-hx + 4, hy - 4),
                ],
                h + 0.3,
            ),
        )
    )
    towers = [
        (-hx, -hy),
        (-11.0, -hy),
        (11.0, -hy),
        (hx, -hy),
        (hx, hy),
        (11.0, hy),
        (-11.0, hy),
        (-hx, hy),
    ]
    for x, y in towers:
        parts.append(("DarkStone", g.cylinder(x, y, -2.0, 6.5, h + 4.0, 12)))
        for i in range(8):
            a = 2 * math.pi * i / 8
            parts.append(
                (
                    "DarkStone",
                    g.box(
                        x + 6.0 * math.cos(a),
                        y + 6.0 * math.sin(a),
                        h + 2.0,
                        1.4,
                        1.4,
                        1.6,
                    ),
                )
            )
    for x in range(-28, 29, 4):
        for y in (-hy, hy):
            parts.append(("DarkStone", g.box(float(x), y, h, 1.4, 1.4, 1.5)))
    return parts


def market_halls():
    """Three long timber market halls under huge tiled roofs."""
    parts = []
    for y in (-17.0, 0.0, 17.0):
        parts.append(("Timber", g.box(0, y, -2.0, 80.0, 13.0, 7.0)))
        parts.append(("Tile", g.gable_roof(0, y, 5.0, 80.0, 13.0, 10.0, overhang=0.8)))
    return parts


# --- Parametric churches, abbeys, keeps, towers, mills ---------------------------------


def church(size=1.0, tower_spire=True):
    """Parish church: nave, choir with apse, west tower and slate spire."""
    s = size
    parts = [
        ("Stone", g.box(-4 * s, 0, -2.0, 36 * s, 13 * s, 17 * s)),
        ("Slate", g.gable_roof(-4 * s, 0, 15 * s, 36 * s, 13 * s, 9 * s)),
        ("Stone", g.box(20 * s, 0, -2.0, 12 * s, 10 * s, 15 * s)),
        ("Slate", g.gable_roof(20 * s, 0, 13 * s, 12 * s, 10 * s, 7 * s)),
        ("Stone", g.cylinder(26 * s, 0, -2.0, 5 * s, 15 * s, 8)),
        ("Slate", g.cone(26 * s, 0, 13 * s, 5.2 * s, 7 * s, 8)),
        ("Stone", g.box(-26 * s, 0, -2.0, 9 * s, 9 * s, 30 * s)),
    ]
    if tower_spire:
        parts.append(
            (
                "Slate",
                g.pyramid(
                    -26 * s, 0, 28 * s, 9.5 * s, 9.5 * s, 22 * s, math.pi / 4 * 0
                ),
            )
        )
    return parts


def abbey(size=1.0):
    """Abbey: large church, cloister with its galleries, conventual ranges, precinct wall."""
    s = size
    parts = church(1.35 * s)
    cx, cy, half = 2 * s, -28 * s, 14 * s
    parts.append(
        (
            "Garden",
            g.flat(
                [
                    (cx - half, cy - half),
                    (cx + half, cy - half),
                    (cx + half, cy + half),
                    (cx - half, cy + half),
                ],
                0.12,
            ),
        )
    )
    for angle, (ox, oy) in (
        (0.0, (0, -half - 5 * s)),
        (math.pi / 2, (half + 5 * s, 0)),
        (math.pi / 2, (-half - 5 * s, 0)),
    ):
        parts.append(
            (
                "Stone",
                g.box(cx + ox, cy + oy, -2.0, 2 * half + 20 * s, 10 * s, 12 * s, angle),
            )
        )
        parts.append(
            (
                "Tile",
                g.gable_roof(
                    cx + ox, cy + oy, 10 * s, 2 * half + 20 * s, 10 * s, 7 * s, angle
                ),
            )
        )
    wall = [(-55 * s, -62 * s), (48 * s, -62 * s), (48 * s, 22 * s), (-55 * s, 22 * s)]
    for i in range(4):
        (ax, ay), (bx, by) = wall[i], wall[(i + 1) % 4]
        parts.append(
            (
                "Stone",
                g.box(
                    (ax + bx) / 2,
                    (ay + by) / 2,
                    -2.0,
                    math.hypot(bx - ax, by - ay),
                    1.6 * s,
                    7 * s,
                    math.atan2(by - ay, bx - ax),
                ),
            )
        )
    parts.append(
        (
            "Garden",
            g.flat(
                [
                    (-50 * s, -58 * s),
                    (-20 * s, -58 * s),
                    (-20 * s, -30 * s),
                    (-50 * s, -30 * s),
                ],
                0.1,
            ),
        )
    )
    return parts


def keep(size=1.0):
    """Square great tower with four corner turrets (tour du Temple) and a small enclosure."""
    s = size
    h = 45 * s
    parts = [
        ("Stone", g.box(0, 0, -2.0, 18 * s, 18 * s, h + 2.0)),
        ("Slate", g.pyramid(0, 0, h, 18 * s, 18 * s, 14 * s)),
    ]
    for dx in (-9 * s, 9 * s):
        for dy in (-9 * s, 9 * s):
            parts.append(("Stone", g.cylinder(dx, dy, -2.0, 2.8 * s, h + 5 * s, 8)))
            parts.append(("Slate", g.cone(dx, dy, h + 3 * s, 3.1 * s, 10 * s, 8)))
    parts.append(("Stone", g.box(-25 * s, 0, -2.0, 24 * s, 12 * s, 12 * s)))
    parts.append(("Slate", g.gable_roof(-25 * s, 0, 10 * s, 24 * s, 12 * s, 8 * s)))
    return parts


def tower(size=1.0):
    """Round tower with a conical roof and a stair turret (tour de Nesle)."""
    s = size
    return [
        ("Stone", g.cylinder(0, 0, -2.0, 5.5 * s, 27 * s, 12)),
        ("Slate", g.cone(0, 0, 25 * s, 5.9 * s, 12 * s, 12)),
        ("Stone", g.cylinder(4.5 * s, 3 * s, -2.0, 2.0 * s, 33 * s, 8)),
        ("Slate", g.cone(4.5 * s, 3 * s, 31 * s, 2.2 * s, 6 * s, 8)),
    ]


def windmill(size=1.0):
    """Post mill on a trestle mound with four sails."""
    s = size
    parts = [
        ("Dirt", g.cone(0, 0, -1.0, 7 * s, 3.5 * s, 10, r_top=3 * s)),
        ("Wood", g.box(0, 0, 2.0 * s, 1.2 * s, 1.2 * s, 5 * s)),
        ("Timber", g.box(0, 0, 5 * s, 5 * s, 4 * s, 6 * s)),
        ("Wood", g.gable_roof(0, 0, 11 * s, 5 * s, 4 * s, 2 * s)),
    ]
    hub = (2.9 * s, 0.0, 9 * s)
    for i in range(4):
        a = math.pi / 4 + i * math.pi / 2
        tip = (2.9 * s, 10 * s * math.cos(a), 9 * s + 10 * s * math.sin(a))
        parts.append(("Canvas", g.beam(hub, tip, 1.8 * s, 0.3 * s)))
    return parts


# --- Reusable parametric templates (lot L2) -----------------------------------------------
#
# Every template takes a ``params`` dict (metres) from ``monuments[].params`` of the landmark
# JSON; missing keys fall back to the defaults written in the signature docstrings. A template
# may list ``variants`` in its params: ``{"variants": {"<name>": {...overrides}}}`` merged when the
# generator builds the ``<id>__<name>`` layer (``variant_from_year``).


def _p(params, key, default):
    """Parameter with default."""
    value = params.get(key, default)
    return default if value is None else value


def wall_segment(parts, a, b, z0, height, thick, mat="Stone", merlons=0.0):
    """Straight wall between two local points; ``merlons`` > 0 adds crenels of that size."""
    (ax, ay), (bx, by) = a, b
    length = math.hypot(bx - ax, by - ay)
    if length < 1e-6:
        return
    angle = math.atan2(by - ay, bx - ax)
    mx, my = (ax + bx) / 2, (ay + by) / 2
    parts.append((mat, g.box(mx, my, z0, length, thick, height - z0, angle)))
    if merlons > 0:
        count = max(1, int(length / (merlons * 2)))
        c, s = math.cos(angle), math.sin(angle)
        for i in range(count):
            t = (i + 0.5) / count - 0.5
            parts.append(
                (
                    mat,
                    g.box(
                        mx + c * t * length,
                        my + s * t * length,
                        height,
                        merlons,
                        thick * 1.05,
                        merlons * 0.9,
                        angle,
                    ),
                )
            )


def _flat_end_outline(radius, east, west, bays, steps):
    """Like ``_outline`` but with a flat east end (same vertex count, for the lean-to ring)."""
    points = []
    for i in range(bays + 1):
        points.append((west + (east - west) * i / bays, -radius))
    for i in range(1, steps):
        points.append((east, -radius + 2 * radius * i / steps))
    for i in range(bays, -1, -1):
        points.append((west + (east - west) * i / bays, radius))
    return points


def spire(parts, x, y, z, radius, height, mat="Lead", pinnacles=True, cross=True):
    """Octagonal needle with four corner pinnacles and a gilded cross."""
    parts.append((mat, g.cone(x, y, z, radius, height, 8)))
    if pinnacles:
        for i in range(4):
            a = math.pi / 4 + i * math.pi / 2
            parts.append(
                (
                    mat,
                    g.cone(
                        x + radius * 1.15 * math.cos(a),
                        y + radius * 1.15 * math.sin(a),
                        z - 1.0,
                        radius * 0.22,
                        height * 0.2,
                        6,
                    ),
                )
            )
    if cross:
        top = z + height
        parts.append(("Gold", g.box(x, y, top - 0.5, 0.5, 0.5, 4.0)))
        parts.append(("Gold", g.box(x, y, top + 2.0, 0.5, 2.4, 0.5)))


def square_tower(parts, x, y, size, z0, height, mat, top="flat", top_h=0.0, angle=0.0):
    """Square tower with buttressed corners; ``top``: flat (crenellated), pyramid, spire, turrets."""
    parts.append((mat, g.box(x, y, z0, size, size, height - z0, angle)))
    c, s = math.cos(angle), math.sin(angle)
    for dx in (-1, 1):
        for dy in (-1, 1):
            lx, ly = dx * size * 0.5, dy * size * 0.5
            parts.append(
                (
                    mat,
                    g.box(
                        x + lx * c - ly * s,
                        y + lx * s + ly * c,
                        z0,
                        size * 0.14,
                        size * 0.14,
                        height - z0 - size * 0.25,
                        angle,
                    ),
                )
            )
    # Belfry openings on the four faces.
    for k in range(4):
        a = angle + k * math.pi / 2
        parts.append(
            (
                "Dark",
                g.panel(
                    x + math.cos(a) * size / 2,
                    y + math.sin(a) * size / 2,
                    height - size * 0.95,
                    size * 0.28,
                    size * 0.45,
                    a,
                    0.15,
                ),
            )
        )
    if top == "flat":
        square = [
            (x + lx * c - ly * s, y + lx * s + ly * c)
            for lx, ly in (
                (-size / 2, -size / 2),
                (size / 2, -size / 2),
                (size / 2, size / 2),
                (-size / 2, size / 2),
            )
        ]
        for i in range(4):
            wall_segment(
                parts,
                square[i],
                square[(i + 1) % 4],
                height - 0.5,
                height + size * 0.08,
                size * 0.08,
                mat,
                merlons=size * 0.07,
            )
    elif top == "pyramid":
        parts.append(
            ("Slate", g.pyramid(x, y, height, size * 1.02, size * 1.02, top_h, angle))
        )
    elif top == "spire":
        parts.append(("Lead", g.cone(x, y, height, size * 0.5, top_h, 8)))
        for dx in (-1, 1):
            for dy in (-1, 1):
                lx, ly = dx * size * 0.42, dy * size * 0.42
                parts.append(
                    (
                        "Lead",
                        g.cone(
                            x + lx * c - ly * s,
                            y + lx * s + ly * c,
                            height,
                            size * 0.08,
                            top_h * 0.25,
                            6,
                        ),
                    )
                )
        parts.append(("Gold", g.box(x, y, height + top_h - 0.5, 0.4, 0.4, 3.0)))


def gothic_cathedral(params):
    """Parametric gothic cathedral (Old St Paul's, Westminster, Rouen, Bordeaux…).

    Params (metres): ``length_m`` 120 (west front to east end), ``nave_half`` 6.5,
    ``aisle_half`` 15, ``vault_m`` 28, ``ridge_m`` 38, ``aisle_wall_m`` 11, ``east`` "round" |
    "flat" (flat: great east window), ``transept_m`` 55 (north-south span), ``transept_x`` -5
    (crossing, from the centre), ``west_towers`` null | {``height``, ``size``, ``tops``: ["flat",
    "spire", ...] (south, north), ``spire_m``, ``heights``: [south, north]} ; ``crossing``
    null | {``height``, ``size``, ``spire_m``, ``lantern``}; ``transept_spires`` null |
    {``side``: 1 (north), ``height``, ``spire_m``}; ``flyers`` true; ``stone`` "NDStone";
    ``roof`` "Lead"; ``bays`` 10.
    """
    stone = _p(params, "stone", "NDStone")
    roof = _p(params, "roof", "Lead")
    length = _p(params, "length_m", 120.0)
    inner_r = _p(params, "nave_half", 6.5)
    outer_r = _p(params, "aisle_half", 15.0)
    vault = _p(params, "vault_m", 28.0)
    ridge = _p(params, "ridge_m", 38.0)
    aisle_wall = _p(params, "aisle_wall_m", 11.0)
    aisle_top = aisle_wall + (vault - aisle_wall) * 0.55
    round_east = _p(params, "east", "round") == "round"
    bays = int(_p(params, "bays", 10))
    steps = 8
    facade_depth = 12.0
    west = -length / 2
    body_west = west + facade_depth - 1.0
    east_x = length / 2 - (inner_r if round_east else 0.0)
    parts = []

    def outline(radius):
        if round_east:
            return _outline(radius, east_x, body_west, bays, steps)
        return _flat_end_outline(radius, east_x, body_west, bays, steps)

    inner = outline(inner_r)
    outer = outline(outer_r)
    if round_east:
        # The ambulatory reaches beyond the vessel's apse: shift the outer chevet east.
        outer = _outline(
            outer_r, east_x - (outer_r - inner_r) * 0.55, body_west, bays, steps
        )
    parts.append((stone, g.prism(inner, -2.0, vault, top=False)))
    parts.append((stone, g.prism(outer, -2.0, aisle_wall, top=False)))
    n = len(outer)
    ring = [(x, y, aisle_wall) for x, y in outer] + [
        (x, y, aisle_top) for x, y in inner
    ]
    parts.append((roof, (ring, [(i, i + 1, n + i + 1, n + i) for i in range(n - 1)])))
    straight = east_x - body_west
    parts.append(
        (
            roof,
            g.gable_roof(
                body_west + straight / 2, 0, vault, straight, inner_r * 2, ridge - vault
            ),
        )
    )
    if round_east:
        apse = [(east_x, 0.0, ridge)]
        for i in range(steps + 1):
            a = -math.pi / 2 + math.pi * i / steps
            apse.append((east_x + inner_r * math.cos(a), inner_r * math.sin(a), vault))
        parts.append((roof, (apse, [(i + 1, i + 2, 0) for i in range(steps)])))
    else:
        # Flat east end: gable and great window (Old St Paul's rose over seven lancets).
        parts.append(
            (stone, g.gable_roof(east_x, 0, vault, 0.8, inner_r * 2, ridge - vault))
        )
        parts.append(
            (
                "Glass",
                g.disc_vertical(east_x, 0, vault - 4.0, inner_r * 0.8, 0.0, 14, 0.3),
            )
        )
        for k in range(-2, 3):
            parts.append(
                (
                    "Glass",
                    g.panel(
                        east_x,
                        k * inner_r * 0.34,
                        aisle_wall * 0.5,
                        inner_r * 0.26,
                        vault * 0.45,
                        0.0,
                        0.3,
                    ),
                )
            )
        for side in (-1, 1):
            parts.append(
                (
                    stone,
                    g.cylinder(
                        east_x + 1.0, side * (inner_r + 1.0), -2.0, 1.8, ridge + 4.0, 8
                    ),
                )
            )
            parts.append(
                (
                    roof,
                    g.cone(
                        east_x + 1.0, side * (inner_r + 1.0), ridge + 2.0, 2.0, 8.0, 8
                    ),
                )
            )
    # Transept.
    tx = _p(params, "transept_x", -5.0)
    span = _p(params, "transept_m", 55.0)
    width = inner_r * 2.2
    if span > 0:
        parts.append((stone, g.box(tx, 0, -2.0, width, span, vault + 2.0)))
        parts.append(
            (roof, g.gable_roof(tx, 0, vault, span, width, ridge - vault, math.pi / 2))
        )
        for side in (-1, 1):
            normal = side * math.pi / 2
            parts.append(
                (
                    stone,
                    g.gable_roof(
                        tx,
                        side * span / 2,
                        vault,
                        0.8,
                        width,
                        ridge - vault,
                        math.pi / 2,
                    ),
                )
            )
            parts.append(
                (
                    "Glass",
                    g.disc_vertical(
                        tx,
                        side * span / 2,
                        vault - 6.0,
                        inner_r * 0.75,
                        normal,
                        14,
                        0.3,
                    ),
                )
            )
            parts.append(
                ("Dark", g.panel(tx, side * span / 2, 0.0, 4.0, 7.0, normal, 0.3))
            )
            for dx in (-1, 1):
                parts.append(
                    (
                        stone,
                        g.box(
                            tx + dx * width / 2,
                            side * (span / 2 + 0.6),
                            -2.0,
                            1.8,
                            1.8,
                            vault + 4.0,
                        ),
                    )
                )
                parts.append(
                    (
                        stone,
                        g.pyramid(
                            tx + dx * width / 2,
                            side * (span / 2 + 0.6),
                            vault + 2.0,
                            1.8,
                            1.8,
                            5.0,
                        ),
                    )
                )
    # Windows and flying buttresses.
    bay = straight / bays
    flyers = _p(params, "flyers", True)
    for i in range(bays):
        x = body_west + bay * (i + 0.5)
        if span > 0 and abs(x - tx) < width * 0.7:
            continue
        for side in (-1, 1):
            normal = side * math.pi / 2
            parts.append(
                (
                    "Glass",
                    g.panel(
                        x,
                        side * inner_r,
                        aisle_top + 1.0,
                        bay * 0.45,
                        (vault - aisle_top) * 0.62,
                        normal,
                        0.25,
                    ),
                )
            )
            parts.append(
                (
                    "Glass",
                    g.panel(
                        x,
                        side * outer_r,
                        aisle_wall * 0.2,
                        bay * 0.42,
                        aisle_wall * 0.55,
                        normal,
                        0.25,
                    ),
                )
            )
            if flyers and i > 0:
                bx = body_west + bay * i
                pier = side * (outer_r + 1.2)
                parts.append((stone, g.box(bx, pier, -2.0, 1.6, 2.6, aisle_top + 2.0)))
                parts.append(
                    (stone, g.pyramid(bx, pier, aisle_top + 1.0, 1.6, 2.6, 5.0))
                )
                parts.append(
                    (
                        stone,
                        g.beam(
                            (bx, side * (outer_r + 0.2), aisle_top),
                            (bx, side * (inner_r + 0.3), vault - 2.5),
                            1.0,
                            1.5,
                        ),
                    )
                )
    if round_east and flyers:
        for i in range(1, steps, 2):
            a = -math.pi / 2 + math.pi * i / steps
            c, s = math.cos(a), math.sin(a)
            cx0 = east_x - (outer_r - inner_r) * 0.55
            px, py = cx0 + (outer_r + 1.2) * c, (outer_r + 1.2) * s
            parts.append((stone, g.box(px, py, -2.0, 2.6, 1.6, aisle_top + 2.0, a)))
            parts.append((stone, g.pyramid(px, py, aisle_top + 1.0, 2.6, 1.6, 5.0, a)))
            parts.append(
                (
                    stone,
                    g.beam(
                        (cx0 + (outer_r + 0.2) * c, (outer_r + 0.2) * s, aisle_top),
                        (
                            east_x + (inner_r + 0.3) * c,
                            (inner_r + 0.3) * s,
                            vault - 2.5,
                        ),
                        1.0,
                        1.5,
                    ),
                )
            )
    # West front.
    wx = west + facade_depth / 2
    towers = params.get("west_towers")
    if towers:
        size = _p(towers, "size", outer_r * 0.9)
        heights = _p(towers, "heights", [_p(towers, "height", ridge + 25.0)] * 2)
        tops = _p(towers, "tops", ["flat", "flat"])
        spire_m = _p(towers, "spire_m", 30.0)
        parts.append(
            (stone, g.box(wx, 0, -2.0, facade_depth, outer_r * 2 - size, ridge + 4.0))
        )
        for k, side in enumerate((-1, 1)):
            ty = side * (outer_r - size / 2)
            square_tower(parts, wx, ty, size, -2.0, heights[k], stone, tops[k], spire_m)
            parts.append(
                (
                    "Dark",
                    g.panel(west, ty, 0.0, size * 0.35, size * 0.55, math.pi, 0.25),
                )
            )
        gable_w = outer_r * 2 - size
    else:
        parts.append(
            (stone, g.box(wx, 0, -2.0, facade_depth, outer_r * 2, aisle_wall + 3.0))
        )
        parts.append(
            (stone, g.box(wx, 0, -2.0, facade_depth, inner_r * 2.4, vault + 1.0))
        )
        for side in (-1, 1):
            for y in (side * inner_r * 1.25, side * outer_r):
                parts.append(
                    (
                        stone,
                        g.cylinder(
                            wx - facade_depth / 2 + 1.0, y, -2.0, 1.7, vault + 8.0, 8
                        ),
                    )
                )
                parts.append(
                    (
                        roof,
                        g.cone(
                            wx - facade_depth / 2 + 1.0, y, vault + 6.0, 1.9, 9.0, 8
                        ),
                    )
                )
        gable_w = inner_r * 2.4
    parts.append(
        (
            stone,
            g.gable_roof(
                west + 0.5,
                0,
                vault + (4.0 if towers else 1.0),
                1.0,
                gable_w,
                ridge - vault + 2.0,
            ),
        )
    )
    parts.append(
        (
            "Glass",
            g.disc_vertical(west, 0, vault * 0.72, inner_r * 0.75, math.pi, 16, 0.3),
        )
    )
    parts.append(
        ("Dark", g.panel(west, 0, 0.0, inner_r * 0.9, inner_r * 1.5, math.pi, 0.3))
    )
    for side in (-1, 1):
        parts.append(
            ("Dark", g.panel(west, side * (inner_r + 3.5), 0.0, 3.5, 6.0, math.pi, 0.3))
        )
    # Crossing tower and spire.
    crossing = params.get("crossing")
    if crossing:
        size = _p(crossing, "size", width * 1.05)
        height = _p(crossing, "height", ridge + 20.0)
        square_tower(
            parts,
            tx,
            0,
            size,
            vault,
            height,
            stone,
            "flat" if not crossing.get("spire_m") else "none",
        )
        if crossing.get("spire_m"):
            spire(
                parts,
                tx,
                0,
                height,
                size * 0.48,
                crossing["spire_m"],
                _p(crossing, "spire_mat", "Lead"),
            )
    # Spires on a transept arm (Bordeaux, north transept).
    tspires = params.get("transept_spires")
    if tspires and span > 0:
        side = _p(tspires, "side", 1)
        height = _p(tspires, "height", vault + 20.0)
        size = _p(tspires, "size", 7.0)
        for dx in (-1, 1):
            x = tx + dx * (width / 2 + size * 0.2)
            y = side * (span / 2 - size / 2)
            square_tower(parts, x, y, size, -2.0, height, stone, "none")
            spire(
                parts,
                x,
                y,
                height,
                size * 0.5,
                _p(tspires, "spire_m", 30.0),
                stone,
                cross=False,
            )
    return parts


def _round_tower(
    parts, x, y, r, height, mat, roof="cone", roof_mat="Slate", merlon=1.2
):
    """Round tower: conical roof or crenellated flat top."""
    parts.append((mat, g.cylinder(x, y, -2.0, r, height + 2.0, 10)))
    if roof == "cone":
        parts.append((roof_mat, g.cone(x, y, height, r * 1.08, r * 2.2, 10)))
    else:
        for i in range(8):
            a = 2 * math.pi * i / 8
            parts.append(
                (
                    mat,
                    g.box(
                        x + r * 0.9 * math.cos(a),
                        y + r * 0.9 * math.sin(a),
                        height,
                        merlon,
                        merlon,
                        merlon,
                    ),
                )
            )


def castle(params):
    """Parametric castle: concentric curtain rings with towers, a keep, moat, halls, gatehouses.

    Params (metres): ``rings``: [{``points`` [[x, y], …] closed polygon, ``height`` 10,
    ``thick`` 3, ``tower_radius`` 5, ``tower_height`` height × 1.35, ``tower_shape`` "round" |
    "square", ``tower_roof`` "cone" | "flat", ``extra_towers`` 0 (between corners)}] ;
    ``keep``: {``shape`` "square_turrets" (White Tower) | "round" | "square", ``at`` [0, 0],
    ``size`` [sx, sy] or radius, ``height``, ``mat``, ``roof`` "flat" | "cone" | "pyramid",
    ``angle``} ; ``moat``: {``points``, ``width``} (water band outside the polygon) ;
    ``halls``: [[x, y, sx, sy, height, angle_deg], …] ; ``gates``: [[x, y, angle_deg], …] ;
    ``stone`` "Stone" ; ``roof_mat`` "Slate" ; ``ward`` "Grass" (inner ground).
    """
    parts = []
    stone = _p(params, "stone", "Stone")
    roof_mat = _p(params, "roof_mat", "Slate")
    moat = params.get("moat")
    if moat:
        poly = g.ccw([tuple(p) for p in moat["points"]])
        outer = g.inset_polygon(poly, -moat.get("width", 20.0))
        n = len(poly)
        verts = [(x, y, 0.15) for x, y in outer] + [(x, y, 0.15) for x, y in poly]
        faces = [(i, (i + 1) % n, n + (i + 1) % n, n + i) for i in range(n)]
        parts.append(("Water", (verts, faces)))
    rings = params.get("rings", [])
    if rings:
        parts.append(
            (
                _p(params, "ward", "Grass"),
                g.flat([tuple(p) for p in rings[0]["points"]], 0.2),
            )
        )
    for ring in rings:
        pts = [tuple(p) for p in ring["points"]]
        height = ring.get("height", 10.0)
        thick = ring.get("thick", 3.0)
        tr = ring.get("tower_radius", 5.0)
        th = ring.get("tower_height", height * 1.35)
        shape = ring.get("tower_shape", "round")
        troof = ring.get("tower_roof", "cone")
        extra = int(ring.get("extra_towers", 0))
        for i in range(len(pts)):
            a, b = pts[i], pts[(i + 1) % len(pts)]
            wall_segment(parts, a, b, -2.0, height, thick, stone, merlons=thick * 0.45)
            towers = [a] + [
                (
                    a[0] + (b[0] - a[0]) * k / (extra + 1),
                    a[1] + (b[1] - a[1]) * k / (extra + 1),
                )
                for k in range(1, extra + 1)
            ]
            for x, y in towers:
                if shape == "round":
                    _round_tower(parts, x, y, tr, th, stone, troof, roof_mat)
                else:
                    angle = math.atan2(b[1] - a[1], b[0] - a[0])
                    square_tower(
                        parts,
                        x,
                        y,
                        tr * 1.8,
                        -2.0,
                        th,
                        stone,
                        "flat" if troof == "flat" else "pyramid",
                        tr * 1.6,
                        angle,
                    )
    for hall in params.get("halls", []):
        x, y, sx, sy, h, angle = hall
        angle = math.radians(angle)
        parts.append((stone, g.box(x, y, -2.0, sx, sy, h + 2.0, angle)))
        parts.append((roof_mat, g.gable_roof(x, y, h, sx, sy, sy * 0.55, angle)))
    for gate in params.get("gates", []):
        x, y, angle = gate
        angle = math.radians(angle)
        c, s = math.cos(angle), math.sin(angle)
        parts.append((stone, g.box(x, y, -2.0, 14.0, 12.0, 18.0, angle)))
        parts.append(
            (
                "Dark",
                g.panel(x - c * 6.1, y - s * 6.1, 0.0, 4.0, 6.0, angle + math.pi, 0.1),
            )
        )
        for side in (-1, 1):
            _round_tower(
                parts, x - s * side * 7.5, y + c * side * 7.5, 5.0, 20.0, stone, "flat"
            )
    keep = params.get("keep")
    if keep:
        x, y = keep.get("at", [0.0, 0.0])
        h = keep.get("height", 27.0)
        mat = keep.get("mat", stone)
        shape = keep.get("shape", "square")
        angle = math.radians(keep.get("angle", 0.0))
        if shape == "round":
            r = keep.get("size", 8.0)
            _round_tower(parts, x, y, r, h, mat, keep.get("roof", "cone"), roof_mat)
        else:
            sx, sy = keep.get("size", [30.0, 30.0])
            parts.append((mat, g.box(x, y, -2.0, sx, sy, h + 2.0, angle)))
            c, s = math.cos(angle), math.sin(angle)
            corners = [
                (-sx / 2, -sy / 2),
                (sx / 2, -sy / 2),
                (sx / 2, sy / 2),
                (-sx / 2, sy / 2),
            ]
            world = [(x + lx * c - ly * s, y + lx * s + ly * c) for lx, ly in corners]
            for i in range(4):
                wall_segment(
                    parts,
                    world[i],
                    world[(i + 1) % 4],
                    h - 1.0,
                    h + 1.2,
                    1.4,
                    mat,
                    merlons=1.1,
                )
            # Pilaster strips and round-headed windows (Norman keep).
            for face in range(4):
                a = angle + face * math.pi / 2 - math.pi / 2
                length = sx if face % 2 == 0 else sy
                fx = x + math.cos(a) * (sy if face % 2 == 0 else sx) / 2
                fy = y + math.sin(a) * (sy if face % 2 == 0 else sx) / 2
                tx_, ty_ = -math.sin(a), math.cos(a)
                for k in range(-2, 3):
                    off = k * length / 5.5
                    for z in (h * 0.45, h * 0.72):
                        parts.append(
                            (
                                "Dark",
                                g.panel(
                                    fx + tx_ * off, fy + ty_ * off, z, 1.4, 2.6, a, 0.2
                                ),
                            )
                        )
            if shape == "square_turrets":
                for k, (wx, wy) in enumerate(world):
                    if k == 2:
                        # The round north-east stair turret of the White Tower.
                        parts.append((mat, g.cylinder(wx, wy, -2.0, 4.2, h + 9.0, 10)))
                        parts.append(("Lead", g.cone(wx, wy, h + 7.0, 4.4, 6.0, 10)))
                    else:
                        parts.append(
                            (mat, g.box(wx, wy, -2.0, 7.0, 7.0, h + 9.0, angle))
                        )
                        parts.append(
                            ("Lead", g.pyramid(wx, wy, h + 7.0, 7.4, 7.4, 5.0, angle))
                        )
                # Chapel apse (St John's) on the south-east corner.
                parts.append(
                    (
                        mat,
                        g.cylinder(
                            x + sx / 2 - 6.0, y - sy / 2 + 2.0, -2.0, 6.5, h + 2.0, 10
                        ),
                    )
                )
                parts.append(
                    (
                        "Lead",
                        g.gable_roof(x, y, h + 1.0, sx * 0.8, sy * 0.35, 3.0, angle),
                    )
                )
            elif keep.get("roof", "flat") == "pyramid":
                parts.append(
                    (roof_mat, g.pyramid(x, y, h, sx, sy, min(sx, sy) * 0.7, angle))
                )
    return parts


def belfry(params):
    """Civic belfry (Bruges, Rouen's Gros-Horloge…): stacked stages, top, optional cloth halls.

    Params (metres): ``stages``: [{``size`` 16, ``height`` 30, ``shape`` "square" | "octagon"}, …]
    (heights cumulative from the ground) ; ``top`` "spire" | "flat" | "pyramid" | "lantern" ;
    ``top_m`` 20 ; ``turrets`` true (corner turrets on the top stage) ; ``halls``
    {``length`` 80, ``width`` 45, ``height`` 12} (courtyard building behind the tower, +x) ;
    ``arch`` false (gate arch through the base, Gros-Horloge) ; ``clock`` true ; ``stone``
    "Stone" ; ``variants``.
    """
    stone = _p(params, "stone", "Stone")
    stages = _p(
        params,
        "stages",
        [{"size": 16.0, "height": 30.0}, {"size": 14.0, "height": 50.0}],
    )
    parts = []
    z = -2.0
    size = stages[0]["size"]
    for stage in stages:
        size = stage["size"]
        top = stage["height"]
        if stage.get("shape", "square") == "octagon":
            parts.append(
                (
                    stone,
                    g.cylinder(0, 0, z, size / 2 / math.cos(math.pi / 8), top - z, 8),
                )
            )
            for i in range(8):
                a = 2 * math.pi * (i + 0.5) / 8
                parts.append(
                    (
                        "Dark",
                        g.panel(
                            size / 2 * math.cos(a),
                            size / 2 * math.sin(a),
                            top - size * 1.1,
                            size * 0.2,
                            size * 0.7,
                            a,
                            0.2,
                        ),
                    )
                )
            # Corner turrets on the square stage below the octagon.
            for dx in (-1, 1):
                for dy in (-1, 1):
                    parts.append(
                        (
                            stone,
                            g.cylinder(
                                dx * size * 0.62,
                                dy * size * 0.62,
                                z - 4.0,
                                size * 0.1,
                                size * 0.7,
                                6,
                            ),
                        )
                    )
                    parts.append(
                        (
                            "Lead",
                            g.cone(
                                dx * size * 0.62,
                                dy * size * 0.62,
                                z - 4.0 + size * 0.7,
                                size * 0.11,
                                size * 0.4,
                                6,
                            ),
                        )
                    )
        else:
            square_tower(parts, 0, 0, size, z, top, stone, "none")
        # String course between stages.
        parts.append((stone, g.box(0, 0, top - 0.6, size * 1.08, size * 1.08, 0.8)))
        z = top
    top_kind = _p(params, "top", "spire")
    top_m = _p(params, "top_m", 20.0)
    if top_kind == "spire":
        spire(parts, 0, 0, z, size * 0.5, top_m, "Lead")
    elif top_kind == "pyramid":
        parts.append(("Slate", g.pyramid(0, 0, z, size * 1.02, size * 1.02, top_m)))
    elif top_kind == "lantern":
        parts.append(
            ("Lead", g.cone(0, 0, z, size * 0.52, top_m * 0.35, 8, r_top=size * 0.25))
        )
        parts.append(
            ("Lead", g.cylinder(0, 0, z + top_m * 0.35, size * 0.25, top_m * 0.25, 8))
        )
        parts.append(
            ("Lead", g.cone(0, 0, z + top_m * 0.6, size * 0.27, top_m * 0.4, 8))
        )
    else:
        for i in range(4):
            a = i * math.pi / 2
            c, s = math.cos(a), math.sin(a)
            wall_segment(
                parts,
                (c * size / 2 - s * size / 2, s * size / 2 + c * size / 2),
                (c * size / 2 + s * size / 2, s * size / 2 - c * size / 2),
                z - 0.5,
                z + 1.5,
                1.0,
                stone,
                merlons=1.0,
            )
    if _p(params, "turrets", True):
        for dx in (-1, 1):
            for dy in (-1, 1):
                parts.append(
                    (
                        stone,
                        g.cylinder(
                            dx * size * 0.5, dy * size * 0.5, z - 6.0, 1.3, 8.0, 8
                        ),
                    )
                )
                parts.append(
                    (
                        "Lead",
                        g.cone(dx * size * 0.5, dy * size * 0.5, z + 2.0, 1.4, 5.0, 8),
                    )
                )
    if _p(params, "clock", True):
        base = stages[0]
        for a in (math.pi, 0.0):
            parts.append(
                (
                    "Gold",
                    g.disc_vertical(
                        math.cos(a) * base["size"] / 2,
                        0,
                        base["height"] * 0.85,
                        base["size"] * 0.22,
                        a,
                        12,
                        0.3,
                    ),
                )
            )
    if _p(params, "arch", False):
        base = stages[0]
        for a in (-math.pi / 2, math.pi / 2):
            parts.append(
                (
                    "Dark",
                    g.panel(
                        0,
                        math.sin(a) * base["size"] / 2,
                        0.0,
                        base["size"] * 0.5,
                        base["size"] * 0.4,
                        a,
                        0.3,
                    ),
                )
            )
        # The pavilion over the arch spanning the street.
        parts.append(
            (
                stone,
                g.box(
                    0,
                    base["size"] * 0.9,
                    4.0,
                    base["size"] * 0.7,
                    base["size"] * 1.2,
                    7.0,
                ),
            )
        )
        parts.append(
            (
                "Slate",
                g.gable_roof(
                    0,
                    base["size"] * 0.9,
                    11.0,
                    base["size"] * 1.2,
                    base["size"] * 0.7,
                    5.0,
                    math.pi / 2,
                ),
            )
        )
    halls = params.get("halls")
    if halls:
        hl, hw, hh = (
            _p(halls, "length", 80.0),
            _p(halls, "width", 45.0),
            _p(halls, "height", 12.0),
        )
        base = stages[0]["size"]
        x0 = base / 2
        cx = x0 + hl / 2
        for y in (-hw / 2 + 5.0, hw / 2 - 5.0):
            parts.append(
                (
                    "Brick" if stone == "Brick" else stone,
                    g.box(cx, y, -2.0, hl, 10.0, hh + 2.0),
                )
            )
            parts.append(
                ("Slate", g.gable_roof(cx, y, hh, hl, 10.0, 8.0, overhang=0.5))
            )
        for x in (x0 + 5.0, x0 + hl - 5.0):
            parts.append((stone, g.box(x, 0, -2.0, 10.0, hw - 20.0, hh + 2.0)))
            parts.append(
                ("Slate", g.gable_roof(x, 0, hh, hw - 20.0, 10.0, 8.0, math.pi / 2))
            )
        parts.append(
            (
                "Paving",
                g.flat(
                    [
                        (x0 + 10, -hw / 2 + 10),
                        (x0 + hl - 10, -hw / 2 + 10),
                        (x0 + hl - 10, hw / 2 - 10),
                        (x0 + 10, hw / 2 - 10),
                    ],
                    0.2,
                ),
            )
        )
    return parts


def royal_palace(params):
    """Royal palace around a great hall (Westminster): hall, palatine chapel, lodgings, towers.

    Params (metres): ``hall`` [length 73, width 21, wall 14, roof 16] ; ``hall_towers`` false
    (twin towers on the north gable, Richard II's rebuilding) ; ``chapel_at`` [x, y] (St
    Stephen's, a Sainte-Chapelle) ; ``chapel_angle`` 0 ; ``ranges``: [[x, y, sx, sy, h,
    angle_deg], …] ; ``towers``: [[x, y, size, height], …] (square, pyramid top) ; ``wall``
    [[x, y], …] precinct polygon ; ``variants``.
    """
    stone = _p(params, "stone", "Stone")
    parts = []
    length, width, wall, roof = _p(params, "hall", [73.0, 21.0, 14.0, 16.0])
    # The hall runs along +x (south to north at Westminster).
    parts.append((stone, g.box(0, 0, -2.0, length, width, wall + 2.0)))
    parts.append(("Lead", g.gable_roof(0, 0, wall, length, width, roof, overhang=0.6)))
    for i in range(7):
        x = -length / 2 + length * (i + 0.5) / 7
        for side in (-1, 1):
            parts.append(
                (
                    "Glass",
                    g.panel(
                        x,
                        side * width / 2,
                        wall * 0.45,
                        2.6,
                        wall * 0.4,
                        side * math.pi / 2,
                        0.2,
                    ),
                )
            )
            parts.append(
                (
                    stone,
                    g.box(
                        x + length / 14,
                        side * (width / 2 + 0.8),
                        -2.0,
                        1.4,
                        1.8,
                        wall + 1.0,
                    ),
                )
            )
    parts.append(
        (
            "Glass",
            g.disc_vertical(
                length / 2, 0, wall + roof * 0.35, width * 0.18, 0.0, 12, 0.3
            ),
        )
    )
    parts.append(("Dark", g.panel(length / 2, 0, 0.0, 4.5, 7.0, 0.0, 0.3)))
    if _p(params, "hall_towers", False):
        for side in (-1, 1):
            square_tower(
                parts,
                length / 2 + 2.0,
                side * (width / 2 - 1.0),
                7.0,
                -2.0,
                wall + roof + 2.0,
                stone,
                "flat",
            )
    chapel = params.get("chapel_at")
    if chapel:
        angle = math.radians(_p(params, "chapel_angle", 0.0))
        c, s = math.cos(angle), math.sin(angle)
        for mat, (verts, faces) in sainte_chapelle():
            parts.append(
                (
                    mat,
                    (
                        [
                            (chapel[0] + x * c - y * s, chapel[1] + x * s + y * c, z)
                            for x, y, z in verts
                        ],
                        faces,
                    ),
                )
            )
    for rng in params.get("ranges", []):
        x, y, sx, sy, h, angle = rng
        angle = math.radians(angle)
        parts.append((stone, g.box(x, y, -2.0, sx, sy, h + 2.0, angle)))
        parts.append(("Slate", g.gable_roof(x, y, h, sx, sy, sy * 0.6, angle)))
    for x, y, size, h in params.get("towers", []):
        square_tower(parts, x, y, size, -2.0, h, stone, "pyramid", size * 1.1)
    precinct = params.get("wall")
    if precinct:
        pts = [tuple(p) for p in precinct]
        for i in range(len(pts)):
            wall_segment(
                parts,
                pts[i],
                pts[(i + 1) % len(pts)],
                -2.0,
                7.0,
                1.6,
                stone,
                merlons=0.9,
            )
        parts.append(("Paving", g.flat(pts, 0.15)))
    return parts


def gate_tower(params):
    """Town gate with two round towers and a gatehouse (Grosse Cloche, Porte de Calais…).

    Params (metres): ``width`` 18 ; ``height`` 20 ; ``tower_radius`` 5 ; ``tower_height`` 26 ;
    ``cones`` true ; ``bell`` false (belfry lantern over the gatehouse) ; ``clock`` false ;
    ``stone`` "Stone".
    """
    stone = _p(params, "stone", "Stone")
    w = _p(params, "width", 18.0)
    h = _p(params, "height", 20.0)
    tr = _p(params, "tower_radius", 5.0)
    th = _p(params, "tower_height", 26.0)
    parts = [(stone, g.box(0, 0, -2.0, w * 0.7, w, h + 2.0))]
    parts.append(("Slate", g.gable_roof(0, 0, h, w, w * 0.7, 6.0, math.pi / 2)))
    for a in (0.0, math.pi):
        parts.append(
            ("Dark", g.panel(math.cos(a) * w * 0.35, 0, 0.0, 4.5, 6.5, a, 0.2))
        )
    for side in (-1, 1):
        _round_tower(
            parts,
            0,
            side * w / 2,
            tr,
            th,
            stone,
            "cone" if _p(params, "cones", True) else "flat",
        )
    if _p(params, "bell", False):
        parts.append(("Wood", g.box(0, 0, h + 4.0, 5.0, 5.0, 5.0)))
        parts.append(("Gold", g.box(0, 0, h + 5.0, 2.0, 2.0, 2.2)))
        parts.append(("Slate", g.cone(0, 0, h + 9.0, 3.8, 7.0, 8)))
    if _p(params, "clock", False):
        for a in (0.0, math.pi):
            parts.append(
                (
                    "Gold",
                    g.disc_vertical(
                        math.cos(a) * w * 0.36, 0, h * 0.7, 2.2, a, 12, 0.2
                    ),
                )
            )
    return parts


def papal_palace(variant=""):
    """Palais des Papes, Avignon: Palais Vieux, then Palais Neuf (``clement_vi`` variant).

    Benedict XII's Palais Vieux (1335-1342) around its cloister; Clement VI's Palais Neuf
    (1342-1352) added from the ``clement_vi`` variant. Local frame: +x east, +y north (the palace is set square to the compass on the Rocher des
    Doms), origin at the centre of the whole palace. Tall sheer walls with deep machicolated
    arcades, flat crenellated tops, square towers, warm limestone.
    """
    stone = "Ochre"
    parts = []

    def block(x0, y0, x1, y1, h, arcades=True):
        """Wing between two corners, crenellated, with arcades on its outer faces."""
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        sx, sy = abs(x1 - x0), abs(y1 - y0)
        parts.append((stone, g.box(cx, cy, -2.0, sx, sy, h + 2.0)))
        corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        for i in range(4):
            wall_segment(
                parts,
                corners[i],
                corners[(i + 1) % 4],
                h - 0.5,
                h + 1.3,
                1.4,
                stone,
                merlons=1.2,
            )
        if arcades:
            for normal, (ax, ay), (bx, by) in (
                (-math.pi / 2, (x0, y0), (x1, y0)),
                (0.0, (x1, y0), (x1, y1)),
                (math.pi / 2, (x1, y1), (x0, y1)),
                (math.pi, (x0, y1), (x0, y0)),
            ):
                span = math.hypot(bx - ax, by - ay)
                count = max(1, int(span / 9.0))
                for k in range(count):
                    t = (k + 0.5) / count
                    px, py = ax + (bx - ax) * t, ay + (by - ay) * t
                    parts.append(
                        (
                            "Dark",
                            g.panel(
                                px,
                                py,
                                h * 0.35,
                                span / count * 0.55,
                                h * 0.45,
                                normal,
                                0.15,
                            ),
                        )
                    )
                    # Pilasters carrying the great arcades.
                    qx, qy = ax + (bx - ax) * k / count, ay + (by - ay) * k / count
                    parts.append(
                        (
                            stone,
                            g.box(
                                qx + math.cos(normal) * 0.8,
                                qy + math.sin(normal) * 0.8,
                                -2.0,
                                2.2,
                                2.2,
                                h + 2.0,
                            ),
                        )
                    )

    def tower(x, y, sx, sy, h):
        """Square tower, flat crenellated top with a little watch turret."""
        parts.append((stone, g.box(x, y, -2.0, sx, sy, h + 2.0)))
        corners = [
            (x - sx / 2, y - sy / 2),
            (x + sx / 2, y - sy / 2),
            (x + sx / 2, y + sy / 2),
            (x - sx / 2, y + sy / 2),
        ]
        for i in range(4):
            wall_segment(
                parts,
                corners[i],
                corners[(i + 1) % 4],
                h - 0.5,
                h + 1.5,
                1.4,
                stone,
                merlons=1.3,
            )
        for k in range(3):
            z = h * (0.3 + 0.22 * k)
            for normal, ox, oy in (
                (0.0, sx / 2, 0),
                (math.pi, -sx / 2, 0),
                (math.pi / 2, 0, sy / 2),
                (-math.pi / 2, 0, -sy / 2),
            ):
                parts.append(
                    ("Dark", g.panel(x + ox, y + oy, z, 1.4, 2.8, normal, 0.2))
                )

    # Palais Vieux: four wings around the cloister (north-east part).
    block(-10.0, 30.0, 45.0, 42.0, 24.0)  # north: chapel wing (Benedict XII's chapel)
    block(33.0, -8.0, 45.0, 30.0, 22.0)  # east: Consistory / Grand Tinel wing
    block(-10.0, -8.0, 33.0, 2.0, 20.0)  # south: Conclave wing
    block(-10.0, 2.0, 0.0, 30.0, 20.0)  # west: Familiars wing
    parts.append(
        ("Paving", g.flat([(0.0, 2.0), (33.0, 2.0), (33.0, 30.0), (0.0, 30.0)], 0.2))
    )
    tower(40.0, 50.0, 17.0, 17.0, 52.0)  # Tour de Trouillas
    tower(49.0, 6.0, 16.0, 20.0, 46.0)  # Tour des Anges (papal tower)
    tower(38.0, -12.0, 10.0, 10.0, 38.0)  # Tour de la Campane
    tower(-14.0, 44.0, 10.0, 10.0, 34.0)  # Tour de la Glacière
    if variant == "clement_vi":
        # Palais Neuf: Grande Audience below the Clementine chapel (south wing), west wing with
        # the Champeaux gate, the great courtyard between the two palaces.
        block(-52.0, -52.0, 20.0, -34.0, 34.0)
        parts.append(("Lead", g.gable_roof(-16.0, -43.0, 34.0, 70.0, 16.0, 7.0)))
        block(-60.0, -34.0, -46.0, 30.0, 26.0)
        parts.append(
            (
                "Paving",
                g.flat(
                    [(-46.0, -34.0), (-10.0, -34.0), (-10.0, 30.0), (-46.0, 30.0)], 0.2
                ),
            )
        )
        tower(26.0, -48.0, 13.0, 13.0, 42.0)  # Tour Saint-Laurent
        tower(52.0, -10.0, 12.0, 12.0, 44.0)  # Tour de la Garde-Robe
        tower(-60.0, -44.0, 12.0, 12.0, 40.0)  # Tour de la Gâche
        # Champeaux gate: two turrets with pointed roofs on the west facade.
        for y in (-6.0, 6.0):
            parts.append((stone, g.cylinder(-61.0, y, 12.0, 2.2, 20.0, 8)))
            parts.append(("Slate", g.cone(-61.0, y, 32.0, 2.4, 9.0, 8)))
        parts.append(("Dark", g.panel(-60.0, 0.0, 0.0, 5.0, 8.0, math.pi, 1.2)))
    return parts


def cog(params):
    """Cog (single-mast merchant or war ship) at anchor: clinker hull, castles, square sail.

    Params (metres): ``length`` 25 ; ``sail`` true (furled when false).
    """
    length = _p(params, "length", 25.0)
    beam = length * 0.3
    parts = []
    hull = [
        (-length / 2, 0.0),
        (-length / 2 + 2.0, -beam / 2),
        (length / 2 - 3.0, -beam / 2),
        (length / 2, 0.0),
        (length / 2 - 3.0, beam / 2),
        (-length / 2 + 2.0, beam / 2),
    ]
    parts.append(("Wood", g.prism(hull, -1.0, 3.2)))
    parts.append(("Wood", g.box(-length / 2 + 3.5, 0, 3.2, 6.0, beam * 0.95, 2.4)))
    parts.append(("Wood", g.box(length / 2 - 3.0, 0, 3.2, 4.5, beam * 0.8, 2.0)))
    mast = length * 0.9
    parts.append(("Wood", g.cylinder(0, 0, 3.0, 0.35, mast, 6)))
    if _p(params, "sail", True):
        parts.append(
            (
                "Canvas",
                g.panel(
                    0.6,
                    0,
                    mast * 0.35,
                    beam * 1.6,
                    mast * 0.55,
                    0.0,
                    0.0,
                    pointed=False,
                ),
            )
        )
        parts.append(
            (
                "Canvas",
                g.panel(
                    0.6,
                    0,
                    mast * 0.35,
                    beam * 1.6,
                    mast * 0.55,
                    math.pi,
                    0.0,
                    pointed=False,
                ),
            )
        )
    parts.append(("Wood", g.box(0, 0, mast * 0.92, 0.4, beam * 1.7, 0.4)))
    return parts


def _with_variant(params, variant):
    """Params with the overrides of ``variants[variant]`` merged."""
    merged = dict(params or {})
    overrides = merged.pop("variants", {}).get(variant, {}) if variant else {}
    merged.pop("variants", None)
    merged.update(overrides)
    return merged


BUILDERS = {
    "notre_dame": lambda size, variant, params: notre_dame(),
    "sainte_chapelle": lambda size, variant, params: sainte_chapelle(),
    "palais_cite": lambda size, variant, params: palais_cite(),
    "louvre": lambda size, variant, params: louvre(variant),
    "chatelet": lambda size, variant, params: chatelet(size),
    "bastille": lambda size, variant, params: bastille(),
    "market_halls": lambda size, variant, params: market_halls(),
    "church": lambda size, variant, params: church(size),
    "abbey": lambda size, variant, params: abbey(size),
    "keep": lambda size, variant, params: keep(size),
    "tower": lambda size, variant, params: tower(size),
    "windmill": lambda size, variant, params: windmill(size),
    "gothic_cathedral": lambda size, variant, params: gothic_cathedral(
        _with_variant(params, variant)
    ),
    "castle": lambda size, variant, params: castle(_with_variant(params, variant)),
    "belfry": lambda size, variant, params: belfry(_with_variant(params, variant)),
    "royal_palace": lambda size, variant, params: royal_palace(
        _with_variant(params, variant)
    ),
    "gate_tower": lambda size, variant, params: gate_tower(
        _with_variant(params, variant)
    ),
    "papal_palace": lambda size, variant, params: papal_palace(variant),
    "cog": lambda size, variant, params: cog(_with_variant(params, variant)),
}
