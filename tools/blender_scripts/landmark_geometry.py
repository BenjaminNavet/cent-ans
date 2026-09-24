"""Geometry helpers of the landmark city generator (lot L1), without Blender dependencies.

Shapes are built as raw ``(vertices, faces)`` lists (Z up, faces counter-clockwise seen from
outside) and accumulated per material in a ``Layer``; ``landmark_city.py`` turns every layer into
one Blender object (one node of the exported glTF). Each vertex carries a drape anchor (the
world XY point whose terrain height lifts it in Godot) and a tint colour.
"""

import math

FOUNDATION = 0.06  # depth of walls below their base, in world units


class Layer:
    """Triangles accumulated per material for one exported node."""

    def __init__(self, name):
        self.name = name
        self.parts = {}  # material -> [verts, faces, anchors, colors]

    def add(self, mat, verts, faces, anchor=None, color=(1.0, 1.0, 1.0)):
        """Append a shape; ``anchor`` None = each vertex drapes on its own XY."""
        part = self.parts.setdefault(mat, [[], [], [], []])
        base = len(part[0])
        part[0].extend(verts)
        part[1].extend(tuple(index + base for index in face) for face in faces)
        if anchor is None:
            part[2].extend((v[0], v[1]) for v in verts)
        else:
            part[2].extend([anchor] * len(verts))
        part[3].extend([color] * len(verts))

    def triangles(self):
        """Triangle count (n-gons fanned)."""
        return sum(len(face) - 2 for part in self.parts.values() for face in part[1])


class Transform:
    """Local shape frame (metres or units) to world units: scale, rotation about Z, offset."""

    def __init__(self, origin=(0.0, 0.0), angle=0.0, k=1.0, kz=None, z0=0.0):
        self.ox, self.oy = origin
        self.cos, self.sin = math.cos(angle), math.sin(angle)
        self.k = k
        self.kz = k if kz is None else kz
        self.z0 = z0

    def point(self, x, y, z=0.0):
        """Transform one local point."""
        x, y = x * self.k, y * self.k
        return (
            self.ox + x * self.cos - y * self.sin,
            self.oy + x * self.sin + y * self.cos,
            self.z0 + z * self.kz,
        )

    def apply(self, shape):
        """Transform a ``(verts, faces)`` shape."""
        verts, faces = shape
        return [self.point(*v) for v in verts], faces


# --- 2D helpers ----------------------------------------------------------------------


def signed_area(poly):
    """Shoelace area (positive when counter-clockwise)."""
    area = 0.0
    for i, (x0, y0) in enumerate(poly):
        x1, y1 = poly[(i + 1) % len(poly)]
        area += x0 * y1 - x1 * y0
    return area / 2.0


def ccw(poly):
    """Counter-clockwise copy of a polygon."""
    return list(poly) if signed_area(poly) >= 0 else list(reversed(poly))


def point_in_polygon(x, y, poly):
    """Even-odd rule."""
    inside = False
    n = len(poly)
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi + 1e-12) + xi:
            inside = not inside
        j = i
    return inside


def distance_to_segment(px, py, ax, ay, bx, by):
    """Distance from P to segment AB."""
    dx, dy = bx - ax, by - ay
    length2 = dx * dx + dy * dy
    t = 0.0 if length2 == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length2))
    return math.hypot(px - ax - t * dx, py - ay - t * dy)


def distance_to_polyline(px, py, points):
    """Distance from P to an open polyline."""
    return min(
        distance_to_segment(px, py, *points[i], *points[i + 1]) for i in range(len(points) - 1)
    )


def densify(points, step, closed=False):
    """Insert points so that no segment is longer than ``step``."""
    pts = list(points) + ([points[0]] if closed else [])
    out = []
    for i in range(len(pts) - 1):
        (x0, y0), (x1, y1) = pts[i], pts[i + 1]
        n = max(1, int(math.ceil(math.hypot(x1 - x0, y1 - y0) / step)))
        for k in range(n):
            t = k / n
            out.append((x0 + (x1 - x0) * t, y0 + (y1 - y0) * t))
    if not closed:
        out.append(pts[-1])
    return out


def clip_polygon_halfplane(poly, nx, ny, c):
    """Keep the part of ``poly`` where ``nx*x + ny*y <= c`` (Sutherland-Hodgman)."""
    out = []
    n = len(poly)
    for i in range(n):
        ax, ay = poly[i]
        bx, by = poly[(i + 1) % n]
        da = nx * ax + ny * ay - c
        db = nx * bx + ny * by - c
        if da <= 0:
            out.append((ax, ay))
        if (da <= 0) != (db <= 0):
            t = da / (da - db)
            out.append((ax + (bx - ax) * t, ay + (by - ay) * t))
    return out


def inset_polygon(poly, distance):
    """Shrink a convex counter-clockwise polygon by ``distance`` (half-plane clipping)."""
    result = list(poly)
    n = len(poly)
    for i in range(n):
        ax, ay = poly[i]
        bx, by = poly[(i + 1) % n]
        ex, ey = bx - ax, by - ay
        length = math.hypot(ex, ey)
        if length < 1e-9:
            continue
        # Outward normal of a CCW edge is (ey, -ex).
        nx, ny = ey / length, -ex / length
        result = clip_polygon_halfplane(result, nx, ny, nx * ax + ny * ay - distance)
        if len(result) < 3:
            return []
    return result


def polyline_normals(points):
    """Unit left normals at each vertex of a polyline (averaged at joints)."""
    normals = []
    n = len(points)
    for i in range(n):
        x0, y0 = points[max(i - 1, 0)]
        x1, y1 = points[min(i + 1, n - 1)]
        dx, dy = x1 - x0, y1 - y0
        length = math.hypot(dx, dy) or 1.0
        normals.append((-dy / length, dx / length))
    return normals


# --- 3D shapes (local frame) -----------------------------------------------------------


def box(cx, cy, z0, sx, sy, h, angle=0.0, bottom=False):
    """Box of footprint ``sx`` x ``sy`` centred on (cx, cy), from z0 to z0 + h."""
    c, s = math.cos(angle), math.sin(angle)
    corners = []
    for lx, ly in ((-sx / 2, -sy / 2), (sx / 2, -sy / 2), (sx / 2, sy / 2), (-sx / 2, sy / 2)):
        corners.append((cx + lx * c - ly * s, cy + lx * s + ly * c))
    verts = [(x, y, z0) for x, y in corners] + [(x, y, z0 + h) for x, y in corners]
    faces = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7)]
    if bottom:
        faces.append((3, 2, 1, 0))
    return verts, faces


def prism(poly, z0, z1, top=True):
    """Vertical extrusion of a polygon (any orientation); top face triangulated as a fan."""
    poly = ccw(poly)
    n = len(poly)
    verts = [(x, y, z0) for x, y in poly] + [(x, y, z1) for x, y in poly]
    faces = [(i, (i + 1) % n, n + (i + 1) % n, n + i) for i in range(n)]
    if top:
        faces += [tuple(n + i for i in tri) for tri in triangulate(poly)]
    return verts, faces


def flat(poly, z):
    """Horizontal polygon facing up."""
    poly = ccw(poly)
    return [(x, y, z) for x, y in poly], triangulate(poly)


def triangulate(poly):
    """Ear clipping of a simple counter-clockwise polygon; returns index triangles."""
    indices = list(range(len(poly)))
    triangles = []
    guard = 0
    while len(indices) > 3 and guard < 10000:
        guard += 1
        n = len(indices)
        ear_found = False
        for k in range(n):
            i0, i1, i2 = indices[(k - 1) % n], indices[k], indices[(k + 1) % n]
            (ax, ay), (bx, by), (cx, cy) = poly[i0], poly[i1], poly[i2]
            if (bx - ax) * (cy - ay) - (by - ay) * (cx - ax) <= 1e-12:
                continue
            if any(
                _in_triangle(poly[j], (ax, ay), (bx, by), (cx, cy))
                for j in indices
                if j not in (i0, i1, i2)
            ):
                continue
            triangles.append((i0, i1, i2))
            indices.pop(k)
            ear_found = True
            break
        if not ear_found:
            break
    if len(indices) >= 3:
        for k in range(1, len(indices) - 1):
            triangles.append((indices[0], indices[k], indices[k + 1]))
    return triangles


def _in_triangle(p, a, b, c):
    def sign(p1, p2, p3):
        return (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])

    d1, d2, d3 = sign(p, a, b), sign(p, b, c), sign(p, c, a)
    return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))


def gable_roof(cx, cy, z0, length, width, h, angle=0.0, overhang=0.0):
    """Triangular prism, ridge along local x."""
    c, s = math.cos(angle), math.sin(angle)
    hl, hw = length / 2 + overhang, width / 2 + overhang

    def p(lx, ly, z):
        return (cx + lx * c - ly * s, cy + lx * s + ly * c, z)

    verts = [
        p(-hl, -hw, z0),
        p(-hl, hw, z0),
        p(-hl, 0, z0 + h),
        p(hl, -hw, z0),
        p(hl, hw, z0),
        p(hl, 0, z0 + h),
    ]
    faces = [(0, 2, 1), (3, 4, 5), (0, 3, 5, 2), (1, 2, 5, 4)]
    return verts, faces


def hip_roof(cx, cy, z0, length, width, h, angle=0.0):
    """Hipped roof over a rectangle, short ridge along local x."""
    c, s = math.cos(angle), math.sin(angle)
    hl, hw = length / 2, width / 2
    ridge = max(hl - hw, 0.0)

    def p(lx, ly, z):
        return (cx + lx * c - ly * s, cy + lx * s + ly * c, z)

    verts = [
        p(-hl, -hw, z0),
        p(hl, -hw, z0),
        p(hl, hw, z0),
        p(-hl, hw, z0),
        p(-ridge, 0, z0 + h),
        p(ridge, 0, z0 + h),
    ]
    faces = [(0, 1, 5, 4), (2, 3, 4, 5), (1, 2, 5), (3, 0, 4)]
    return verts, faces


def cylinder(cx, cy, z0, r, h, sides=8, top=True):
    """Vertical cylinder (prism of a regular polygon)."""
    poly = [
        (cx + r * math.cos(2 * math.pi * i / sides), cy + r * math.sin(2 * math.pi * i / sides))
        for i in range(sides)
    ]
    return prism(poly, z0, z0 + h, top)


def cone(cx, cy, z0, r, h, sides=8, r_top=0.0):
    """Cone (or frustum) standing on z0."""
    verts = [
        (cx + r * math.cos(2 * math.pi * i / sides), cy + r * math.sin(2 * math.pi * i / sides), z0)
        for i in range(sides)
    ]
    if r_top <= 0:
        verts.append((cx, cy, z0 + h))
        faces = [(i, (i + 1) % sides, sides) for i in range(sides)]
    else:
        verts += [
            (
                cx + r_top * math.cos(2 * math.pi * i / sides),
                cy + r_top * math.sin(2 * math.pi * i / sides),
                z0 + h,
            )
            for i in range(sides)
        ]
        faces = [(i, (i + 1) % sides, sides + (i + 1) % sides, sides + i) for i in range(sides)]
        faces.append(tuple(range(sides, 2 * sides)))
    return verts, faces


def pyramid(cx, cy, z0, sx, sy, h, angle=0.0):
    """Four-sided spire over a rectangle."""
    c, s = math.cos(angle), math.sin(angle)
    verts = []
    for lx, ly in ((-sx / 2, -sy / 2), (sx / 2, -sy / 2), (sx / 2, sy / 2), (-sx / 2, sy / 2)):
        verts.append((cx + lx * c - ly * s, cy + lx * s + ly * c, z0))
    verts.append((cx, cy, z0 + h))
    return verts, [(0, 1, 4), (1, 2, 4), (2, 3, 4), (3, 0, 4)]


def beam(a, b, width, thickness):
    """Straight bar between two 3D points (flying buttress, sail): rectangular section."""
    ax, ay, az = a
    bx, by, bz = b
    dx, dy, dz = bx - ax, by - ay, bz - az
    length = math.sqrt(dx * dx + dy * dy + dz * dz) or 1.0
    dx, dy, dz = dx / length, dy / length, dz / length
    # Side vector: horizontal, perpendicular to the direction.
    sx, sy = -dy, dx
    norm = math.hypot(sx, sy)
    if norm < 1e-6:
        sx, sy = 1.0, 0.0
    else:
        sx, sy = sx / norm, sy / norm
    # Up vector = direction x side.
    ux, uy, uz = -dz * sy, dz * sx, dx * sy - dy * sx
    hw, ht = width / 2, thickness / 2
    verts = []
    for px, py, pz in (a, b):
        for ws, ts in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            verts.append(
                (
                    px + sx * hw * ws + ux * ht * ts,
                    py + sy * hw * ws + uy * ht * ts,
                    pz + uz * ht * ts,
                )
            )
    faces = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (0, 3, 2, 1), (4, 5, 6, 7)]
    return verts, faces


def disc_vertical(cx, cy, z, r, normal_angle, sides=12, offset=0.0):
    """Vertical disc (rose window) facing the horizontal direction ``normal_angle``."""
    nx, ny = math.cos(normal_angle), math.sin(normal_angle)
    tx, ty = -ny, nx
    verts = [(cx + nx * offset, cy + ny * offset, z)]
    for i in range(sides):
        a = 2 * math.pi * i / sides
        verts.append(
            (
                cx + nx * offset + tx * r * math.cos(a),
                cy + ny * offset + ty * r * math.cos(a),
                z + r * math.sin(a),
            )
        )
    faces = [(0, 1 + i, 1 + (i + 1) % sides) for i in range(sides)]
    return verts, faces


def panel(cx, cy, z0, width, h, normal_angle, offset=0.0, pointed=True):
    """Vertical window/door panel (lancet when ``pointed``) facing ``normal_angle``."""
    nx, ny = math.cos(normal_angle), math.sin(normal_angle)
    tx, ty = -ny, nx
    bx, by = cx + nx * offset, cy + ny * offset
    hw = width / 2
    verts = [
        (bx - tx * hw, by - ty * hw, z0),
        (bx + tx * hw, by + ty * hw, z0),
        (bx + tx * hw, by + ty * hw, z0 + h),
        (bx - tx * hw, by - ty * hw, z0 + h),
    ]
    faces = [(0, 1, 2, 3)]
    if pointed:
        verts.append((bx, by, z0 + h + width * 0.8))
        faces.append((3, 2, 4))
    return verts, faces
