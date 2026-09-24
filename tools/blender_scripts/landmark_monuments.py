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
                west_body + length / 2, 0, ND_VAULT, length, ND_INNER * 2, ND_RIDGE - ND_VAULT
            ),
        )
    )
    apse_verts = [(ND_APSE_X, 0.0, ND_RIDGE)]
    for i in range(apse_steps + 1):
        a = -math.pi / 2 + math.pi * i / apse_steps
        apse_verts.append(
            (ND_APSE_X + ND_INNER * math.cos(a), ND_INNER * math.sin(a), ND_VAULT)
        )
    parts.append(
        (lead, (apse_verts, [(i + 1, i + 2, 0) for i in range(apse_steps)]))
    )
    # Transept (barely protruding) with gabled roof and the north and south roses.
    tx0, tx1 = -5.0, 9.0
    tcx = (tx0 + tx1) / 2
    parts.append((stone, g.box(tcx, 0, -2.0, tx1 - tx0, 50.0, ND_VAULT + 2.0)))
    parts.append(
        (lead, g.gable_roof(tcx, 0, ND_VAULT, 50.0, tx1 - tx0, ND_RIDGE - ND_VAULT, math.pi / 2))
    )
    for side in (-1, 1):
        parts.append((glass, g.disc_vertical(tcx, side * 25.0, 24.0, 5.2, side * math.pi / 2, 14, 0.3)))
        parts.append((stone, g.disc_vertical(tcx, side * 25.0, 24.0, 1.4, side * math.pi / 2, 8, 0.5)))
        parts.append((glass, g.panel(tcx, side * 25.0, 0.0, 5.0, 8.0, side * math.pi / 2, 0.3)))
        parts.append((stone, g.pyramid(tcx, side * 25.3, ND_RIDGE - 1.0, 1.6, 1.2, 6.0)))

    # Clerestory lancets, aisle windows and the flying buttresses of each bay.
    nave_bays = [west_body + 3.5 + 7.2 * i for i in range(6)]
    choir_bays = [12.5 + 6.8 * i for i in range(4)]
    for x in nave_bays + choir_bays:
        for side in (-1, 1):
            normal = side * math.pi / 2
            parts.append((glass, g.panel(x + 3.4, side * ND_INNER, 23.5, 3.2, 6.0, normal, 0.25)))
            parts.append((glass, g.panel(x + 3.4, side * ND_OUTER, 2.5, 3.4, 6.0, normal, 0.25)))
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
    parts.append(("NDStone", g.beam((x, side * (ND_OUTER + 0.2), 18.0), (x, side * 15.0, 21.0), 0.9, 1.2)))
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
                g.panel(3.45 * math.cos(a) + x, 3.45 * math.sin(a), base + 3.0, 1.6, 6.0, a),
            )
        )
    parts.append(("Lead", g.cone(x, 0, base + 12.0, 3.6, 43.0, 8)))
    for i in range(4):
        a = 2 * math.pi * i / 4 + math.pi / 4
        parts.append(
            ("Lead", g.cone(x + 4.6 * math.cos(a), 4.6 * math.sin(a), base, 0.9, 16.0, 6))
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
            parts.append((stone, g.box(x, side * (half + 1.3), -2.0, 1.4, 2.6, wall + 1.0)))
            parts.append((stone, g.pyramid(x, side * (half + 1.3), wall - 1.0, 1.4, 2.6, 7.0)))
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
        parts.append((stone, g.cylinder(x0 - 0.5, side * (half + 0.4), -2.0, 1.6, 36.0, 8)))
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
    parts.append((slate, g.gable_roof(-52.0, 10.0, 12.0, 30.0, 22.0, 10.0, math.pi / 2)))
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
    parts.append(("Gold", g.panel(72.0, 22.4, 30.0, 4.0, 4.0, -math.pi / 2, 0.1, pointed=False)))
    # Precinct wall with its courtyard (the Sainte-Chapelle stands in the south-east part).
    wall = [(-68.0, -52.0), (88.0, -52.0), (88.0, 36.0), (-68.0, 36.0)]
    for i in range(4):
        (ax, ay), (bx, by) = wall[i], wall[(i + 1) % 4]
        if i == 2:
            continue  # the quay side is closed by the buildings
        length = math.hypot(bx - ax, by - ay)
        parts.append(
            (
                stone,
                g.box(
                    (ax + bx) / 2, (ay + by) / 2, -2.0, length, 2.0, 9.0, math.atan2(by - ay, bx - ax)
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
    parts.append((water, g.flat([(-hx - 12, -hy - 12), (hx + 12, -hy - 12), (hx + 12, hy + 12), (-hx - 12, hy + 12)], 0.1)))
    parts.append((stone, g.flat([(-hx - 3, -hy - 3), (hx + 3, -hy - 3), (hx + 3, hy + 3), (-hx - 3, hy + 3)], 0.2)))
    corners = [(-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy)]
    for i in range(4):
        (ax, ay), (bx, by) = corners[i], corners[(i + 1) % 4]
        parts.append(
            (
                stone,
                g.box((ax + bx) / 2, (ay + by) / 2, -2.0, math.hypot(bx - ax, by - ay), 3.0, 15.0, math.atan2(by - ay, bx - ax)),
            )
        )
        mx, my = (ax + bx) / 2, (ay + by) / 2
        for t in (-0.18, 0.18):
            tx, ty = mx + (bx - ax) * t, my + (by - ay) * t
            parts.append((stone, g.cylinder(tx, ty, -2.0, 4.0, 20.0, 10)))
            parts.append((slate, g.cone(tx, ty, 18.0, 4.4, 9.0 if not residence else 13.0, 10)))
    for x, y in corners:
        parts.append((stone, g.cylinder(x, y, -2.0, 5.2, 24.0, 12)))
        parts.append((slate, g.cone(x, y, 22.0, 5.6, 11.0 if not residence else 16.0, 12)))
    # Great tower (grosse tour) with its own moat.
    parts.append((water, g.flat([(9.5 * math.cos(a), 9.5 * math.sin(a)) for a in [2 * math.pi * i / 16 for i in range(16)]], 0.25)))
    parts.append((stone, g.cylinder(0, 0, -2.0, 7.5, 33.0, 16)))
    parts.append((slate, g.cone(0, 0, 31.0, 8.0, 16.0, 16)))
    parts.append(("Gold", g.box(0, 0, 46.5, 0.5, 0.5, 3.0)))
    if residence:
        # Charles V's lodgings along the four sides, tall roofs, stair tower and turrets.
        for (cx, cy, length, angle) in (
            (0.0, -hy + 7.0, 2 * hx - 12, 0.0),
            (0.0, hy - 7.0, 2 * hx - 12, 0.0),
            (-hx + 7.0, 0.0, 2 * hy - 26, math.pi / 2),
            (hx - 7.0, 0.0, 2 * hy - 26, math.pi / 2),
        ):
            parts.append((stone, g.box(cx, cy, -2.0, length, 10.0, 22.0, angle)))
            parts.append((slate, g.gable_roof(cx, cy, 20.0, length, 10.0, 12.0, angle)))
        for x, y in ((-hx + 12, -hy + 12), (hx - 12, hy - 12), (-hx + 12, hy - 12), (hx - 12, -hy + 12)):
            parts.append((stone, g.cylinder(x, y, 18.0, 2.2, 16.0, 8)))
            parts.append((slate, g.cone(x, y, 34.0, 2.5, 8.0, 8)))
    else:
        for (cx, cy, length, angle) in ((0.0, -hy + 6.0, 2 * hx - 14, 0.0), (-hx + 6.0, 0.0, 2 * hy - 26, math.pi / 2)):
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
    parts.append(("Water", g.flat([(-hx - 16, -hy - 16), (hx + 16, -hy - 16), (hx + 16, hy + 16), (-hx - 16, hy + 16)], 0.1)))
    parts.append(("DarkStone", g.box(0, 0, -2.0, 2 * hx, 2 * hy, h + 2.0)))
    parts.append(("Paving", g.flat([(-hx + 4, -hy + 4), (hx - 4, -hy + 4), (hx - 4, hy - 4), (-hx + 4, hy - 4)], h + 0.3)))
    towers = [(-hx, -hy), (-11.0, -hy), (11.0, -hy), (hx, -hy), (hx, hy), (11.0, hy), (-11.0, hy), (-hx, hy)]
    for x, y in towers:
        parts.append(("DarkStone", g.cylinder(x, y, -2.0, 6.5, h + 4.0, 12)))
        for i in range(8):
            a = 2 * math.pi * i / 8
            parts.append(("DarkStone", g.box(x + 6.0 * math.cos(a), y + 6.0 * math.sin(a), h + 2.0, 1.4, 1.4, 1.6)))
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
        parts.append(("Slate", g.pyramid(-26 * s, 0, 28 * s, 9.5 * s, 9.5 * s, 22 * s, math.pi / 4 * 0)))
    return parts


def abbey(size=1.0):
    """Abbey: large church, cloister with its galleries, conventual ranges, precinct wall."""
    s = size
    parts = church(1.35 * s)
    cx, cy, half = 2 * s, -28 * s, 14 * s
    parts.append(("Garden", g.flat([(cx - half, cy - half), (cx + half, cy - half), (cx + half, cy + half), (cx - half, cy + half)], 0.12)))
    for angle, (ox, oy) in ((0.0, (0, -half - 5 * s)), (math.pi / 2, (half + 5 * s, 0)), (math.pi / 2, (-half - 5 * s, 0))):
        parts.append(("Stone", g.box(cx + ox, cy + oy, -2.0, 2 * half + 20 * s, 10 * s, 12 * s, angle)))
        parts.append(("Tile", g.gable_roof(cx + ox, cy + oy, 10 * s, 2 * half + 20 * s, 10 * s, 7 * s, angle)))
    wall = [(-55 * s, -62 * s), (48 * s, -62 * s), (48 * s, 22 * s), (-55 * s, 22 * s)]
    for i in range(4):
        (ax, ay), (bx, by) = wall[i], wall[(i + 1) % 4]
        parts.append(("Stone", g.box((ax + bx) / 2, (ay + by) / 2, -2.0, math.hypot(bx - ax, by - ay), 1.6 * s, 7 * s, math.atan2(by - ay, bx - ax))))
    parts.append(("Garden", g.flat([(-50 * s, -58 * s), (-20 * s, -58 * s), (-20 * s, -30 * s), (-50 * s, -30 * s)], 0.1)))
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


BUILDERS = {
    "notre_dame": lambda size, variant: notre_dame(),
    "sainte_chapelle": lambda size, variant: sainte_chapelle(),
    "palais_cite": lambda size, variant: palais_cite(),
    "louvre": lambda size, variant: louvre(variant),
    "chatelet": lambda size, variant: chatelet(size),
    "bastille": lambda size, variant: bastille(),
    "market_halls": lambda size, variant: market_halls(),
    "church": lambda size, variant: church(size),
    "abbey": lambda size, variant: abbey(size),
    "keep": lambda size, variant: keep(size),
    "tower": lambda size, variant: tower(size),
    "windmill": lambda size, variant: windmill(size),
}
