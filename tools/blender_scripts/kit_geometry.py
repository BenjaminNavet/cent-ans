"""Geometry core of the realistic building kit (lot BR1, ADR 0021).

Pure Python (no ``bpy``): a :class:`Geometry` accumulates polygons per material with
per-corner UVs in **real metres** (box projection along each face's own tangent frame, so roof
textures run up the slope and wall textures stay upright) and per-corner vertex colours
(building tint x ambient occlusion near the ground and under the eaves). ``kit_export.py`` turns
it into a Blender mesh. Blender convention: Z up, metres; a building's front faces -Y.
"""

import math

Vec = tuple[float, float, float]
UP: Vec = (0.0, 0.0, 1.0)


def add(a: Vec, b: Vec) -> Vec:
    """Vector sum."""
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def sub(a: Vec, b: Vec) -> Vec:
    """Vector difference."""
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def mul(a: Vec, s: float) -> Vec:
    """Scale a vector."""
    return (a[0] * s, a[1] * s, a[2] * s)


def dot(a: Vec, b: Vec) -> float:
    """Dot product."""
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a: Vec, b: Vec) -> Vec:
    """Cross product."""
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def length(a: Vec) -> float:
    """Euclidean norm."""
    return math.sqrt(dot(a, a))


def norm(a: Vec) -> Vec:
    """Unit vector (zero stays zero)."""
    size = length(a)
    return a if size < 1e-12 else mul(a, 1.0 / size)


def lerp(a: Vec, b: Vec, t: float) -> Vec:
    """Linear interpolation."""
    return (
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    )


def smoothstep(e0: float, e1: float, x: float) -> float:
    """Hermite step between ``e0`` and ``e1``."""
    t = min(max((x - e0) / (e1 - e0), 0.0), 1.0)
    return t * t * (3.0 - 2.0 * t)


def _hash(ix: int, iy: int, iz: int) -> float:
    """Deterministic lattice value in [0, 1)."""
    h = (ix * 374761393 + iy * 668265263 + iz * 2147483647) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((h ^ (h >> 16)) & 0xFFFF) / 65536.0


def noise3(p: Vec, scale: float) -> float:
    """Smooth value noise in [0, 1] with features of about ``scale`` metres."""
    x, y, z = p[0] / scale, p[1] / scale, p[2] / scale
    ix, iy, iz = math.floor(x), math.floor(y), math.floor(z)
    fx, fy, fz = x - ix, y - iy, z - iz
    fx, fy, fz = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy), fz * fz * (3 - 2 * fz)
    total = 0.0
    for dx in (0, 1):
        for dy in (0, 1):
            for dz in (0, 1):
                w = (
                    (fx if dx else 1 - fx)
                    * (fy if dy else 1 - fy)
                    * (fz if dz else 1 - fz)
                )
                total += w * _hash(ix + dx, iy + dy, iz + dz)
    return total


# Weathering amplitude of the baked colour noise per material (roofs: moss and wear).
WEATHER = {
    "RoofTile": 0.3,
    "RoofFlat": 0.32,
    "RoofSlate": 0.22,
    "Thatch": 0.3,
    "Plaster": 0.12,
    "Rubble": 0.12,
    "Ashlar": 0.14,
    "Planks": 0.16,
    "Timber": 0.1,
}


def newell_normal(points: list[Vec]) -> Vec:
    """Normal of a (possibly non-planar) polygon, counter-clockwise seen from the front."""
    nx = ny = nz = 0.0
    for i, p in enumerate(points):
        q = points[(i + 1) % len(points)]
        nx += (p[1] - q[1]) * (p[2] + q[2])
        ny += (p[2] - q[2]) * (p[0] + q[0])
        nz += (p[0] - q[0]) * (p[1] + q[1])
    return norm((nx, ny, nz))


def tangent_frame(normal: Vec) -> tuple[Vec, Vec]:
    """Horizontal tangent and up-slope bitangent of a face (UV axes)."""
    tangent = cross(UP, normal)
    if length(tangent) < 1e-6:
        tangent = (1.0, 0.0, 0.0) if normal[2] > 0 else (-1.0, 0.0, 0.0)
    tangent = norm(tangent)
    return tangent, cross(normal, tangent)


class Geometry:
    """Polygon soup grouped by material, with metric UVs and vertex colours.

    ``transform`` places the local frame of the building part being built (yaw + offset); the
    UVs are taken *after* the transform, so neighbouring parts share a continuous texture.
    """

    def __init__(self) -> None:
        """Start empty, identity transform, neutral tint."""
        self.polys: dict[
            str, list[tuple[list[Vec], list[tuple[float, float]], list[Vec]]]
        ] = {}
        self.offset: Vec = (0.0, 0.0, 0.0)
        self.yaw = 0.0
        self.tint: Vec = (1.0, 1.0, 1.0)
        self.uv_shift = (0.0, 0.0)
        self.ground_ao = True
        self.eave_z: float | None = None
        self.char = 0.0  # 0..1 blackening (burned ruins)
        self.seed_shift = 0.0  # decorrelates the weathering noise between buildings

    # --- frame --------------------------------------------------------------------------

    def to_world(self, p: Vec) -> Vec:
        """Apply the current yaw + offset to a local point."""
        c, s = math.cos(self.yaw), math.sin(self.yaw)
        return (
            p[0] * c - p[1] * s + self.offset[0],
            p[0] * s + p[1] * c + self.offset[1],
            p[2] + self.offset[2],
        )

    def set_frame(self, offset: Vec = (0.0, 0.0, 0.0), yaw: float = 0.0) -> None:
        """Set the local frame (translation then rotation about Z)."""
        self.offset = offset
        self.yaw = yaw

    # --- polygons -----------------------------------------------------------------------

    def shade(self, p: Vec, normal: Vec, occlude: bool) -> float:
        """Baked ambient occlusion: dark at the foot of walls and under the eaves."""
        factor = 1.0
        if occlude and self.ground_ao:
            factor *= 0.58 + 0.42 * smoothstep(-0.2, 1.6, p[2])
        if occlude and self.eave_z is not None and abs(normal[2]) < 0.5:
            factor *= 1.0 - 0.28 * smoothstep(self.eave_z - 1.1, self.eave_z, p[2])
        if normal[2] < -0.5:  # soffits
            factor *= 0.55
        return factor

    def poly(
        self,
        points: list[Vec],
        mat: str,
        color: Vec | None = None,
        grain: bool = False,
        occlude: bool = True,
        local: bool = True,
    ) -> None:
        """Add a polygon (counter-clockwise seen from outside).

        ``grain`` swaps U and V (wood grain along the face's slope/vertical axis). Colours are
        ``color`` (or the building tint) x baked occlusion x char.
        """
        world = [self.to_world(p) for p in points] if local else list(points)
        normal = newell_normal(world)
        tangent, bitangent = tangent_frame(normal)
        su, sv = self.uv_shift
        uvs = []
        for p in world:
            u, v = dot(p, tangent) + su, dot(p, bitangent) + sv
            uvs.append((v, u) if grain else (u, v))
        base = color if color is not None else self.tint
        char = 1.0 - 0.9 * self.char
        amp = WEATHER.get(mat, 0.0)
        cols = []
        for p in world:
            k = self.shade(p, normal, occlude) * (
                char
                if not self.char
                else 1.0 - self.char * (0.75 + 0.25 * smoothstep(0.0, 3.0, p[2]))
            )
            if amp:
                q = (p[0] + self.seed_shift, p[1], p[2])
                n = 0.65 * noise3(q, 2.6) + 0.35 * noise3(q, 0.9)
                k *= 1.0 - amp + 2.0 * amp * n
            cols.append((base[0] * k, base[1] * k, base[2] * k))
        self.polys.setdefault(mat, []).append((world, uvs, cols))

    def quad(self, a: Vec, b: Vec, c: Vec, d: Vec, mat: str, **kw) -> None:
        """Shortcut for a four-sided polygon."""
        self.poly([a, b, c, d], mat, **kw)

    def warp(self, fn) -> None:
        """Deform every point in place with ``fn(point) -> point`` (sag, lean)."""
        for polys in self.polys.values():
            for points, _, _ in polys:
                points[:] = [fn(p) for p in points]

    def triangle_count(self) -> int:
        """Triangles after fan triangulation."""
        return sum(
            len(points) - 2 for polys in self.polys.values() for points, _, _ in polys
        )

    def merge(self, other: "Geometry") -> None:
        """Append another geometry's polygons (already in world space)."""
        for mat, polys in other.polys.items():
            self.polys.setdefault(mat, []).extend(polys)

    def transformed(self, scale: Vec, offset: Vec, yaw: float) -> "Geometry":
        """Copy scaled (per axis), rotated about Z then translated; UVs and colours kept."""
        c, s = math.cos(yaw), math.sin(yaw)
        out = Geometry()
        for mat, polys in self.polys.items():
            dst = out.polys.setdefault(mat, [])
            for points, uvs, cols in polys:
                moved = []
                for p in points:
                    x, y, z = p[0] * scale[0], p[1] * scale[1], p[2] * scale[2]
                    moved.append(
                        (
                            x * c - y * s + offset[0],
                            x * s + y * c + offset[1],
                            z + offset[2],
                        )
                    )
                dst.append((moved, uvs, cols))
        return out


# --- solids ---------------------------------------------------------------------------------


def obox(
    g: Geometry,
    center: Vec,
    axes: tuple[Vec, Vec, Vec],
    half: Vec,
    mat: str,
    bottom=False,
    back=True,
    **kw,
) -> None:
    """Oriented box: ``axes`` = (x, y, z) unit vectors, ``half`` = half sizes along them.

    ``bottom`` / ``back`` keep the -z / +y faces (skipped when hidden against a wall or ground).
    """
    ax, ay, az = (mul(a, h) for a, h in zip(axes, half, strict=True))

    def corner(sx, sy, sz):
        return add(add(add(center, mul(ax, sx)), mul(ay, sy)), mul(az, sz))

    c = {
        (sx, sy, sz): corner(sx, sy, sz)
        for sx in (-1, 1)
        for sy in (-1, 1)
        for sz in (-1, 1)
    }
    faces = [
        ((-1, -1, -1), (1, -1, -1), (1, -1, 1), (-1, -1, 1)),  # -y
        ((1, -1, -1), (1, 1, -1), (1, 1, 1), (1, -1, 1)),  # +x
        ((-1, 1, -1), (-1, -1, -1), (-1, -1, 1), (-1, 1, 1)),  # -x
        ((-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)),  # +z
    ]
    if back:
        faces.append(((1, 1, -1), (-1, 1, -1), (-1, 1, 1), (1, 1, 1)))
    if bottom:
        faces.append(((-1, 1, -1), (1, 1, -1), (1, -1, -1), (-1, -1, -1)))
    for face in faces:
        g.poly([c[k] for k in face], mat, **kw)


def box(g: Geometry, center: Vec, size: Vec, mat: str, yaw: float = 0.0, **kw) -> None:
    """Axis-aligned (then yawed) box of full ``size`` centred on ``center`` (local frame)."""
    c, s = math.cos(yaw), math.sin(yaw)
    obox(
        g,
        center,
        ((c, s, 0.0), (-s, c, 0.0), UP),
        (size[0] / 2, size[1] / 2, size[2] / 2),
        mat,
        **kw,
    )


def beam(
    g: Geometry,
    p0: Vec,
    p1: Vec,
    width: float,
    depth: float,
    mat: str,
    outward: Vec,
    **kw,
) -> None:
    """Timber of section ``width`` x ``depth`` from ``p0`` to ``p1``; ``depth`` along ``outward``.

    The back face (against the wall, at ``-outward``) is skipped. Grain runs along the beam.
    """
    axis = sub(p1, p0)
    size = length(axis)
    if size < 1e-4:
        return
    ax = mul(axis, 1.0 / size)
    ay = norm(mul(outward, -1.0))  # +y = into the wall (back face skipped)
    az = norm(cross(ax, ay))
    ay = cross(az, ax)
    center = mul(add(p0, p1), 0.5)
    vertical = abs(ax[2]) > 0.7
    kw.setdefault("grain", vertical)
    obox(
        g,
        center,
        (ax, ay, az),
        (size / 2, depth / 2, width / 2),
        mat,
        back=False,
        bottom=True,
        **kw,
    )


def slab(
    g: Geometry,
    top: list[Vec],
    thickness: float,
    mat: str,
    edge_mat: str | None = None,
    cell: float = 0.0,
    **kw,
) -> None:
    """Extrude a planar top polygon downwards along its normal (roof pans, eaves).

    ``cell`` > 0 subdivides the top face (triangle or quad) into a grid of about ``cell``
    metres, so that the roof can sag and carry weathering colour.
    """
    normal = newell_normal([g.to_world(p) for p in top])
    # Local-frame normal (rotation about Z only): undo the yaw.
    c, s = math.cos(-g.yaw), math.sin(-g.yaw)
    ln = (normal[0] * c - normal[1] * s, normal[0] * s + normal[1] * c, normal[2])
    bottom = [sub(p, mul(ln, thickness)) for p in top]
    if cell > 0 and len(top) in (3, 4):
        quad = top if len(top) == 4 else [top[0], top[1], top[2], top[2]]
        nu = max(1, round(length(sub(quad[1], quad[0])) / cell))
        nv = max(1, round(length(sub(quad[3], quad[0])) / cell))

        def at(u, v):
            return lerp(lerp(quad[0], quad[1], u), lerp(quad[3], quad[2], u), v)

        for i in range(nu):
            for j in range(nv):
                a, b = at(i / nu, j / nv), at((i + 1) / nu, j / nv)
                c, d = at((i + 1) / nu, (j + 1) / nv), at(i / nu, (j + 1) / nv)
                if length(sub(c, d)) < 1e-6:
                    g.poly([a, b, c], mat, occlude=False, **kw)
                elif length(sub(a, b)) < 1e-6:
                    g.poly([a, c, d], mat, occlude=False, **kw)
                else:
                    g.poly([a, b, c, d], mat, occlude=False, **kw)
    else:
        g.poly(top, mat, occlude=False, **kw)
    g.poly(list(reversed(bottom)), mat, occlude=True, **kw)
    n = len(top)
    for i in range(n):
        j = (i + 1) % n
        g.poly(
            [bottom[i], bottom[j], top[j], top[i]], edge_mat or mat, occlude=False, **kw
        )


def prism_x(
    g: Geometry,
    x0: float,
    x1: float,
    y: float,
    z: float,
    half_w: float,
    h: float,
    mat: str,
    **kw,
) -> None:
    """Triangular ridge cap along X (apex ``h`` above ``z``, base half-width ``half_w``)."""
    a0, b0, c0 = (x0, y - half_w, z), (x0, y + half_w, z), (x0, y, z + h)
    a1, b1, c1 = (x1, y - half_w, z), (x1, y + half_w, z), (x1, y, z + h)
    g.quad(a0, a1, c1, c0, mat, occlude=False, **kw)
    g.quad(c0, c1, b1, b0, mat, occlude=False, **kw)
    g.poly([a0, c0, b0], mat, occlude=False, **kw)
    g.poly([b1, c1, a1], mat, occlude=False, **kw)


def cylinder(
    g: Geometry,
    center: Vec,
    radius: float,
    height: float,
    mat: str,
    sides: int = 10,
    top=True,
    radius_top=None,
    **kw,
) -> None:
    """Vertical cylinder / frustum standing on ``center``."""
    rt = radius if radius_top is None else radius_top
    ring0 = [
        (center[0] + radius * math.cos(a), center[1] + radius * math.sin(a), center[2])
        for a in _angles(sides)
    ]
    ring1 = [
        (center[0] + rt * math.cos(a), center[1] + rt * math.sin(a), center[2] + height)
        for a in _angles(sides)
    ]
    for i in range(sides):
        j = (i + 1) % sides
        g.quad(ring0[i], ring0[j], ring1[j], ring1[i], mat, **kw)
    if top and rt > 0:
        g.poly(ring1, mat, occlude=False, **kw)


def cone(
    g: Geometry,
    center: Vec,
    radius: float,
    height: float,
    mat: str,
    sides: int = 8,
    phase: float = 0.0,
    **kw,
) -> None:
    """Pyramid / cone (spire) standing on ``center``."""
    apex = (center[0], center[1], center[2] + height)
    ring = [
        (
            center[0] + radius * math.cos(a + phase),
            center[1] + radius * math.sin(a + phase),
            center[2],
        )
        for a in _angles(sides)
    ]
    for i in range(sides):
        g.poly([ring[i], ring[(i + 1) % sides], apex], mat, occlude=False, **kw)


def _angles(sides: int) -> list[float]:
    return [2.0 * math.pi * i / sides for i in range(sides)]


# --- walls with openings --------------------------------------------------------------------


class Opening:
    """Rectangular hole in a wall (wall-local metres: ``x`` along the wall, ``y`` up)."""

    def __init__(
        self, x0: float, y0: float, x1: float, y1: float, kind: str = "window"
    ) -> None:
        """Store the rectangle and its kind (``window``, ``door``, ``shop``, ``dark``)."""
        self.x0, self.y0, self.x1, self.y1, self.kind = x0, y0, x1, y1, kind


def wall(
    g: Geometry,
    origin: Vec,
    along: Vec,
    width: float,
    height: float,
    mat: str,
    openings: list[Opening] | tuple = (),
    depth: float = 0.22,
    reveal_mat: str | None = None,
    back_mats: dict[str, str] | None = None,
    **kw,
) -> None:
    """Wall face from ``origin`` (outer bottom-left) along the horizontal unit ``along``.

    The outward normal is ``(along.y, -along.x, 0)``; the face is cut into bands around the
    rectangular ``openings``, each opening gets four reveals ``depth`` deep and a back face
    (``Window`` glass, ``Planks`` door...).
    """
    normal = (along[1], -along[0], 0.0)
    inward = mul(normal, -1.0)

    def P(x, y, d=0.0):
        return add(add(add(origin, mul(along, x)), (0.0, 0.0, y)), mul(inward, d))

    ys = sorted({0.0, height, *[o.y0 for o in openings], *[o.y1 for o in openings]})
    ys = [y for y in ys if 0.0 <= y <= height]
    for ya, yb in zip(ys, ys[1:], strict=False):
        if yb - ya < 1e-4:
            continue
        holes = sorted(
            (o for o in openings if o.y0 <= ya + 1e-5 and o.y1 >= yb - 1e-5),
            key=lambda o: o.x0,
        )
        x = 0.0
        for o in holes:
            if o.x0 - x > 1e-4:
                g.quad(P(x, ya), P(o.x0, ya), P(o.x0, yb), P(x, yb), mat, **kw)
            x = max(x, o.x1)
        if width - x > 1e-4:
            g.quad(P(x, ya), P(width, ya), P(width, yb), P(x, yb), mat, **kw)
    backs = {"window": "Window", "door": "Door", "shop": "Window", "dark": "Window"}
    if back_mats:
        backs.update(back_mats)
    rmat = reveal_mat or mat
    back_kw = {key: value for key, value in kw.items() if key not in ("grain", "color")}
    for o in openings:
        d = depth
        g.quad(
            P(o.x1, o.y0), P(o.x1, o.y0, d), P(o.x0, o.y0, d), P(o.x0, o.y0), rmat, **kw
        )  # sill (up)
        g.quad(
            P(o.x0, o.y1), P(o.x0, o.y1, d), P(o.x1, o.y1, d), P(o.x1, o.y1), rmat, **kw
        )  # head (down)
        g.quad(
            P(o.x0, o.y0), P(o.x0, o.y0, d), P(o.x0, o.y1, d), P(o.x0, o.y1), rmat, **kw
        )
        g.quad(
            P(o.x1, o.y1), P(o.x1, o.y1, d), P(o.x1, o.y0, d), P(o.x1, o.y0), rmat, **kw
        )
        g.quad(
            P(o.x0, o.y0, d),
            P(o.x1, o.y0, d),
            P(o.x1, o.y1, d),
            P(o.x0, o.y1, d),
            backs[o.kind],
            grain=o.kind == "door",
            **back_kw,
        )
