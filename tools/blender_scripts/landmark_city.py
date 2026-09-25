"""Landmark city generator (lot L1): Paris first, from ``data/landmarks/<id>.json``.

Run headless:
    blender --background --python landmark_city.py -- <landmark.json> <out.glb> [--siege]

``--siege`` builds the backdrop of siege battles instead (real scale, metres, elements listed in
the ``siege`` block: the river, the island and the far bank behind the generic besieged town).

The plan (local metres, x east, y true north) is magnified by the radial warp of the
``scale`` block: exaggerated centre, map scale again at the edge of the reserved zone, so that the
river and the roads of the campaign map join it. Output units are map pixels (1 unit = 1 px),
true-north frame (Godot rotates by ``anchor.north_bearing_deg``), built at z = 0: Godot drapes every
vertex on the terrain using its anchor (first UV map = world XY of the vertex, or of the building it
belongs to, so that buildings stay rigid).

Nodes of the glTF (one per layer, all with the same materials):

* ``ground``: river, islands, street plates and main streets (always visible);
* ``houses``: detailed urban fabric (near); ``blocks``: the same blocks as simple masses (far);
* ``landmarks``: monuments, walls, bridges without dates;
* ``<element id>``: an element with ``from_year``/``until_year`` (Charles V's wall, the Bastille…);
  ``<id>__<variant>``: an element's variant from a year (the Louvre of Charles V).

A line ``LAYER <name> <triangles>`` is printed per layer and ``OK`` at the end.
"""

import json
import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import landmark_geometry as g  # noqa: E402
import landmark_monuments as monuments  # noqa: E402

# name: (base colour (linear RGB), roughness)
PALETTE = {
    "NDStone": ((0.64, 0.59, 0.48), 0.85),
    "Stone": ((0.42, 0.39, 0.33), 0.88),
    "DarkStone": ((0.30, 0.28, 0.24), 0.9),
    "WallStone": ((0.40, 0.37, 0.31), 0.9),
    "Lead": ((0.17, 0.19, 0.21), 0.45),
    "Slate": ((0.10, 0.11, 0.13), 0.6),
    "Tile": ((0.36, 0.14, 0.07), 0.8),
    "TileOld": ((0.27, 0.15, 0.09), 0.85),
    "Thatch": ((0.30, 0.23, 0.11), 0.97),
    "Plaster": ((0.58, 0.52, 0.41), 0.92),
    "Timber": ((0.40, 0.32, 0.22), 0.9),
    "Wood": ((0.20, 0.13, 0.08), 0.82),
    "Glass": ((0.03, 0.045, 0.07), 0.25),
    "Dark": ((0.035, 0.03, 0.028), 0.95),
    "Gold": ((0.78, 0.58, 0.20), 0.35),
    "Water": ((0.045, 0.09, 0.10), 0.12),
    "Grass": ((0.13, 0.19, 0.06), 0.95),
    "Garden": ((0.10, 0.16, 0.05), 0.95),
    "Dirt": ((0.30, 0.25, 0.17), 1.0),
    "Street": ((0.36, 0.31, 0.23), 1.0),
    "Paving": ((0.40, 0.37, 0.31), 0.95),
    "Canvas": ((0.66, 0.61, 0.49), 0.9),
    "Vine": ((0.16, 0.19, 0.07), 0.95),
    "Whitewash": ((0.78, 0.76, 0.70), 0.85),
    "Brick": ((0.42, 0.17, 0.09), 0.9),
    "Ochre": ((0.66, 0.53, 0.36), 0.88),
}

Z_WATER = 0.004
Z_GROUND = 0.008
Z_ISLAND = 0.010
Z_STREET = 0.012
LANE = 0.014  # half width of the lanes between blocks (world units)
CELL = 0.34  # spacing of the block seeds (world units)
PLAN_STEP = 30.0  # densification of plan lines before warping (metres)


class Plan:
    """Radial warp and the plan elements converted to world units."""

    def __init__(self, landmark, meters_per_px):
        """Initialise from the given parameters."""
        self.data = landmark
        scale = landmark["scale"]
        self.a = 1.0 / scale["center_meters_per_unit"]
        self.core_m = scale["core_radius_m"]
        self.core_px = scale["core_radius_px"]
        self.b = (self.core_px - self.a * self.core_m) / self.core_m**2
        self.zone_px = scale["zone_radius_px"]
        self.meters_per_px = meters_per_px
        self.zone_m = self.zone_px * meters_per_px
        self.monument_scale = scale["monument_scale"]
        self.house_scale = scale["house_scale"]
        self.height_scale = scale["height_scale"]

    def radius(self, r):
        """World radius (px) of a plan radius (m)."""
        if r <= self.core_m:
            return self.a * r + self.b * r * r
        if r <= self.zone_m:
            t = (r - self.core_m) / (self.zone_m - self.core_m)
            return self.core_px + t * (self.zone_px - self.core_px)
        return r / self.meters_per_px

    def warp(self, x, y):
        """Plan metres -> world units (true-north frame)."""
        r = math.hypot(x, y)
        if r < 1e-9:
            return (0.0, 0.0)
        f = self.radius(r) / r
        return (x * f, y * f)

    def local_scale(self, x, y):
        """Tangential scale (units per metre) at a plan point."""
        r = math.hypot(x, y)
        return self.a if r < 1e-6 else self.radius(r) / r

    def line(self, points, closed=False):
        """Densified and warped polyline."""
        return [self.warp(*p) for p in g.densify(points, PLAN_STEP, closed)]

    def polygon(self, points):
        """Densified and warped polygon (without the closing point)."""
        dense = g.densify(points, PLAN_STEP, closed=True)
        return [self.warp(*p) for p in dense]


class SiegePlan(Plan):
    """Siege backdrop: no magnifier, real proportions, plan rotated so that the river runs along x.

    Built at the map's centre scale (so that the world-unit constants of the generator keep their
    meaning) and scaled to metres at export (``export_scale``).
    """

    def __init__(self, landmark, meters_per_px):
        """Initialise from the given parameters."""
        super().__init__(landmark, meters_per_px)
        siege = landmark["siege"]
        self.monument_scale = 1.0
        self.house_scale = 1.0
        self.height_scale = 1.0
        self.zone_m = siege["radius_m"]
        self.zone_px = siege["radius_m"] * self.a
        angle = math.radians(siege["rotate_deg"])
        self.rot = (math.cos(angle), math.sin(angle))
        self.export_scale = 1.0 / self.a

    def radius(self, r):
        """Linear: no magnifier."""
        return self.a * r

    def warp(self, x, y):
        """Rotate then scale."""
        c, s = self.rot
        return ((x * c - y * s) * self.a, (x * s + y * c) * self.a)


def siege_subset(landmark):
    """Plan restricted to the elements of the siege backdrop."""
    siege = landmark["siege"]
    subset = dict(landmark)
    for key in ("areas", "monuments", "bridges", "streets", "walls"):
        keep = set(siege.get(key, []))
        subset[key] = [item for item in landmark.get(key, []) if item["id"] in keep]
    # Open spaces: only those on the islands or in the kept quarters (not on the besieged bank).
    zones = [area["polygon"] for area in subset["areas"]]
    zones += [island["polygon"] for island in landmark.get("islands", [])]

    def kept(space):
        poly = space["polygon"]
        cx = sum(p[0] for p in poly) / len(poly)
        cy = sum(p[1] for p in poly) / len(poly)
        return any(g.point_in_polygon(cx, cy, zone) for zone in zones)

    subset["open_spaces"] = [
        space for space in landmark.get("open_spaces", []) if kept(space)
    ]
    return subset


def active(item):
    """Elements without dates go to the shared layers."""
    return "from_year" not in item and "until_year" not in item


# --- River and islands -------------------------------------------------------------------


def river_banks(plan, river):
    """Left and right banks (world) of a river clipped to the reserved zone."""
    points = river["points"]
    dense = []
    for i in range(len(points) - 1):
        x0, y0, w0 = points[i]
        x1, y1, w1 = points[i + 1]
        n = max(1, int(math.ceil(math.hypot(x1 - x0, y1 - y0) / 25.0)))
        for k in range(n):
            t = k / n
            dense.append((x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, w0 + (w1 - w0) * t))
    dense.append(tuple(points[-1]))
    # Clip to the zone (plus a small overlap with the map's river).
    limit = plan.zone_m * 1.01
    inside = [p for p in dense if math.hypot(p[0], p[1]) <= limit]
    normals = g.polyline_normals([(p[0], p[1]) for p in inside])
    left, right = [], []
    for (x, y, w), (nx, ny) in zip(inside, normals, strict=True):
        left.append(plan.warp(x + nx * w / 2, y + ny * w / 2))
        right.append(plan.warp(x - nx * w / 2, y - ny * w / 2))
    return left, right


def build_ground(plan, layers, rng):
    """River, islands, open spaces as flat draped surfaces."""
    ground = layers["ground"]
    waters = []
    for river in plan.data.get("rivers", []):
        left, right = river_banks(plan, river)
        verts = [(x, y, Z_WATER) for x, y in left] + [(x, y, Z_WATER) for x, y in right]
        n = len(left)
        faces = [(i, n + i, n + i + 1, i + 1) for i in range(n - 1)]
        ground.add("Water", verts, faces)
        waters.append(left + list(reversed(right)))
    for water in plan.data.get("waters", []):
        poly = plan.polygon(water["polygon"])
        kind = water.get("kind", "water")
        ground.add("Garden" if kind == "marsh" else "Water", *g.flat(poly, Z_WATER))
        waters.append(poly)
    islands = []
    for island in plan.data.get("islands", []):
        poly = plan.polygon(island["polygon"])
        islands.append(poly)
        verts, faces = g.flat(poly, Z_ISLAND)
        ground.add("Grass", verts, faces)
        # Stone quays / banks: a low rim.
        ground.add("Paving", *g.prism(poly, Z_WATER - 0.02, Z_ISLAND, top=False))
    opens = []
    for space in plan.data.get("open_spaces", []):
        poly = plan.polygon(space["polygon"])
        kind = space.get("kind", "square")
        opens.append(poly)
        mat = {
            "meadow": "Grass",
            "garden": "Garden",
            "cemetery": "Garden",
            "vineyard": "Vine",
            "hill": "Grass",
        }.get(kind, "Paving")
        ground.add(mat, *g.flat(poly, Z_ISLAND + 0.002))
    return waters, islands, opens


class Site:
    """Queries on the warped plan: land, open spaces, monuments, walls, streets."""

    def __init__(self, plan, waters, islands, opens):
        """Initialise from the given parameters."""
        self.plan = plan
        self.waters = waters
        self.islands = islands
        self.opens = opens
        self.areas = []
        for area in plan.data.get("areas", []):
            self.areas.append((plan.polygon(area["polygon"]), area.get("density", 0.5)))
        self.clearings = []  # (x, y, radius)
        self.walls = []  # warped polylines
        self.streets = []  # (warped polyline, half width)
        self.bridges = []  # (a, b, half width)

    def on_land(self, x, y):
        """Outside the water or on an island."""
        if any(g.point_in_polygon(x, y, poly) for poly in self.islands):
            return True
        return not any(g.point_in_polygon(x, y, poly) for poly in self.waters)

    def density(self, x, y):
        """Built density at a world point (0 outside the quarters)."""
        for poly, density in self.areas:
            if g.point_in_polygon(x, y, poly):
                return density
        return 0.0

    def free(self, x, y, margin=0.0):
        """No open space, monument, wall, street or bridge here."""
        if any(g.point_in_polygon(x, y, poly) for poly in self.opens):
            return False
        for cx, cy, r in self.clearings:
            if (x - cx) ** 2 + (y - cy) ** 2 < (r + margin) ** 2:
                return False
        for line in self.walls:
            if g.distance_to_polyline(x, y, line) < 0.05 + margin:
                return False
        for line, half in self.streets:
            if g.distance_to_polyline(x, y, line) < half + 0.006 + margin:
                return False
        for a, b, half in self.bridges:
            if g.distance_to_segment(x, y, *a, *b) < half + 0.01 + margin:
                return False
        return True


# --- Streets, walls, bridges, monuments -----------------------------------------------


def build_streets(plan, site, layers):
    """Main streets as light strips; they also clear the houses."""
    for street in plan.data.get("streets", []):
        line = plan.line(street["points"])
        half = street.get("width_m", 10.0) * plan.a * 0.6
        site.streets.append((line, half))
        normals = g.polyline_normals(line)
        verts = [
            (x + nx * half, y + ny * half, Z_STREET)
            for (x, y), (nx, ny) in zip(line, normals, strict=True)
        ]
        verts += [
            (x - nx * half, y - ny * half, Z_STREET)
            for (x, y), (nx, ny) in zip(line, normals, strict=True)
        ]
        n = len(line)
        layers["ground"].add(
            "Street", verts, [(n + i, n + i + 1, i + 1, i) for i in range(n - 1)]
        )


def wall_units(plan, meters):
    """Height/size of fortifications: monument scale at the centre."""
    return meters * plan.a * plan.monument_scale


def build_wall(plan, site, wall, layer):
    """Curtain wall with towers every ``tower_spacing_m`` (plan metres), gates and ditch."""
    pts = wall["points"]
    line = plan.line(pts, wall.get("closed", False))
    site.walls.append(line)
    h = wall_units(plan, wall["height_m"]) * plan.height_scale
    thick = 0.022
    mat = "WallStone"
    for i in range(len(line) - 1):
        (ax, ay), (bx, by) = line[i], line[i + 1]
        length = math.hypot(bx - ax, by - ay)
        angle = math.atan2(by - ay, bx - ax)
        mid = ((ax + bx) / 2, (ay + by) / 2)
        layer.add(
            mat,
            *g.box(
                mid[0],
                mid[1],
                -g.FOUNDATION,
                length + thick,
                thick,
                h + g.FOUNDATION,
                angle,
            ),
            anchor=mid,
        )
        # Wall-walk parapet (slightly wider) for a crenellated silhouette.
        layer.add(
            mat,
            *g.box(mid[0], mid[1], h, length * 0.7, thick * 1.25, h * 0.12, angle),
            anchor=mid,
        )
    # Towers along the plan length.
    dense = g.densify(pts, 5.0, wall.get("closed", False))
    spacing = wall.get("tower_spacing_m", 80.0)
    travelled = 0.0
    next_tower = spacing / 2
    round_towers = wall.get("tower_shape", "round") == "round"
    tr = thick * 1.25
    for i in range(len(dense) - 1):
        (x0, y0), (x1, y1) = dense[i], dense[i + 1]
        travelled += math.hypot(x1 - x0, y1 - y0)
        if travelled >= next_tower:
            next_tower += spacing
            tx, ty = plan.warp(x1, y1)
            th = h * 1.35
            if round_towers:
                layer.add(
                    mat,
                    *g.cylinder(tx, ty, -g.FOUNDATION, tr, th + g.FOUNDATION, 8),
                    anchor=(tx, ty),
                )
                layer.add(
                    "Slate", *g.cone(tx, ty, th, tr * 1.1, tr * 2.2, 8), anchor=(tx, ty)
                )
            else:
                angle = math.atan2(y1 - y0, x1 - x0)
                layer.add(
                    mat,
                    *g.box(
                        tx,
                        ty,
                        -g.FOUNDATION,
                        tr * 1.8,
                        tr * 1.8,
                        th + g.FOUNDATION,
                        angle,
                    ),
                    anchor=(tx, ty),
                )
                layer.add(
                    mat,
                    *g.box(tx, ty, th, tr * 2.0, tr * 2.0, h * 0.14, angle),
                    anchor=(tx, ty),
                )
    # Gates: two towers and a gatehouse.
    for gate in wall.get("gates", []):
        gx, gy = plan.warp(*gate["at"])
        best = min(
            range(len(line) - 1),
            key=lambda k: g.distance_to_segment(gx, gy, *line[k], *line[k + 1]),
        )
        (ax, ay), (bx, by) = line[best], line[best + 1]
        angle = math.atan2(by - ay, bx - ax)
        c, s = math.cos(angle), math.sin(angle)
        gh = h * 1.6
        layer.add(
            mat,
            *g.box(gx, gy, -g.FOUNDATION, tr * 3.2, tr * 2.6, gh + g.FOUNDATION, angle),
            anchor=(gx, gy),
        )
        layer.add(
            "Slate",
            *g.hip_roof(gx, gy, gh, tr * 3.2, tr * 2.6, tr * 1.6, angle),
            anchor=(gx, gy),
        )
        for side in (-1, 1):
            px, py = gx + c * side * tr * 1.9, gy + s * side * tr * 1.9
            layer.add(
                mat,
                *g.cylinder(
                    px, py, -g.FOUNDATION, tr * 1.1, gh * 1.1 + g.FOUNDATION, 8
                ),
                anchor=(gx, gy),
            )
            layer.add(
                "Slate",
                *g.cone(px, py, gh * 1.1, tr * 1.2, tr * 2.4, 8),
                anchor=(gx, gy),
            )
    # Ditch: a water strip on the outer side.
    if wall.get("ditch", False):
        normals = g.polyline_normals(line)
        verts = []
        # Outer side = away from the centre: the enclosed area's centroid for a closed wall
        # (the anchor may lie outside the town, Calais), else the plan origin.
        ox, oy = (0.0, 0.0)
        if wall.get("closed", False):
            ox = sum(p[0] for p in line) / len(line)
            oy = sum(p[1] for p in line) / len(line)
        for (x, y), (nx, ny) in zip(line, normals, strict=True):
            sign = 1.0 if (nx * (x - ox) + ny * (y - oy)) > 0 else -1.0
            for offset in (0.035, 0.075):
                verts.append(
                    (x + sign * nx * offset, y + sign * ny * offset, Z_WATER + 0.004)
                )
        n = len(line)
        faces = [(2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2) for i in range(n - 1)]
        layer.add("Water", verts, faces)


def build_bridge(plan, site, bridge, layer, rng):
    """Stone or timber bridge; inhabited bridges carry two rows of houses."""
    a = plan.warp(*bridge["from"])
    b = plan.warp(*bridge["to"])
    length = math.hypot(b[0] - a[0], b[1] - a[1])
    angle = math.atan2(b[1] - a[1], b[0] - a[0])
    half = bridge["width_m"] * plan.a * 0.5
    site.bridges.append((a, b, half))
    mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    stone = bridge.get("material", "stone") == "stone"
    mat = "WallStone" if stone else "Wood"
    deck = 0.045 if "arches" in bridge else 0.03
    layer.add(
        mat,
        *g.box(mid[0], mid[1], deck - 0.012, length + 0.02, half * 2, 0.014, angle),
        anchor=mid,
    )
    layer.add(
        "Street",
        *g.box(mid[0], mid[1], deck + 0.002, length + 0.02, half * 1.2, 0.001, angle),
        anchor=mid,
    )
    c, s = math.cos(angle), math.sin(angle)
    piers = bridge.get("arches") or max(2, int(length / 0.06))
    pier_w = 0.012 if stone else 0.006
    for k in range(1, piers):
        t = k / piers - 0.5
        px, py = mid[0] + c * t * length, mid[1] + s * t * length
        layer.add(
            mat,
            *g.box(px, py, -0.02, pier_w, half * 2.1, deck + 0.01, angle),
            anchor=mid,
        )
        if stone and "arches" in bridge:
            # Starlings: pointed cutwaters around the pier, upstream and downstream.
            ext = half * 1.6
            local = [
                (-pier_w, -ext * 0.8),
                (0.0, -ext),
                (pier_w, -ext * 0.8),
                (pier_w, ext * 0.8),
                (0.0, ext),
                (-pier_w, ext * 0.8),
            ]
            poly = [(px + lx * c - ly * s, py + lx * s + ly * c) for lx, ly in local]
            layer.add(mat, *g.prism(poly, -0.02, deck * 0.35), anchor=mid)
            # Haunches: the pier widens up to the deck, leaving round-looking arches between.
            spread = length / piers * 0.42
            verts = []
            for lx, z in (
                (-pier_w / 2, deck * 0.3),
                (pier_w / 2, deck * 0.3),
                (spread, deck - 0.01),
                (-spread, deck - 0.01),
            ):
                for ly in (-half, half):
                    verts.append((px + lx * c - ly * s, py + lx * s + ly * c, z))
            faces = [(0, 2, 4, 6), (1, 7, 5, 3), (2, 3, 5, 4), (0, 6, 7, 1)]
            layer.add(mat, verts, faces, anchor=mid)
    # Features on the deck: chapel, gatehouses, drawbridge (fractions from ``from``).
    busy = []
    k_house = plan.a * plan.house_scale * plan.height_scale

    def along(t):
        return (mid[0] + c * (t - 0.5) * length, mid[1] + s * (t - 0.5) * length)

    if "chapel_at" in bridge:
        t = bridge["chapel_at"]
        cx, cy = along(t)
        # Chapel on an enlarged pier, on the downstream side, choir across the river.
        ox, oy = -s * half * 1.3, c * half * 1.3
        layer.add(
            mat,
            *g.box(cx + ox, cy + oy, -0.02, 0.05, half * 2.4, deck + 0.01, angle),
            anchor=mid,
        )
        transform = g.Transform(
            (cx + ox, cy + oy),
            angle + math.pi / 2,
            plan.a * plan.house_scale * 0.8,
            k_house * 0.8,
            z0=deck,
        )
        for part_mat, shape in monuments.church(0.9, tower_spire=False):
            verts, faces = transform.apply(shape)
            layer.add(part_mat, verts, faces, anchor=mid)
        busy.append((t, 0.035 / max(length, 1e-6)))
    for t in bridge.get("gatehouses_at", []):
        gx, gy = along(t)
        gw = half * 2.6
        layer.add(
            "WallStone",
            *g.box(gx, gy, deck - 0.01, gw * 1.1, gw, k_house * 13.0, angle),
            anchor=mid,
        )
        for side in (-1, 1):
            tx, ty = gx - s * side * gw / 2, gy + c * side * gw / 2
            layer.add(
                "WallStone",
                *g.cylinder(tx, ty, -0.02, gw * 0.26, deck + k_house * 16.0, 8),
                anchor=mid,
            )
            layer.add(
                "Slate",
                *g.cone(tx, ty, deck + k_house * 16.0, gw * 0.28, gw * 0.45, 8),
                anchor=mid,
            )
        busy.append((t, gw * 0.7 / max(length, 1e-6)))
    if "drawbridge_at" in bridge:
        t = bridge["drawbridge_at"]
        dx, dy = along(t)
        span = 0.03
        # Raised leaf (tilted about its landward edge).
        hinge = (dx - c * span / 2, dy - s * span / 2, deck)
        tip = (dx - c * span * 0.1, dy - s * span * 0.1, deck + span * 0.8)
        layer.add("Wood", *g.beam(hinge, tip, half * 1.4, 0.004), anchor=mid)
        busy.append((t, span * 1.2 / max(length, 1e-6)))
    if bridge.get("inhabited", False):
        step = 0.034
        count = int(length / step)
        for k in range(count):
            t = (k + 0.5) / count - 0.5
            if any(abs(t + 0.5 - bt) < bw for bt, bw in busy):
                continue
            for side in (-1, 1):
                hx = mid[0] + c * t * length - s * side * half * 0.62
                hy = mid[1] + s * t * length + c * side * half * 0.62
                depth = half * 0.75
                height = house_height(plan, rng) * 0.75
                house(
                    layer,
                    hx,
                    hy,
                    deck,
                    step * 0.92,
                    depth,
                    height,
                    angle,
                    rng,
                    anchor=mid,
                )


def build_monument(plan, site, monument, layers, variant=""):
    """Place a dedicated mesh; returns nothing (adds to the right layer)."""
    x, y = plan.warp(*monument["at"])
    k = plan.a * monument.get("scale", plan.monument_scale)
    transform = g.Transform(
        (x, y), math.radians(monument.get("angle_deg", 0.0)), k, k * plan.height_scale
    )
    builder = monuments.BUILDERS[monument["model"]]
    parts = builder(monument.get("size", 1.0), variant, monument.get("params", {}))
    layer_name = (
        monument["id"]
        if not active(monument) or monument.get("variant_from_year")
        else "landmarks"
    )
    if variant:
        layer_name = f"{monument['id']}__{variant}"
    layer = layers.setdefault(layer_name, g.Layer(layer_name))
    for mat, shape in parts:
        verts, faces = transform.apply(shape)
        layer.add(mat, verts, faces, anchor=(x, y))
    radius = monument.get("clear_radius_m", 0.0) * k
    if radius > 0 and not variant:
        site.clearings.append((x, y, radius))


# --- Houses --------------------------------------------------------------------------


def house_height(plan, rng):
    """Eaves height of a town house (2 to 4 storeys) in world units."""
    return rng.uniform(8.0, 14.0) * plan.a * plan.house_scale * plan.height_scale


ROOFS = (("Tile", 0.55), ("TileOld", 0.25), ("Slate", 0.2))
WALLS = (("Plaster", 0.55), ("Timber", 0.3), ("Stone", 0.15))


def pick(rng, table):
    """Weighted choice."""
    roll = rng.random()
    for name, weight in table:
        roll -= weight
        if roll <= 0:
            return name
    return table[-1][0]


def house(layer, x, y, z, width, depth, height, angle, rng, anchor=None, roof=None):
    """Town house, gable towards the street (ridge along the depth)."""
    tint = rng.uniform(0.82, 1.12)
    color = (tint, tint * rng.uniform(0.97, 1.03), tint * rng.uniform(0.94, 1.02))
    anchor = anchor or (x, y)
    wall = pick(rng, WALLS)
    roof = roof or pick(rng, ROOFS)
    layer.add(
        wall,
        *g.box(
            x,
            y,
            z - g.FOUNDATION,
            width,
            depth,
            height + g.FOUNDATION,
            angle,
            top=False,
        ),
        anchor=anchor,
        color=color,
    )
    pitch = width * rng.uniform(0.75, 1.0)
    # Gable to the street: ridge along the depth (local y).
    layer.add(
        roof,
        *g.gable_roof(
            x,
            y,
            z + height,
            depth,
            width,
            pitch,
            angle + math.pi / 2,
            overhang=width * 0.06,
        ),
        anchor=anchor,
        color=color,
    )


def voronoi_cells(radius, rng):
    """Voronoi cells of jittered seeds covering a disc (half-plane clipping of neighbours)."""
    seeds = []
    n = int(radius / CELL) + 1
    for i in range(-n, n + 1):
        for j in range(-n, n + 1):
            x = (i + rng.uniform(-0.38, 0.38)) * CELL
            y = (j + rng.uniform(-0.38, 0.38)) * CELL
            if x * x + y * y <= radius * radius:
                seeds.append((x, y))
    buckets = {}
    for index, (x, y) in enumerate(seeds):
        buckets.setdefault(
            (int(math.floor(x / CELL)), int(math.floor(y / CELL))), []
        ).append(index)
    cells = []
    for index, (x, y) in enumerate(seeds):
        half = CELL * 1.6
        poly = [
            (x - half, y - half),
            (x + half, y - half),
            (x + half, y + half),
            (x - half, y + half),
        ]
        bx, by = int(math.floor(x / CELL)), int(math.floor(y / CELL))
        for dx in range(-2, 3):
            for dy in range(-2, 3):
                for other in buckets.get((bx + dx, by + dy), []):
                    if other == index:
                        continue
                    ox, oy = seeds[other]
                    nx, ny = ox - x, oy - y
                    c = (ox * ox + oy * oy - x * x - y * y) / 2.0
                    poly = g.clip_polygon_halfplane(poly, nx, ny, c)
                    if len(poly) < 3:
                        break
        if len(poly) >= 3:
            cells.append(((x, y), poly))
    return cells


def build_fabric(plan, site, layers, rng):
    """Blocks of houses lining every cell edge (lanes between cells), plus block masses for the LOD."""
    houses, blocks = layers["houses"], layers["blocks"]
    count = 0
    for (sx, sy), cell in voronoi_cells(plan.zone_px, rng):
        density = site.density(sx, sy)
        if density <= 0.0:
            continue
        cell = g.ccw(cell)
        inset = g.inset_polygon(cell, LANE)
        if len(inset) < 3 or abs(g.signed_area(inset)) < 0.004:
            continue
        land = all(site.on_land(x, y) for x, y in cell)
        if land and site.free(sx, sy, 0.0):
            layers["ground"].add("Dirt", *g.flat(cell, Z_GROUND))
        placed = 0
        for i in range(len(inset)):
            (ax, ay), (bx, by) = inset[i], inset[(i + 1) % len(inset)]
            edge = math.hypot(bx - ax, by - ay)
            if edge < 0.03:
                continue
            ex, ey = (bx - ax) / edge, (by - ay) / edge
            nx, ny = -ey, ex  # inward normal of a CCW polygon
            angle = math.atan2(ey, ex)
            t = rng.uniform(0.0, 0.01)
            while t < edge - 0.02:
                width = rng.uniform(5.5, 9.0) * plan.a * plan.house_scale
                depth = rng.uniform(10.0, 16.0) * plan.a * plan.house_scale
                if t + width > edge:
                    break
                cx = ax + ex * (t + width / 2) + nx * depth / 2
                cy = ay + ey * (t + width / 2) + ny * depth / 2
                t += width
                if rng.random() > density:
                    t += rng.uniform(0.0, 0.02)
                    continue
                back = (cx + nx * depth * 0.45, cy + ny * depth * 0.45)
                front = (cx - nx * depth * 0.45, cy - ny * depth * 0.45)
                if not g.point_in_polygon(*back, inset):
                    continue
                if not (
                    site.on_land(*front)
                    and site.on_land(*back)
                    and site.on_land(cx, cy)
                ):
                    continue
                if not site.free(cx, cy) or not site.free(*front):
                    continue
                roof = "Thatch" if density < 0.4 and rng.random() < 0.5 else None
                house(
                    houses,
                    cx,
                    cy,
                    0.0,
                    width * 0.97,
                    depth,
                    house_height(plan, rng),
                    angle,
                    rng,
                    roof=roof,
                )
                placed += 1
        count += placed
        if placed >= 4 and land:
            mass = g.inset_polygon(inset, 0.01)
            if len(mass) >= 3:
                height = (
                    11.0
                    * plan.a
                    * plan.house_scale
                    * plan.height_scale
                    * min(1.0, 0.5 + density)
                )
                blocks.add(
                    "Plaster",
                    *g.prism(mass, -g.FOUNDATION, height, top=False),
                    color=(0.9, 0.85, 0.8),
                )
                blocks.add("TileOld", *g.flat(mass, height), color=(1.0, 1.0, 1.0))
    return count


# --- Blender ---------------------------------------------------------------------------


def build(landmark, meters_per_px, seed=1337, siege=False):
    """Build every layer of a landmark (or of its siege backdrop); returns {name: Layer}."""
    if siege:
        plan = SiegePlan(landmark, meters_per_px)
        landmark = siege_subset(landmark)
        plan.data = landmark
    else:
        plan = Plan(landmark, meters_per_px)
    rng = random.Random(seed)
    layers = {
        name: g.Layer(name) for name in ("ground", "houses", "blocks", "landmarks")
    }
    waters, islands, opens = build_ground(plan, layers, rng)
    site = Site(plan, waters, islands, opens)
    build_streets(plan, site, layers)
    for wall in landmark.get("walls", []):
        name = "landmarks" if active(wall) else wall["id"]
        build_wall(plan, site, wall, layers.setdefault(name, g.Layer(name)))
    for bridge in landmark.get("bridges", []):
        name = "landmarks" if active(bridge) else bridge["id"]
        build_bridge(plan, site, bridge, layers.setdefault(name, g.Layer(name)), rng)
    for monument in landmark.get("monuments", []):
        build_monument(plan, site, monument, layers)
        for variant in monument.get("variant_from_year", {}):
            build_monument(plan, site, monument, layers, variant)
    houses = build_fabric(plan, site, layers, rng)
    print(f"HOUSES {houses}")
    if siege:
        # Metres, and every element shown whatever the year.
        scale = plan.export_scale
        merged = {name: g.Layer(name) for name in ("ground", "houses", "landmarks")}
        for name, layer in layers.items():
            if name == "blocks":
                continue
            target = merged.get(name, merged["landmarks"])
            for mat, (v, f, a, c) in layer.parts.items():
                if name.endswith("__charles_v"):
                    continue
                target.add(
                    mat,
                    [(x * scale, y * scale, z * scale) for x, y, z in v],
                    f,
                    None,
                    (1.0, 1.0, 1.0),
                )
                part = target.parts[mat]
                part[2][len(part[2]) - len(v) :] = [
                    (x * scale, y * scale) for x, y in a
                ]
                part[3][len(part[3]) - len(v) :] = c
        layers = merged
    return layers


def material(bpy, name):
    """Principled material of the palette (created once)."""
    existing = bpy.data.materials.get(name)
    if existing is not None:
        return existing
    color, roughness = PALETTE[name]
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = next(node for node in mat.node_tree.nodes if node.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    mat.diffuse_color = (*color, 1.0)
    return mat


def layer_object(bpy, layer):
    """One mesh object per layer: material slots, anchor UV map, tint colour attribute."""
    verts, faces, anchors, colors, mat_index = [], [], [], [], []
    materials = sorted(layer.parts)
    for slot, mat in enumerate(materials):
        v, f, a, c = layer.parts[mat]
        base = len(verts)
        verts.extend(v)
        faces.extend(tuple(i + base for i in face) for face in f)
        anchors.extend(a)
        colors.extend(c)
        mat_index.extend([slot] * len(f))
    mesh = bpy.data.meshes.new(layer.name)
    mesh.from_pydata(verts, [], faces)
    for mat in materials:
        mesh.materials.append(material(bpy, mat))
    mesh.polygons.foreach_set("material_index", mat_index)
    uv = mesh.uv_layers.new(name="anchor")
    loop_vertices = [0] * len(mesh.loops)
    mesh.loops.foreach_get("vertex_index", loop_vertices)
    uv_data = []
    for vi in loop_vertices:
        ax, ay = anchors[vi]
        # glTF flips V (v' = 1 - v): store 1 + y so that Godot reads UV.y = -y = world z.
        uv_data += [ax, 1.0 + ay]
    uv.data.foreach_set("uv", uv_data)
    tint = mesh.color_attributes.new(name="tint", type="BYTE_COLOR", domain="CORNER")
    tint_data = []
    for vi in loop_vertices:
        r, gg, b = colors[vi]
        tint_data += [min(r / 1.2, 1.0), min(gg / 1.2, 1.0), min(b / 1.2, 1.0), 1.0]
    tint.data.foreach_set("color", tint_data)
    mesh.color_attributes.active_color = tint
    mesh.update()
    obj = bpy.data.objects.new(layer.name, mesh)
    bpy.context.collection.objects.link(obj)
    return obj


def export(bpy, layers, out_path):
    """Create the objects and export the glTF binary."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    for layer in layers.values():
        if not layer.parts:
            continue
        obj = layer_object(bpy, layer)
        obj.select_set(True)
        print(f"LAYER {layer.name} {layer.triangles()}")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(out_path),
        export_format="GLB",
        export_yup=True,
        export_apply=False,
        export_vertex_color="ACTIVE",
        use_selection=False,
    )


def compact_glb(path):
    """Store the tint colours as normalised bytes (core glTF) instead of shorts: -9 % of the file.

    Rebuilds the binary chunk (each buffer view repacked, 4-byte aligned). Returns the new size.
    """
    import struct

    blob = path.read_bytes()
    json_len = struct.unpack("<I", blob[12:16])[0]
    doc = json.loads(blob[20 : 20 + json_len])
    bin_start = 20 + json_len + 8
    binary = blob[bin_start:]
    views = doc["bufferViews"]
    data = [
        bytearray(
            binary[v.get("byteOffset", 0) : v.get("byteOffset", 0) + v["byteLength"]]
        )
        for v in views
    ]
    colors = {
        prim["attributes"]["COLOR_0"]
        for mesh in doc["meshes"]
        for prim in mesh["primitives"]
        if "COLOR_0" in prim["attributes"]
    }
    for index in colors:
        accessor = doc["accessors"][index]
        if accessor["componentType"] != 5123 or accessor.get("byteOffset", 0):
            continue
        view = accessor["bufferView"]
        count = accessor["count"] * (4 if accessor["type"] == "VEC4" else 3)
        shorts = struct.unpack(f"<{count}H", data[view][: count * 2])
        data[view] = bytearray(bytes((v * 255 + 32767) // 65535 for v in shorts))
        views[view].pop("byteStride", None)
        accessor["componentType"] = 5121
    out = bytearray()
    for view, chunk in zip(views, data, strict=True):
        out += b"\0" * (-len(out) % 4)
        view["byteOffset"] = len(out)
        view["byteLength"] = len(chunk)
        out += chunk
    out += b"\0" * (-len(out) % 4)
    doc["buffers"][0]["byteLength"] = len(out)
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    total = 12 + 8 + len(text) + 8 + len(out)
    header = struct.pack("<4sII", b"glTF", 2, total)
    path.write_bytes(
        header
        + struct.pack("<I4s", len(text), b"JSON")
        + text
        + struct.pack("<I4s", len(out), b"BIN\0")
        + bytes(out)
    )
    return total


def main() -> None:
    """Parse ``-- <landmark.json> <out.glb>`` and build the model."""
    import bpy

    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    siege = "--siege" in args
    args = [arg for arg in args if arg != "--siege"]
    if len(args) < 2:
        raise SystemExit(
            "usage: landmark_city.py -- <landmark.json> <out.glb> [--siege]"
        )
    landmark_path = Path(args[0])
    landmark = json.loads(landmark_path.read_text(encoding="utf-8"))
    map_json = landmark_path.resolve().parents[1] / "map" / "map.json"
    meters_per_px = json.loads(map_json.read_text(encoding="utf-8"))["meters_per_px"]
    layers = build(landmark, meters_per_px, siege=siege)
    export(bpy, layers, Path(args[1]))
    print(f"SIZE {compact_glb(Path(args[1])) / 1e6:.2f} MB")
    print("OK")


if __name__ == "__main__":
    main()
