"""Realistic 14th-century buildings for the battle and campaign maps (lot BR1, ADR 0021).

Recipes build into a :class:`kit_geometry.Geometry` (metres, Z up, front facing -Y, footprint
centred on the origin, ground at z = 0, foundations down to z = -2 so that buildings sit on
slopes). Two levels of detail:

* ``high`` (battles): windows and doors cut into the walls with reveals and shutters, timber
  framing in relief (posts, rails, braces, close studding), jettied upper floors with joist ends,
  thick roof pans with overhangs and ridge tiles, chimneys, dormers;
* ``low`` (campaign models): same volumes, roofs and chimneys, painted openings, main timbers.

Materials are names only (``Plaster``, ``Rubble``, ``Ashlar``, ``Timber``, ``Planks``, ``Door``,
``RoofTile``, ``RoofFlat``, ``RoofSlate``, ``Thatch``, ``Window``, ``Iron``, ``Canvas``,
``Banner``); Godot maps them to shared PBR materials (``building_materials.gd``).
"""

import math
import random
from contextlib import contextmanager
from dataclasses import dataclass, field

import kit_geometry as k
from kit_geometry import Geometry, Opening

FOUNDATION_DEPTH = 2.0
PLINTH = 0.35

# Vertex tints (multiplied with the textures in Godot).
PLASTER_TINTS = [
    (1.0, 0.97, 0.9),  # lime white
    (0.98, 0.9, 0.74),  # ochre wash
    (0.97, 0.86, 0.78),  # pinkish
    (0.9, 0.88, 0.82),  # grey lime
    (0.95, 0.86, 0.66),  # yellow ochre
]
DAUB_TINTS = [(0.86, 0.74, 0.58), (0.8, 0.7, 0.56), (0.9, 0.8, 0.64)]
TIMBER_TINTS = [
    (0.62, 0.52, 0.44),
    (0.5, 0.4, 0.33),
    (0.7, 0.42, 0.32),
    (0.44, 0.36, 0.3),
]
STONE_TINTS = [
    (1.0, 0.97, 0.92),
    (0.95, 0.92, 0.86),
    (0.9, 0.9, 0.9),
    (1.0, 0.94, 0.84),
]
ROOF_TINTS = {
    "RoofTile": [
        (1.0, 0.95, 0.9),
        (0.9, 0.82, 0.76),
        (1.0, 0.88, 0.78),
        (0.82, 0.8, 0.72),
    ],
    "RoofFlat": [(1.0, 0.95, 0.92), (0.9, 0.86, 0.82), (0.8, 0.8, 0.74)],
    "RoofSlate": [(1.0, 1.0, 1.0), (0.9, 0.92, 0.95), (0.85, 0.86, 0.84)],
    "Thatch": [
        (1.0, 0.96, 0.9),
        (0.86, 0.8, 0.72),
        (0.74, 0.7, 0.62),
        (0.95, 0.9, 0.78),
    ],
}
WHITE = (1.0, 1.0, 1.0)
# Lot TF (ADR 0105 addendum): surface of the panels of half-timbered walls (``style.frame``).
# ``high`` detail: daub infill (torchis) under the modelled ``Timber`` beams; ``low`` detail
# (only plates and corner posts modelled): painted beam lattice. Last layers of the atlas.
FRAME_PANEL = "TimberFrame"
FRAME_PANEL_FAR = "TimberFrameFar"


def frame_panel(detail: str) -> str:
    """Panel surface of a half-timbered wall at a level of detail."""
    return FRAME_PANEL if detail == "high" else FRAME_PANEL_FAR


@dataclass
class Style:
    """Architectural choices of one building."""

    wall: str = "Plaster"  # Plaster | Rubble | Ashlar | Planks
    frame: str | None = None  # None | rural | close | cross
    roof: str = "RoofTile"  # RoofTile | RoofFlat | RoofSlate | Thatch
    shape: str = "gable"  # gable | hip | half_hip
    pitch: float = 52.0
    floors: int = 1
    jetty: float = 0.0
    ground_stone: bool = False  # stone ground floor under timber upper floors
    gable_front: bool = False  # town house: gable end towards the street
    shop: bool = False
    shutters: float = 0.5  # probability
    chimneys: int = 1
    dormers: int = 0
    wall_tint: tuple = WHITE
    timber_tint: tuple = TIMBER_TINTS[0]
    roof_tint: tuple = WHITE
    stone_tint: tuple = WHITE
    window: str = "small"  # small | mullion | wide
    ruined: bool = False
    extras: dict = field(default_factory=dict)


@contextmanager
def frame(g: Geometry, offset: k.Vec, yaw: float = 0.0):
    """Nest a local frame (offset in the current frame, then extra yaw)."""
    saved = (g.offset, g.yaw)
    g.offset = g.to_world(offset)
    g.yaw = g.yaw + yaw
    try:
        yield
    finally:
        g.offset, g.yaw = saved


@contextmanager
def tinted(g: Geometry, tint: tuple):
    """Temporarily change the vertex tint."""
    saved = g.tint
    g.tint = tint
    try:
        yield
    finally:
        g.tint = saved


# --- roofs ----------------------------------------------------------------------------------


def roof_thickness(roof: str) -> float:
    """Visible thickness of the roof pan at the eaves."""
    return {"Thatch": 0.5, "RoofSlate": 0.14, "RoofFlat": 0.16}.get(roof, 0.2)


def roof_gable(
    g,
    length,
    depth,
    z_eave,
    style: Style,
    detail="high",
    overhang=None,
    gable_over=None,
):
    """Two-pan roof, ridge along local X. Returns the ridge height (top surface)."""
    pitch = math.radians(style.pitch)
    tp = math.tan(pitch)
    t = roof_thickness(style.roof)
    oe = overhang if overhang is not None else (0.55 if style.roof == "Thatch" else 0.4)
    og = (
        gable_over
        if gable_over is not None
        else (0.35 if style.roof == "Thatch" else 0.25)
    )
    lift = t / math.cos(pitch)
    zr = z_eave + depth / 2 * tp + lift
    ze = z_eave - oe * tp + lift
    x0, x1, ye = -length / 2 - og, length / 2 + og, depth / 2 + oe
    with tinted(g, style.roof_tint):
        cell = 1.6 if detail == "high" else 0.0
        k.slab(
            g,
            [(x0, -ye, ze), (x1, -ye, ze), (x1, 0, zr), (x0, 0, zr)],
            t,
            style.roof,
            cell=cell,
        )
        k.slab(
            g,
            [(x1, ye, ze), (x0, ye, ze), (x0, 0, zr), (x1, 0, zr)],
            t,
            style.roof,
            cell=cell,
        )
        _ridge(g, x0, x1, zr, style, detail)
    return zr


def roof_hip(
    g, length, depth, z_eave, style: Style, detail="high", end_run=None, overhang=None
):
    """Hipped roof (``end_run`` = horizontal run of the end pans; small = half-hip look)."""
    pitch = math.radians(style.pitch)
    tp = math.tan(pitch)
    t = roof_thickness(style.roof)
    oe = overhang if overhang is not None else (0.55 if style.roof == "Thatch" else 0.4)
    lift = t / math.cos(pitch)
    zr = z_eave + depth / 2 * tp + lift
    ze = z_eave - oe * tp + lift
    run = depth / 2 if end_run is None else end_run
    hr = max(length / 2 - run, 0.0)
    X, Y = length / 2 + oe, depth / 2 + oe
    cell = 1.6 if detail == "high" else 0.0
    with tinted(g, style.roof_tint):
        if hr > 1e-3:
            k.slab(
                g,
                [(-X, -Y, ze), (X, -Y, ze), (hr, 0, zr), (-hr, 0, zr)],
                t,
                style.roof,
                cell=cell,
            )
            k.slab(
                g,
                [(X, Y, ze), (-X, Y, ze), (-hr, 0, zr), (hr, 0, zr)],
                t,
                style.roof,
                cell=cell,
            )
            _ridge(g, -hr, hr, zr, style, detail)
        else:
            k.slab(g, [(-X, -Y, ze), (X, -Y, ze), (0, 0, zr)], t, style.roof, cell=cell)
            k.slab(g, [(X, Y, ze), (-X, Y, ze), (0, 0, zr)], t, style.roof, cell=cell)
        k.slab(g, [(X, -Y, ze), (X, Y, ze), (hr, 0, zr)], t, style.roof, cell=cell)
        k.slab(g, [(-X, Y, ze), (-X, -Y, ze), (-hr, 0, zr)], t, style.roof, cell=cell)
        if detail == "high" and style.roof != "Thatch":
            for sx in (-1, 1):
                for sy in (-1, 1):
                    k.beam(
                        g,
                        (sx * X, sy * Y, ze + 0.02),
                        (sx * hr, 0, zr + 0.02),
                        0.22,
                        0.1,
                        style.roof,
                        (0, 0, 1),
                        occlude=False,
                    )
    return zr


def _ridge(g, x0, x1, zr, style, detail):
    if style.roof == "Thatch":
        k.prism_x(g, x0 + 0.1, x1 - 0.1, 0.0, zr - 0.35, 0.55, 0.5, "Thatch")
    elif detail == "high":
        k.prism_x(
            g,
            x0,
            x1,
            0.0,
            zr - 0.08,
            0.2,
            0.2,
            style.roof if style.roof != "RoofSlate" else "RoofTile",
        )


def gable_wall(g, x, depth, z_eave, pitch_deg, mat, sign, color=None, vent=False):
    """Triangular gable at ``x`` (sign +1 faces +X)."""
    h = depth / 2 * math.tan(math.radians(pitch_deg))
    a, b, c = (
        (x, -sign * depth / 2, z_eave),
        (x, sign * depth / 2, z_eave),
        (x, 0.0, z_eave + h),
    )
    g.poly([a, b, c], mat, color=color)
    if vent:
        # Attic hatch (gerbière) painted dark, 1 cm proud.
        d = 0.01 * sign
        y = 0.4
        g.quad(
            (x + d, -sign * y, z_eave + 0.5),
            (x + d, sign * y, z_eave + 0.5),
            (x + d, sign * y, z_eave + 1.3),
            (x + d, -sign * y, z_eave + 1.3),
            "Window",
        )
    return h


# --- walls, openings, timber framing ---------------------------------------------------------


def window_size(style: Style, floor: int) -> tuple[float, float, float]:
    """(width, height, sill) of a window."""
    if style.window == "mullion":
        return 1.0, 1.35, 0.85
    if style.window == "wide":
        return 1.1, 1.2, 0.8
    return (0.65, 0.8, 1.0) if floor == 0 else (0.7, 0.85, 0.85)


def plan_openings(
    rng, width, height, style: Style, floor: int, role: str
) -> list[Opening]:
    """Openings of one wall of one floor. ``role``: front | back | side."""
    ops: list[Opening] = []
    if width < 1.6:
        return ops
    bays = max(1, int(width / (2.6 if style.window == "small" else 2.2)))
    bay_w = width / bays
    ww, wh, sill = window_size(style, floor)
    door_bay = -1
    if (
        floor == 0
        and role == "front"
        or floor == 0
        and role == "back"
        and rng.random() < 0.4
    ):
        door_bay = rng.randrange(bays)
    for b in range(bays):
        cx = (b + 0.5) * bay_w + rng.uniform(-0.12, 0.12) * bay_w
        if b == door_bay:
            if style.shop and role == "front":
                dw = min(bay_w - 0.5, 2.2)
                ops.append(Opening(cx - dw / 2, 0.45, cx + dw / 2, 2.1, "shop"))
            else:
                dw = 1.05 if style.floors > 1 or style.wall != "Planks" else 2.6
                dh = 2.05 if dw < 2 else 2.8
                ops.append(Opening(cx - dw / 2, 0.0, cx + dw / 2, dh, "door"))
            continue
        if role == "side" and rng.random() < 0.55:
            continue
        if role != "side" and rng.random() < (0.25 if floor == 0 else 0.12):
            continue
        if style.shop and floor == 0 and role == "front":
            dw = min(bay_w - 0.5, 2.0)
            ops.append(Opening(cx - dw / 2, 0.55, cx + dw / 2, 2.0, "shop"))
            continue
        w = min(ww, bay_w - 0.5)
        top = min(sill + wh, height - 0.35)
        if top - sill > 0.4 and w > 0.3:
            ops.append(Opening(cx - w / 2, sill, cx + w / 2, top, "window"))
    return ops


def timber_frame(g, origin, along, width, height, ops, style: Style, rng, detail):
    """Posts, plates, rails and braces in relief on a wall face (``origin`` = outer bottom-left)."""
    outward = (along[1], -along[0], 0.0)
    proud = 0.045
    tw = 0.17
    color = style.timber_tint

    def P(x, y):
        return k.add(
            k.add(origin, k.mul(along, x)),
            (outward[0] * proud * 0.5, outward[1] * proud * 0.5, y),
        )

    def bm(p0, p1, w=tw):
        k.beam(g, p0, p1, w, proud, "Timber", outward, color=color)

    # Plates (sole plate on the plinth, wall plate under the eaves).
    bm(P(0, 0.09), P(width, 0.09), 0.2)
    bm(P(0, height - 0.1), P(width, height - 0.1), 0.2)
    if detail != "high":
        for x in (0.08, width - 0.08):
            bm(P(x, 0.0), P(x, height))
        return
    spacing = {"close": 0.62, "cross": 1.35, "rural": 1.7}.get(style.frame, 1.5)
    posts = {0.08, width - 0.08}
    for o in ops:
        posts.add(max(o.x0 - tw / 2, 0.08))
        posts.add(min(o.x1 + tw / 2, width - 0.08))
    xs = sorted(posts)
    filled = []
    for a, b in zip(xs, xs[1:], strict=False):
        filled.append(a)
        gap = b - a
        inside = any(o.x0 - 0.1 <= a and b <= o.x1 + 0.1 for o in ops)
        if gap > spacing * 1.25 and not inside:
            n = int(gap / spacing)
            for i in range(1, n + 1):
                filled.append(a + gap * i / (n + 1))
    filled.append(xs[-1])
    xs = sorted(set(round(x, 3) for x in filled))
    mid = height * 0.46

    def blocked(x0, x1, y):
        return any(
            o.x0 - 0.05 < x1 and o.x1 + 0.05 > x0 and o.y0 - 0.05 < y < o.y1 + 0.05
            for o in ops
        )

    for x in xs:
        # A post stops at the lintel / sill of an opening it would cross.
        crossing = [o for o in ops if o.x0 + 0.02 < x < o.x1 - 0.02]
        if not crossing:
            bm(P(x, 0.18), P(x, height - 0.2))
        else:
            for o in crossing:
                if o.y0 > 0.3:
                    bm(P(x, 0.18), P(x, o.y0 - 0.09))
                if o.y1 < height - 0.4:
                    bm(P(x, o.y1 + 0.09), P(x, height - 0.2))
    for a, b in zip(xs, xs[1:], strict=False):
        if b - a < 0.25:
            continue
        if not blocked(a, b, mid):
            bm(P(a, mid), P(b, mid), 0.15)
        for o in ops:
            if o.x0 - 0.1 <= a and b <= o.x1 + 0.1:
                if o.y0 > 0.3:
                    bm(P(a, o.y0 - 0.06), P(b, o.y0 - 0.06), 0.13)
                if o.y1 < height - 0.3:
                    bm(P(a, o.y1 + 0.06), P(b, o.y1 + 0.06), 0.13)
        free_low = not any(o.x0 < b and o.x1 > a and o.y0 < mid for o in ops)
        free_high = not any(o.x0 < b and o.x1 > a and o.y1 > mid for o in ops)
        if style.frame == "cross" and free_low and b - a > 0.7:
            bm(P(a, 0.2), P(b, mid - 0.08), 0.13)
            bm(P(b, 0.2), P(a, mid - 0.08), 0.13)
        elif (
            style.frame in ("rural", "cross")
            and free_high
            and b - a > 0.6
            and rng.random() < 0.7
        ):
            if rng.random() < 0.5:
                bm(P(a, mid + 0.08), P(b, height - 0.2), 0.14)
            else:
                bm(P(b, mid + 0.08), P(a, height - 0.2), 0.14)
        elif style.frame == "close" and free_low and rng.random() < 0.25:
            bm(P(a, 0.2), P(b, mid - 0.08), 0.12)


def dress_openings(g, origin, along, ops, style: Style, rng, detail, stone: bool):
    """Shutters, sills, mullions, lintels and door leaves around cut openings."""
    outward = (along[1], -along[0], 0.0)
    inward = k.mul(outward, -1.0)

    def P(x, y, d=0.0):
        return k.add(
            k.add(k.add(origin, k.mul(along, x)), (0.0, 0.0, y)), k.mul(outward, d)
        )

    for o in ops:
        w, h = o.x1 - o.x0, o.y1 - o.y0
        if o.kind == "window":
            if stone:
                k.beam(
                    g,
                    P(o.x0 - 0.12, o.y0 - 0.06, 0.04),
                    P(o.x1 + 0.12, o.y0 - 0.06, 0.04),
                    0.12,
                    0.1,
                    style.wall if style.wall != "Plaster" else "Ashlar",
                    outward,
                    color=style.stone_tint,
                )
                k.beam(
                    g,
                    P(o.x0 - 0.1, o.y1 + 0.08, 0.02),
                    P(o.x1 + 0.1, o.y1 + 0.08, 0.02),
                    0.16,
                    0.06,
                    style.wall if style.wall != "Plaster" else "Ashlar",
                    outward,
                    color=style.stone_tint,
                )
            if style.window == "mullion" and detail == "high":
                c = k.add(P((o.x0 + o.x1) / 2, (o.y0 + o.y1) / 2), k.mul(inward, 0.1))
                k.box(
                    g,
                    c,
                    (0.1, 0.1, h),
                    "Ashlar" if stone else "Timber",
                    yaw=g_yaw(along),
                    color=style.stone_tint if stone else style.timber_tint,
                )
                c2 = k.add(P((o.x0 + o.x1) / 2, o.y0 + h * 0.62), k.mul(inward, 0.1))
                k.box(
                    g,
                    c2,
                    (w, 0.1, 0.1),
                    "Ashlar" if stone else "Timber",
                    yaw=g_yaw(along),
                    color=style.stone_tint if stone else style.timber_tint,
                )
            if rng.random() < style.shutters:
                closed = rng.random() < 0.2
                if closed:
                    c = k.add(
                        P((o.x0 + o.x1) / 2, (o.y0 + o.y1) / 2), k.mul(inward, 0.04)
                    )
                    k.box(g, c, (w, 0.05, h), "Planks", yaw=g_yaw(along), grain=True)
                else:
                    for side in (-1, 1):
                        x = o.x0 - w / 4 - 0.03 if side < 0 else o.x1 + w / 4 + 0.03
                        c = P(x, (o.y0 + o.y1) / 2, 0.06)
                        k.box(
                            g,
                            c,
                            (w / 2, 0.04, h),
                            "Planks",
                            yaw=g_yaw(along),
                            grain=True,
                            color=style.extras.get("shutter_tint", WHITE),
                        )
        elif o.kind == "door":
            if stone:
                k.beam(
                    g,
                    P(o.x0 - 0.2, o.y1 + 0.14, 0.03),
                    P(o.x1 + 0.2, o.y1 + 0.14, 0.03),
                    0.28,
                    0.08,
                    "Ashlar",
                    outward,
                    color=style.stone_tint,
                )
            else:
                for x in (o.x0 - 0.07, o.x1 + 0.07):
                    k.beam(
                        g,
                        P(x, 0.0, 0.03),
                        P(x, o.y1 + 0.1, 0.03),
                        0.14,
                        0.07,
                        "Timber",
                        outward,
                        color=style.timber_tint,
                    )
                k.beam(
                    g,
                    P(o.x0 - 0.15, o.y1 + 0.08, 0.03),
                    P(o.x1 + 0.15, o.y1 + 0.08, 0.03),
                    0.16,
                    0.07,
                    "Timber",
                    outward,
                    color=style.timber_tint,
                )
            if w > 2.0 and rng.random() < 0.6:
                # Barn door left ajar: one leaf swung open against the wall.
                c = P(o.x1 + w / 4 + 0.05, h / 2, 0.08)
                k.box(g, c, (w / 2, 0.08, h), "Door", yaw=g_yaw(along), grain=True)
        elif o.kind == "shop":
            # Shop front: lower shutter dropped as a counter, upper shutter raised as an awning.
            c = k.add(P((o.x0 + o.x1) / 2, o.y0 - 0.02), k.mul(outward, 0.35))
            k.box(g, c, (w, 0.7, 0.07), "Planks", yaw=g_yaw(along))
            top = k.add(P((o.x0 + o.x1) / 2, o.y1 + 0.2), k.mul(outward, 0.4))
            ax = along
            ay = k.norm(k.add(k.mul(outward, -1.0), (0, 0, -0.9)))
            az = k.norm(k.cross(ax, ay))
            k.obox(
                g,
                top,
                (ax, ay, az),
                (w / 2, 0.45, 0.035),
                "Planks",
                back=True,
                bottom=True,
            )
            if rng.random() < 0.5:
                for dx in (-w * 0.3, 0.0, w * 0.3):
                    c = k.add(
                        P((o.x0 + o.x1) / 2 + dx, o.y0 + 0.18),
                        k.mul(outward, 0.35 + rng.uniform(-0.1, 0.1)),
                    )
                    k.cylinder(g, c, 0.16, 0.3, "Planks", sides=6)


def g_yaw(along) -> float:
    """Yaw of a wall direction (for boxes aligned with a wall)."""
    return math.atan2(along[1], along[0])


def floor_walls(
    g,
    rng,
    x0,
    y0,
    x1,
    y1,
    z,
    height,
    style: Style,
    floor,
    detail,
    wall_mat,
    framed,
    roles,
):
    """Four walls of one storey on rectangle (x0, y0)-(x1, y1) at height ``z``."""
    corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    stone = wall_mat in ("Rubble", "Ashlar")
    if framed:
        # Lot TF: daub panels of a half-timbered wall carry their own atlas layer.
        wall_mat = frame_panel(detail)
    depth = 0.3 if stone else 0.14
    for i in range(4):
        a, b = corners[i], corners[(i + 1) % 4]
        along = k.norm((b[0] - a[0], b[1] - a[1], 0.0))
        width = math.hypot(b[0] - a[0], b[1] - a[1])
        origin = (a[0], a[1], z)
        role = roles[i]
        ops = plan_openings(rng, width, height, style, floor, role) if role else []
        tint = style.stone_tint if stone else style.wall_tint
        if detail == "high":
            reveal = "Timber" if framed else wall_mat
            k.wall(
                g,
                origin,
                along,
                width,
                height,
                wall_mat,
                ops,
                depth=depth,
                reveal_mat=reveal,
                color=tint,
            )
            dress_openings(g, origin, along, ops, style, rng, detail, stone)
        else:
            k.wall(g, origin, along, width, height, wall_mat, [], color=tint)
            outward = (along[1], -along[0], 0.0)
            for o in ops:
                d = k.mul(outward, 0.03)

                def P(x, y, d=d, origin=origin, along=along):
                    return k.add(
                        k.add(k.add(origin, k.mul(along, x)), (0.0, 0.0, y)), d
                    )

                mat = "Door" if o.kind == "door" else "Window"
                g.quad(P(o.x0, o.y0), P(o.x1, o.y0), P(o.x1, o.y1), P(o.x0, o.y1), mat)
        if framed:
            timber_frame(g, origin, along, width, height, ops, style, rng, detail)


# --- chimneys, dormers ------------------------------------------------------------------------


def chimney(g, x, y, z_base, z_top, mat, tint, detail):
    """Masonry stack with a projecting cap."""
    k.box(
        g, (x, y, (z_base + z_top) / 2), (0.75, 0.95, z_top - z_base), mat, color=tint
    )
    if detail == "high":
        k.box(g, (x, y, z_top + 0.06), (0.95, 1.15, 0.12), mat, color=tint)
        k.box(g, (x, y, z_top + 0.2), (0.5, 0.5, 0.2), mat, color=tint)
        g.quad(
            (x - 0.2, y - 0.2, z_top + 0.31),
            (x + 0.2, y - 0.2, z_top + 0.31),
            (x + 0.2, y + 0.2, z_top + 0.31),
            (x - 0.2, y + 0.2, z_top + 0.31),
            "Window",
            occlude=False,
        )


def dormer(g, x, y_wall, z_eave, style: Style, rng, detail):
    """Gabled dormer window standing on the front pan (gable towards -Y)."""
    w, h = 1.3, 1.5
    dstyle = Style(roof=style.roof, pitch=55, roof_tint=style.roof_tint)
    base = z_eave + 0.25
    with frame(g, (x, y_wall + 0.25, 0.0), math.pi / 2):
        # Local frame: +X = world +Y (into the roof), front gable at local -X.
        depth_in = 2.4
        k.wall(
            g,
            (-0.0, w / 2, base),
            (0.0, -1.0, 0.0),
            w,
            h,
            style.wall if style.wall != "Planks" else "Plaster",
            [Opening(0.3, 0.3, w - 0.3, h - 0.15)] if detail == "high" else [],
            depth=0.12,
            reveal_mat="Timber",
            color=style.wall_tint,
        )
        for sy in (-1, 1):
            a, b = (depth_in, sy * w / 2, base), (0.0, sy * w / 2, base)
            if sy < 0:
                a, b = b, a
            g.quad(
                a,
                b,
                (b[0], b[1], base + h),
                (a[0], a[1], base + h),
                style.wall if style.wall != "Planks" else "Plaster",
                color=style.wall_tint,
            )
        with frame(g, (depth_in / 2 - 0.05, 0.0, 0.0)):
            gable_wall(
                g,
                -depth_in / 2 + 0.05,
                w,
                base + h,
                55,
                style.wall if style.wall != "Planks" else "Plaster",
                -1,
                color=style.wall_tint,
            )
            roof_gable(
                g, depth_in, w, base + h, dstyle, detail, overhang=0.18, gable_over=0.15
            )


# --- houses -----------------------------------------------------------------------------------


def house(
    g: Geometry,
    rng: random.Random,
    length: float,
    depth: float,
    style: Style,
    detail="high",
) -> dict:
    """Generic dwelling: plinth, storeys (jettied), roof, chimneys, dormers. Returns metrics."""
    if style.gable_front:
        with frame(g, (0.0, 0.0, 0.0), math.pi / 2):
            gf = Style(**{**style.__dict__, "gable_front": False})
            gf.extras = {**style.extras, "front_is_gable": True, "rotated": True}
            info = house(g, rng, depth, length, gf, detail)
            return {**info, "length": info["depth"], "depth": info["length"]}
    front_gable = style.extras.get("front_is_gable", False)
    floor_h = [2.85] + [2.55] * (style.floors - 1)
    if style.floors == 1 and style.roof == "Thatch":
        floor_h = [2.35]
    stone_tint = style.stone_tint
    plinth_mat = "Ashlar" if style.wall == "Ashlar" else "Rubble"
    # Foundations and plinth (the part above z = 0 is the visible stone course).
    k.box(
        g,
        (0.0, 0.0, (PLINTH - FOUNDATION_DEPTH) / 2),
        (length + 0.12, depth + 0.12, PLINTH + FOUNDATION_DEPTH),
        plinth_mat,
        color=stone_tint,
    )
    z = PLINTH
    x0, y0, x1, y1 = -length / 2, -depth / 2, length / 2, depth / 2
    # Roles by wall index: 0 front(-Y), 1 +X, 2 back(+Y), 3 -X.
    roles = ["front", "side", "back", "side"]
    if front_gable:
        roles = ["side", "back", "side", "front"]
    ruined = style.ruined
    top_floor = style.floors if not ruined else max(1, style.floors - 1)
    jx0, jy0, jx1, jy1 = x0, y0, x1, y1
    for f in range(top_floor):
        h = floor_h[f]
        if f > 0 and style.jetty > 0:
            j = style.jetty
            if front_gable:
                jx0 -= j
            else:
                jy0 -= j
                jy1 += j * 0.6
            _jetty_details(
                g, jx0, jy0, jx1, jy1, x0, y0, x1, y1, z, style, detail, front_gable
            )
            x0, y0, x1, y1 = jx0, jy0, jx1, jy1
        if style.ground_stone and f == 0:
            wall_mat, framed = ("Ashlar" if style.wall == "Ashlar" else "Rubble"), False
        else:
            wall_mat = style.wall
            framed = style.frame is not None and wall_mat == "Plaster"
        if ruined:
            _ruined_walls(g, rng, x0, y0, x1, y1, z, h, wall_mat, style, detail)
        else:
            floor_walls(
                g, rng, x0, y0, x1, y1, z, h, style, f, detail, wall_mat, framed, roles
            )
        z += h
    if ruined:
        _ruin_debris(g, rng, x0, y0, x1, y1, style, detail)
        return {"height": z, "length": x1 - x0, "depth": y1 - y0}
    g.eave_z = z
    L, D = x1 - x0, y1 - y0
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    with frame(g, (cx, cy, 0.0)):
        if style.shape == "gable":
            wm = (
                style.wall
                if not style.ground_stone or style.floors == 1
                else style.wall
            )
            gwall = (
                "Plaster"
                if wm == "Planks" and style.extras.get("plaster_gable")
                else wm
            )
            for sign in (-1, 1):
                gable_wall(
                    g,
                    sign * L / 2,
                    D,
                    z,
                    style.pitch,
                    frame_panel(detail) if style.frame and gwall == "Plaster" else gwall,
                    sign,
                    color=style.stone_tint
                    if gwall in ("Rubble", "Ashlar")
                    else style.wall_tint,
                    vent=style.extras.get("vent", False) and sign < 0,
                )
                if style.frame and gwall == "Plaster" and detail == "high":
                    h = D / 2 * math.tan(math.radians(style.pitch))
                    xo = sign * (L / 2 + 0.02)
                    out = (sign, 0.0, 0.0)
                    k.beam(
                        g,
                        (xo, 0.0, z),
                        (xo, 0.0, z + h - 0.2),
                        0.17,
                        0.045,
                        "Timber",
                        out,
                        color=style.timber_tint,
                    )
                    k.beam(
                        g,
                        (xo, -D * 0.3, z + h * 0.4),
                        (xo, D * 0.3, z + h * 0.4),
                        0.15,
                        0.045,
                        "Timber",
                        out,
                        color=style.timber_tint,
                    )
                    for sy in (-1, 1):
                        k.beam(
                            g,
                            (xo, sy * D / 2, z + 0.05),
                            (xo, 0.0, z + h - 0.1),
                            0.16,
                            0.045,
                            "Timber",
                            out,
                            color=style.timber_tint,
                        )
            zr = roof_gable(g, L, D, z, style, detail)
        elif style.shape == "hip":
            zr = roof_hip(g, L, D, z, style, detail)
        else:
            zr = roof_hip(g, L, D, z, style, detail, end_run=D * 0.22)
        chim_mat = "Rubble" if style.wall in ("Rubble", "Planks") else "Ashlar"
        spots = []
        if style.chimneys >= 1:
            spots.append(-L / 2 + 0.9 if style.shape == "gable" else -L / 4)
        if style.chimneys >= 2:
            spots.append(L / 2 - 0.9 if style.shape == "gable" else L / 4)
        for sx in spots:
            chimney(
                g,
                sx,
                rng.uniform(-0.2, 0.2) if style.shape == "gable" else 0.0,
                z - 0.5,
                zr + 0.7,
                chim_mat,
                stone_tint,
                detail,
            )
        for i in range(style.dormers):
            dx = (i + 0.5) / style.dormers * L - L / 2
            dormer(g, dx, -D / 2, z, style, rng, detail)
    g.eave_z = None
    return {
        "height": zr,
        "length": L,
        "depth": D,
        "eave": z,
        "ridge_axis": "y" if style.extras.get("rotated") else "x",
    }


def _jetty_details(
    g, jx0, jy0, jx1, jy1, x0, y0, x1, y1, z, style, detail, front_gable
):
    """Soffit under the overhang, joist ends and the bressummer beam."""
    tint = style.timber_tint
    if front_gable:
        g.quad(
            (jx0, y1, z), (x0, y1, z), (x0, y0, z), (jx0, y0, z), "Timber", color=tint
        )
        if detail == "high":
            n = int((y1 - y0) / 0.45)
            for i in range(n + 1):
                yy = y0 + 0.1 + (y1 - y0 - 0.2) * i / max(n, 1)
                k.box(
                    g,
                    ((jx0 + x0) / 2, yy, z - 0.08),
                    (x0 - jx0 + 0.02, 0.14, 0.16),
                    "Timber",
                    color=tint,
                    bottom=True,
                )
            k.box(
                g,
                (jx0 + 0.1, (y0 + y1) / 2, z + 0.1),
                (0.24, y1 - y0 + 0.1, 0.24),
                "Timber",
                color=tint,
            )
        return
    for ya, yb in ((jy0, y0), (y1, jy1)):
        if abs(yb - ya) < 1e-3:
            continue
        g.quad((x0, yb, z), (x1, yb, z), (x1, ya, z), (x0, ya, z), "Timber", color=tint)
        if detail == "high":
            n = int((x1 - x0) / 0.45)
            for i in range(n + 1):
                xx = x0 + 0.1 + (x1 - x0 - 0.2) * i / max(n, 1)
                k.box(
                    g,
                    (xx, (ya + yb) / 2, z - 0.08),
                    (0.14, abs(yb - ya) + 0.02, 0.16),
                    "Timber",
                    color=tint,
                    bottom=True,
                )
            ye = ya if ya < y0 else yb
            k.box(
                g,
                ((x0 + x1) / 2, ye + (0.1 if ya < y0 else -0.1), z + 0.1),
                (x1 - x0 + 0.1, 0.24, 0.24),
                "Timber",
                color=tint,
            )


def _ruined_walls(g, rng, x0, y0, x1, y1, z, h, wall_mat, style, detail):
    """Burned-out shell: walls with a jagged broken top, charred."""
    corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    mat = wall_mat if wall_mat != "Plaster" else "Plaster"
    for i in range(4):
        a, b = corners[i], corners[(i + 1) % 4]
        width = math.hypot(b[0] - a[0], b[1] - a[1])
        along = k.norm((b[0] - a[0], b[1] - a[1], 0.0))
        n = max(3, int(width / 0.6))
        base_s = rng.uniform(0, 50)
        tops = []
        for j in range(n + 1):
            t = j / n
            corner = max(1.0 - t * width / 1.2, 1.0 - (1.0 - t) * width / 1.2, 0.0)
            nz = k.noise3((base_s + t * width / 2.2, i * 7.0, 0.0), 1.0)
            level = 0.12 + 0.88 * nz**1.6
            tops.append(h * max(level, 0.55 + 0.45 * corner) * rng.uniform(0.92, 1.0))
        inward = (-along[1], along[0], 0.0)
        for j in range(n):
            xa, xb = width * j / n, width * (j + 1) / n
            pa = (a[0] + along[0] * xa, a[1] + along[1] * xa, z)
            pb = (a[0] + along[0] * xb, a[1] + along[1] * xb, z)
            ta, tb = tops[j], tops[j + 1]
            tint = style.stone_tint if mat in ("Rubble", "Ashlar") else style.wall_tint
            g.quad(
                pa, pb, (pb[0], pb[1], z + tb), (pa[0], pa[1], z + ta), mat, color=tint
            )
            # Inner face and wall top (thickness 0.35) so the shell reads as masonry.
            ia, ib = k.add(pa, k.mul(inward, 0.35)), k.add(pb, k.mul(inward, 0.35))
            g.quad(
                ib, ia, (ia[0], ia[1], z + ta), (ib[0], ib[1], z + tb), mat, color=tint
            )
            g.quad(
                (pa[0], pa[1], z + ta),
                (pb[0], pb[1], z + tb),
                (ib[0], ib[1], z + tb),
                (ia[0], ia[1], z + ta),
                mat,
                color=tint,
            )


def _ruin_debris(g, rng, x0, y0, x1, y1, style, detail):
    """Collapsed charred rafters and rubble heaps inside a ruin."""
    for _ in range(6 if detail == "high" else 2):
        p0 = (rng.uniform(x0, x1), rng.uniform(y0, y1), rng.uniform(0.3, 0.6))
        p1 = (
            p0[0] + rng.uniform(-3, 3),
            p0[1] + rng.uniform(-2, 2),
            rng.uniform(0.3, 2.2),
        )
        k.beam(g, p0, p1, 0.2, 0.2, "Timber", (0, 0, 1), color=(0.25, 0.22, 0.2))
    for _ in range(3):
        c = (rng.uniform(x0 + 1, x1 - 1), rng.uniform(y0 + 1, y1 - 1), 0.2)
        k.cone(
            g,
            c,
            rng.uniform(0.9, 1.6),
            rng.uniform(0.5, 0.9),
            "Rubble",
            sides=7,
            phase=rng.random(),
        )


# --- recipes --------------------------------------------------------------------------------


def pick(rng, items):
    """Random choice."""
    return items[rng.randrange(len(items))]


def southern_style(st: Style, rng) -> None:
    """Lot TF: southern (Midi) variant of a house style, applied after the usual draws.

    No exposed framing (plastered or stone walls), low-pitched canal tile roof, gable. Only the
    ``southern=True`` variants call it, so the random stream of the existing seeds is unchanged.
    """
    st.frame = None
    st.roof = pick(rng, ["RoofTile", "RoofTile", "RoofFlat"])
    st.roof_tint = pick(rng, ROOF_TINTS[st.roof])
    st.shape = "gable"
    st.pitch = rng.uniform(26.0, 34.0)
    st.jetty = 0.0
    st.dormers = 0


def cottage(
    g, rng, detail="high", length=None, depth=None, ruined=False, southern=False
):
    """Rural cottage: one storey, daub or rubble walls, thick thatch (sometimes tiles)."""
    L = length or rng.uniform(7.5, 10.5)
    D = depth or rng.uniform(5.0, 6.2)
    rubble = rng.random() < 0.45
    roof = "Thatch" if rng.random() < 0.72 else pick(rng, ["RoofFlat", "RoofTile"])
    st = Style(
        wall="Rubble" if rubble else "Plaster",
        frame=None if rubble else "rural",
        roof=roof,
        shape=pick(rng, ["gable", "half_hip", "hip"]) if roof == "Thatch" else "gable",
        pitch=rng.uniform(50, 56) if roof == "Thatch" else rng.uniform(42, 50),
        floors=1,
        shutters=0.7,
        chimneys=1 if rng.random() < 0.8 else 0,
        wall_tint=pick(rng, DAUB_TINTS),
        timber_tint=pick(rng, TIMBER_TINTS[:2] + TIMBER_TINTS[3:]),
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        extras={"vent": rng.random() < 0.5},
        ruined=ruined,
    )
    if southern:
        southern_style(st, rng)
    info = house(g, rng, L, D, st, detail)
    if not ruined and detail == "high" and rng.random() < 0.5:
        _lean_to(g, rng, L, D, st, detail)
    return info


def _lean_to(g, rng, L, D, st, detail):
    """Woodshed lean-to against a gable end, with a log pile."""
    side = pick(rng, [-1, 1])
    w, d, h = 2.2, D * 0.8, 2.1
    x = side * (L / 2 + w / 2)
    k.box(g, (x, 0.0, h / 2 - 0.5), (w, d, h + 1.0), "Planks", color=WHITE, grain=True)
    pitch_top = h + 0.7
    xa, xb = (x - side * w / 2 - side * 0.05), (x + side * w / 2 + side * 0.3)
    pts = [
        (xb, -d / 2 - 0.3, h),
        (xb, d / 2 + 0.3, h),
        (xa, d / 2 + 0.3, pitch_top),
        (xa, -d / 2 - 0.3, pitch_top),
    ]
    if side < 0:
        pts = [pts[1], pts[0], pts[3], pts[2]]
    k.slab(
        g, pts, 0.1, "Planks" if st.roof == "Thatch" else st.roof, color=st.roof_tint
    )
    # Log pile under the eaves.
    for row in range(4):
        for i in range(5 - row):
            px = x - 0.9 + 0.36 * i + 0.18 * row
            pz = 0.14 + row * 0.26
            k.beam(
                g,
                (px, -d / 2 - 0.1, pz),
                (px, -d / 2 - 0.85, pz),
                0.24,
                0.24,
                "Timber",
                (0, 0, 1),
                color=(0.72, 0.6, 0.48),
            )


def longere(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Long farmhouse: rubble or ashlar, low, thatch or slate, attic hatch, two chimneys."""
    L = length or rng.uniform(14.0, 19.0)
    D = depth or rng.uniform(5.8, 6.8)
    roof = pick(rng, ["Thatch", "RoofSlate", "Thatch", "RoofFlat"])
    st = Style(
        wall=pick(rng, ["Rubble", "Rubble", "Ashlar"]),
        roof=roof,
        shape="gable",
        pitch=rng.uniform(48, 55),
        floors=1,
        shutters=0.6,
        chimneys=2,
        dormers=1 if roof != "Thatch" and rng.random() < 0.6 else 0,
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        extras={"vent": True},
        ruined=ruined,
    )
    return house(g, rng, L, D, st, detail)


def timber_house(
    g, rng, detail="high", length=None, depth=None, ruined=False, southern=False
):
    """Two-storey half-timbered house with a jettied upper floor (village or town)."""
    L = length or rng.uniform(8.0, 12.0)
    D = depth or rng.uniform(6.0, 7.5)
    roof = pick(rng, ["RoofFlat", "RoofTile", "RoofFlat", "RoofSlate", "Thatch"])
    st = Style(
        wall="Plaster",
        frame=pick(rng, ["rural", "cross", "close"]),
        roof=roof,
        shape="gable" if rng.random() < 0.75 else "half_hip",
        pitch=rng.uniform(50, 58),
        floors=2,
        jetty=rng.uniform(0.35, 0.6),
        ground_stone=rng.random() < 0.35,
        shutters=0.55,
        chimneys=pick(rng, [1, 1, 2]),
        dormers=pick(rng, [0, 0, 1]) if roof != "Thatch" else 0,
        wall_tint=pick(rng, PLASTER_TINTS),
        timber_tint=pick(rng, TIMBER_TINTS),
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        ruined=ruined,
    )
    if southern:
        southern_style(st, rng)
    return house(g, rng, L, D, st, detail)


def town_house(
    g, rng, detail="high", length=None, depth=None, ruined=False, southern=False
):
    """Narrow town house, gable on the street, 2-3 jettied storeys, shop on the ground floor."""
    W = length or rng.uniform(5.2, 7.2)  # frontage
    D = depth or rng.uniform(9.0, 12.0)
    roof = pick(rng, ["RoofFlat", "RoofTile", "RoofSlate", "RoofFlat"])
    st = Style(
        wall="Plaster",
        frame=pick(rng, ["close", "close", "cross"]),
        roof=roof,
        shape="gable",
        pitch=rng.uniform(55, 62),
        floors=pick(rng, [2, 3, 3]),
        jetty=rng.uniform(0.35, 0.55),
        ground_stone=rng.random() < 0.5,
        gable_front=True,
        shop=rng.random() < 0.7,
        shutters=0.45,
        window=pick(rng, ["wide", "mullion", "wide"]),
        chimneys=1,
        wall_tint=pick(rng, PLASTER_TINTS),
        timber_tint=pick(rng, TIMBER_TINTS),
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        ruined=ruined,
    )
    if southern:
        southern_style(st, rng)
    return house(g, rng, W, D, st, detail)


def stone_house(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Bourgeois or presbytery stone house: ashlar, mullioned windows, slate or tiles, dormers."""
    L = length or rng.uniform(10.0, 14.0)
    D = depth or rng.uniform(7.0, 8.5)
    roof = pick(rng, ["RoofSlate", "RoofSlate", "RoofFlat", "RoofTile"])
    st = Style(
        wall=pick(rng, ["Ashlar", "Ashlar", "Rubble"]),
        roof=roof,
        shape=pick(rng, ["gable", "gable", "hip"]),
        pitch=rng.uniform(50, 58),
        floors=2,
        window="mullion",
        shutters=0.3,
        chimneys=2,
        dormers=pick(rng, [1, 2, 2]),
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        ruined=ruined,
    )
    return house(g, rng, L, D, st, detail)


def barn(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Barn: stone plinth, vertical boarding on a timber frame, big doors, half-hip roof."""
    L = length or rng.uniform(12.0, 18.0)
    D = depth or rng.uniform(7.5, 9.5)
    roof = pick(rng, ["Thatch", "RoofFlat", "RoofTile"])
    st = Style(
        wall="Planks",
        roof=roof,
        shape=pick(rng, ["half_hip", "gable"]),
        pitch=rng.uniform(48, 55),
        floors=1,
        shutters=0.0,
        chimneys=0,
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        ruined=ruined,
    )
    st.extras["tall"] = True
    return _barn_body(g, rng, L, D, st, detail)


def _barn_body(g, rng, L, D, st, detail):
    h = 4.6
    k.box(
        g,
        (0.0, 0.0, (0.7 - FOUNDATION_DEPTH) / 2),
        (L + 0.1, D + 0.1, 0.7 + FOUNDATION_DEPTH),
        "Rubble",
        color=st.stone_tint,
    )
    z = 0.7
    x0, y0, x1, y1 = -L / 2, -D / 2, L / 2, D / 2
    corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    for i in range(4):
        a, b = corners[i], corners[(i + 1) % 4]
        along = k.norm((b[0] - a[0], b[1] - a[1], 0.0))
        width = math.hypot(b[0] - a[0], b[1] - a[1])
        ops = []
        if i in (0, 2) and (i == 0 or rng.random() < 0.5):
            cx = width * rng.uniform(0.4, 0.6)
            ops.append(Opening(cx - 1.6, 0.0, cx + 1.6, 3.4, "door"))
        if i == 1 and rng.random() < 0.6:
            ops.append(Opening(width / 2 - 0.6, 3.0, width / 2 + 0.6, 4.1, "dark"))
        origin = (a[0], a[1], z)
        if st.ruined:
            continue
        if detail == "high":
            k.wall(
                g,
                origin,
                along,
                width,
                h,
                "Planks",
                ops,
                depth=0.12,
                reveal_mat="Timber",
                back_mats={"door": "Window"},
                grain=True,
            )
            dress_openings(g, origin, along, ops, st, rng, detail, False)
            outward = (along[1], -along[0], 0.0)
            for x in [0.1, width - 0.1] + [width * t for t in (0.25, 0.5, 0.75)]:
                if any(o.x0 - 0.2 < x < o.x1 + 0.2 for o in ops):
                    continue
                p0 = k.add(k.add(origin, k.mul(along, x)), k.mul(outward, 0.03))
                k.beam(
                    g,
                    p0,
                    (p0[0], p0[1], z + h),
                    0.22,
                    0.06,
                    "Timber",
                    outward,
                    color=st.timber_tint,
                )
        else:
            k.wall(g, origin, along, width, h, "Planks", [], grain=True)
            for o in ops:
                outward = (along[1], -along[0], 0.0)

                def P(x, y, origin=origin, along=along, outward=outward):
                    return k.add(
                        k.add(k.add(origin, k.mul(along, x)), (0.0, 0.0, y)),
                        k.mul(outward, 0.03),
                    )

                g.quad(
                    P(o.x0, o.y0),
                    P(o.x1, o.y0),
                    P(o.x1, o.y1),
                    P(o.x0, o.y1),
                    "Door" if o.kind == "door" else "Window",
                )
    if st.ruined:
        _ruined_walls(g, rng, x0, y0, x1, y1, 0.0, 1.6, "Rubble", st, detail)
        _ruin_debris(g, rng, x0, y0, x1, y1, st, detail)
        return {"height": 1.6, "length": L, "depth": D}
    z += h
    g.eave_z = z
    if st.shape == "gable":
        for sign in (-1, 1):
            gable_wall(g, sign * L / 2, D, z, st.pitch, "Planks", sign)
        zr = roof_gable(g, L, D, z, st, detail)
    else:
        zr = roof_hip(g, L, D, z, st, detail, end_run=D * 0.22)
    g.eave_z = None
    return {"height": zr, "length": L, "depth": D, "eave": z}


def church(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Parish church: nave, lower choir with a polygonal apse, west tower with a spire, porch."""
    L = length or rng.uniform(26.0, 32.0)
    W = depth or rng.uniform(9.0, 11.0)
    tint = pick(rng, STONE_TINTS)
    roof = pick(rng, ["RoofSlate", "RoofSlate", "RoofFlat"])
    st = Style(
        wall="Ashlar",
        roof=roof,
        pitch=54,
        stone_tint=tint,
        roof_tint=pick(rng, ROOF_TINTS[roof]),
    )
    tower = min(max(L * 0.21, 4.4), 6.8)
    nave_l = L * 0.52
    choir_l = L - nave_l - tower
    h = min(max(W * 0.82, 6.5), 11.0)
    nx0 = -L / 2 + tower
    # Foundations: nave at full width, choir narrower (no slab visible around the apse).
    k.box(
        g,
        (nx0 + nave_l / 2, 0.0, -FOUNDATION_DEPTH / 2 + 0.2),
        (nave_l, W + 0.2, FOUNDATION_DEPTH + 0.4),
        "Ashlar",
        color=tint,
    )
    k.box(
        g,
        (nx0 + nave_l + choir_l / 2 - W * 0.2, 0.0, -FOUNDATION_DEPTH / 2 + 0.2),
        (choir_l - W * 0.4, W * 0.78, FOUNDATION_DEPTH + 0.4),
        "Ashlar",
        color=tint,
    )
    k.box(
        g,
        (-L / 2 + tower / 2, 0.0, -FOUNDATION_DEPTH / 2),
        (tower + 0.2, tower + 0.2, FOUNDATION_DEPTH),
        "Ashlar",
        color=tint,
    )
    # Nave.
    with frame(g, (nx0 + nave_l / 2, 0.0, 0.0)):
        _stone_hall(
            g,
            rng,
            nave_l,
            W,
            0.4,
            h,
            st,
            detail,
            lancets=True,
            buttresses=True,
            door_side=True,
        )
        g.eave_z = h + 0.4
        for sign in (1,):
            gable_wall(
                g, sign * nave_l / 2, W, h + 0.4, st.pitch, "Ashlar", sign, color=tint
            )
        roof_gable(g, nave_l, W, h + 0.4, st, detail, overhang=0.3, gable_over=0.1)
        g.eave_z = None
        # South porch.
        if detail == "high":
            with frame(g, (-nave_l * 0.2, -W / 2 - 1.6, 0.0), 0.0):
                k.box(g, (0.0, 0.0, 1.8), (3.2, 3.2, 3.6), "Ashlar", color=tint)
                k.wall(
                    g,
                    (-1.0, -1.61, 0.0),
                    (1.0, 0.0, 0.0),
                    2.0,
                    2.9,
                    "Ashlar",
                    [Opening(0.3, 0.0, 1.7, 2.6, "dark")],
                    depth=0.5,
                    color=tint,
                )
                with frame(g, (0.0, 0.0, 0.0), math.pi / 2):
                    roof_gable(
                        g, 3.2, 3.2, 3.6, st, detail, overhang=0.2, gable_over=0.2
                    )
    # Choir with a three-sided apse.
    cw = W * 0.78
    ch = h * 0.85
    cx0 = nx0 + nave_l
    with frame(g, (cx0 + choir_l / 2 - cw * 0.25, 0.0, 0.0)):
        cl = max(choir_l - cw * 0.5, 1.5)
        _stone_hall(
            g,
            rng,
            cl,
            cw,
            0.4,
            ch,
            st,
            detail,
            lancets=True,
            buttresses=True,
            ends=False,
        )
        g.eave_z = ch + 0.4
        roof_gable(g, cl, cw, ch + 0.4, st, detail, overhang=0.3, gable_over=0.0)
        _apse(g, cl / 2, cw, 0.4, ch, st, detail)
        g.eave_z = None
    # West tower with belfry and octagonal spire.
    tx = -L / 2 + tower / 2
    th = h * rng.uniform(1.9, 2.3)
    with frame(g, (tx, 0.0, 0.0)):
        _tower(g, rng, tower, th, st, detail)
        spire = th * rng.uniform(0.6, 0.8)
        with tinted(g, st.roof_tint):
            k.cone(
                g,
                (0.0, 0.0, th + 0.4),
                tower * 0.58,
                spire,
                "RoofSlate",
                sides=8,
                phase=math.pi / 8,
            )
            if detail == "high":
                for i in range(4):
                    a = math.pi / 4 + i * math.pi / 2
                    c = (math.cos(a) * tower * 0.5, math.sin(a) * tower * 0.5, th + 0.4)
                    k.cone(g, c, 0.9, 2.2, "RoofSlate", sides=4, phase=a)
        _cross(g, (0.0, 0.0, th + 0.4 + spire), detail)
    return {"height": th + spire, "length": L, "depth": W}


def _stone_hall(
    g,
    rng,
    L,
    W,
    z,
    h,
    st,
    detail,
    lancets=False,
    buttresses=False,
    ends=True,
    door_side=False,
    win_lo=0.35,
    win_hi=0.8,
    bay=4.2,
    win_w=0.9,
):
    """Ashlar rectangle with lancet windows and stepped buttresses (church nave, choir, hall)."""
    x0, y0, x1, y1 = -L / 2, -W / 2, L / 2, W / 2
    corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    for i in range(4):
        if not ends and i in (1, 3):
            continue
        a, b = corners[i], corners[(i + 1) % 4]
        along = k.norm((b[0] - a[0], b[1] - a[1], 0.0))
        width = math.hypot(b[0] - a[0], b[1] - a[1])
        ops = []
        bays = max(1, int(width / bay))
        if lancets and i in (0, 2):
            for bi in range(bays):
                cx = (bi + 0.5) * width / bays
                if door_side and i == 0 and bi == 1:
                    ops.append(Opening(cx - 0.8, 0.0, cx + 0.8, 3.0, "door"))
                    continue
                ops.append(
                    Opening(
                        cx - win_w / 2, h * win_lo, cx + win_w / 2, h * win_hi, "window"
                    )
                )
        elif lancets and i == 1 and ends:
            ops.append(
                Opening(width / 2 - 0.7, h * 0.3, width / 2 + 0.7, h * 0.85, "window")
            )
        origin = (a[0], a[1], z)
        outward = (along[1], -along[0], 0.0)
        if detail == "high":
            k.wall(
                g,
                origin,
                along,
                width,
                h,
                "Ashlar",
                ops,
                depth=0.55,
                color=st.stone_tint,
            )
            for o in ops:
                if o.kind == "window":
                    # Pointed arch head painted dark above the rectangular light.
                    cx = (o.x0 + o.x1) / 2
                    top = o.y1 + (o.x1 - o.x0) * 0.75

                    def P(x, y, origin=origin, along=along, outward=outward):
                        return k.add(
                            k.add(k.add(origin, k.mul(along, x)), (0.0, 0.0, y)),
                            k.mul(outward, 0.012),
                        )

                    g.poly(
                        [
                            P(o.x0, o.y1),
                            P(o.x1, o.y1),
                            P(o.x1 - 0.08, o.y1 + 0.35),
                            P(cx, top),
                            P(o.x0 + 0.08, o.y1 + 0.35),
                        ],
                        "Window",
                    )
                    k.beam(
                        g,
                        P(o.x0 - 0.12, o.y0 - 0.1),
                        P(o.x1 + 0.12, o.y0 - 0.1),
                        0.14,
                        0.12,
                        "Ashlar",
                        outward,
                        color=st.stone_tint,
                    )
                    k.beam(
                        g,
                        P(cx, o.y0),
                        P(cx, o.y1 + 0.1),
                        0.1,
                        0.02,
                        "Ashlar",
                        k.mul(outward, -1.0),
                        color=st.stone_tint,
                    )
                elif o.kind == "door":
                    cx = (o.x0 + o.x1) / 2

                    def P(x, y, d=0.0, origin=origin, along=along, outward=outward):
                        return k.add(
                            k.add(k.add(origin, k.mul(along, x)), (0.0, 0.0, y)),
                            k.mul(outward, d),
                        )

                    g.poly(
                        [
                            P(o.x0 - 0.3, o.y1, 0.05),
                            P(o.x1 + 0.3, o.y1, 0.05),
                            P(o.x1 + 0.3, o.y1 + 0.4, 0.05),
                            P(cx, o.y1 + 1.3, 0.05),
                            P(o.x0 - 0.3, o.y1 + 0.4, 0.05),
                        ],
                        "Ashlar",
                        color=st.stone_tint,
                    )
            if buttresses and i in (0, 2):
                for bi in range(bays + 1):
                    x = bi * width / bays
                    if x < 0.3 or x > width - 0.3:
                        x = min(max(x, 0.45), width - 0.45)
                    base = k.add(k.add(origin, k.mul(along, x)), k.mul(outward, 0.45))
                    k.box(
                        g,
                        (base[0], base[1], z + h * 0.3 - 1.0),
                        (0.9, 0.9, h * 0.6 + 2.0),
                        "Ashlar",
                        yaw=g_yaw(along),
                        color=st.stone_tint,
                    )
                    top = k.add(k.add(origin, k.mul(along, x)), k.mul(outward, 0.3))
                    k.box(
                        g,
                        (top[0], top[1], z + h * 0.72),
                        (0.7, 0.6, h * 0.25),
                        "Ashlar",
                        yaw=g_yaw(along),
                        color=st.stone_tint,
                    )
        else:
            k.wall(g, origin, along, width, h, "Ashlar", [], color=st.stone_tint)
            for o in ops:

                def P(x, y, origin=origin, along=along, outward=outward):
                    return k.add(
                        k.add(k.add(origin, k.mul(along, x)), (0.0, 0.0, y)),
                        k.mul(outward, 0.03),
                    )

                g.quad(
                    P(o.x0, o.y0), P(o.x1, o.y0), P(o.x1, o.y1), P(o.x0, o.y1), "Window"
                )


def _apse(g, x_start, W, z, h, st, detail):
    """Three-sided apse (half octagon) closing a choir at ``x_start``, with its roof."""
    r = W / 2
    pts = [
        (x_start + r * math.sin(a), -r * math.cos(a))
        for a in (0.0, math.pi / 4, math.pi / 2, 3 * math.pi / 4, math.pi)
    ]
    apex_z = z + h + r * math.tan(math.radians(st.pitch))
    for (ax, ay), (bx, by) in zip(pts, pts[1:], strict=False):
        g.quad(
            (ax, ay, z),
            (bx, by, z),
            (bx, by, z + h),
            (ax, ay, z + h),
            "Ashlar",
            color=st.stone_tint,
        )
        mx, my = (ax + bx) / 2, (ay + by) / 2
        if detail == "high":
            n = k.norm((mx - x_start, my, 0.0))
            p = (mx + n[0] * 0.02, my + n[1] * 0.02, 0.0)
            t = k.norm((bx - ax, by - ay, 0.0))
            g.quad(
                k.add(p, (-t[0] * 0.4, -t[1] * 0.4, z + h * 0.35)),
                k.add(p, (t[0] * 0.4, t[1] * 0.4, z + h * 0.35)),
                k.add(p, (t[0] * 0.4, t[1] * 0.4, z + h * 0.82)),
                k.add(p, (-t[0] * 0.4, -t[1] * 0.4, z + h * 0.82)),
                "Window",
            )
        o = 0.35
        with tinted(g, st.roof_tint):
            ea = (x_start + (ax - x_start) * (1 + o / r), ay * (1 + o / r), z + h - 0.2)
            eb = (x_start + (bx - x_start) * (1 + o / r), by * (1 + o / r), z + h - 0.2)
            k.slab(g, [ea, eb, (x_start, 0.0, apex_z)], 0.14, st.roof)


def _tower(g, rng, size, height, st, detail):
    """Square bell tower: west portal, slit windows, belfry openings, cornice."""
    s = size / 2
    corners = [(-s, -s), (s, -s), (s, s), (-s, s)]
    for i in range(4):
        a, b = corners[i], corners[(i + 1) % 4]
        along = k.norm((b[0] - a[0], b[1] - a[1], 0.0))
        ops = [
            Opening(
                size / 2 - 0.4, height - 4.2, size / 2 - 0.05, height - 1.2, "dark"
            ),
            Opening(
                size / 2 + 0.05, height - 4.2, size / 2 + 0.4, height - 1.2, "dark"
            ),
        ]
        ops.append(
            Opening(
                size / 2 - 0.12,
                height * 0.45,
                size / 2 + 0.12,
                height * 0.45 + 1.1,
                "dark",
            )
        )
        if i == 3:
            ops.append(Opening(size / 2 - 0.9, 0.0, size / 2 + 0.9, 3.4, "door"))
        k.wall(
            g,
            (a[0], a[1], 0.0),
            along,
            size,
            height,
            "Ashlar",
            ops if detail == "high" else [],
            depth=0.6,
            color=st.stone_tint,
        )
        if detail != "high":
            outward = (along[1], -along[0], 0.0)
            for o in ops:
                p = k.add((a[0], a[1], 0.0), k.mul(outward, 0.03))
                g.quad(
                    k.add(p, (along[0] * o.x0, along[1] * o.x0, o.y0)),
                    k.add(p, (along[0] * o.x1, along[1] * o.x1, o.y0)),
                    k.add(p, (along[0] * o.x1, along[1] * o.x1, o.y1)),
                    k.add(p, (along[0] * o.x0, along[1] * o.x0, o.y1)),
                    "Window",
                )
    # Cornice and corner buttresses.
    k.box(
        g,
        (0.0, 0.0, height + 0.2),
        (size + 0.5, size + 0.5, 0.4),
        "Ashlar",
        color=st.stone_tint,
    )
    if detail == "high":
        for sx in (-1, 1):
            for sy in (-1, 1):
                k.box(
                    g,
                    (sx * (s + 0.25), sy * (s - 0.3), height * 0.3 - 1.0),
                    (0.5, 0.8, height * 0.6 + 2.0),
                    "Ashlar",
                    color=st.stone_tint,
                )
                k.box(
                    g,
                    (sx * (s - 0.3), sy * (s + 0.25), height * 0.3 - 1.0),
                    (0.8, 0.5, height * 0.6 + 2.0),
                    "Ashlar",
                    color=st.stone_tint,
                )
        k.box(
            g,
            (0.0, 0.0, height - 4.35),
            (size + 0.3, size + 0.3, 0.25),
            "Ashlar",
            color=st.stone_tint,
        )


def _cross(g, base, detail):
    """Iron cross (or weathercock rod) on a spire."""
    x, y, z = base
    k.box(g, (x, y, z + 0.9), (0.1, 0.1, 1.8), "Iron")
    k.box(g, (x, y, z + 1.3), (0.8, 0.08, 0.08), "Iron")


def manor(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Fortified manor (maison forte): square tower with a steep hipped roof and a stone hall."""
    tint = pick(rng, STONE_TINTS)
    roof = pick(rng, ["RoofSlate", "RoofFlat"])
    st = Style(
        wall="Ashlar",
        roof=roof,
        pitch=62,
        stone_tint=tint,
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        window="mullion",
        floors=2,
        dormers=1,
        chimneys=2,
        shutters=0.2,
    )
    hall_l, hall_d = rng.uniform(13.0, 16.0), rng.uniform(7.5, 8.5)
    with frame(g, (2.0, 0.0, 0.0)):
        house(g, rng, hall_l, hall_d, st, detail)
    ts = 7.0
    th = rng.uniform(14.0, 17.0)
    with frame(g, (-hall_l / 2 - ts / 2 + 2.5, -0.5, 0.0)):
        k.box(
            g,
            (0.0, 0.0, -FOUNDATION_DEPTH / 2),
            (ts + 0.3, ts + 0.3, FOUNDATION_DEPTH),
            "Ashlar",
            color=tint,
        )
        s = ts / 2
        corners = [(-s, -s), (s, -s), (s, s), (-s, s)]
        for i in range(4):
            a, b = corners[i], corners[(i + 1) % 4]
            along = k.norm((b[0] - a[0], b[1] - a[1], 0.0))
            ops = [
                Opening(ts / 2 - 0.15, y, ts / 2 + 0.15, y + 1.0, "dark")
                for y in (3.5, 7.0)
            ]
            ops.append(
                Opening(ts / 2 - 0.5, th - 3.2, ts / 2 + 0.5, th - 1.8, "window")
            )
            if i == 0:
                ops.append(Opening(1.0, 0.0, 2.2, 2.5, "door"))
            k.wall(
                g,
                (a[0], a[1], 0.0),
                along,
                ts,
                th,
                "Ashlar",
                ops if detail == "high" else [],
                depth=0.7,
                color=tint,
            )
        # Machicolated parapet band then a steep pyramid roof.
        k.box(g, (0.0, 0.0, th + 0.5), (ts + 0.8, ts + 0.8, 1.0), "Ashlar", color=tint)
        if detail == "high":
            for i in range(9):
                for side in range(4):
                    t = -s - 0.2 + (ts + 0.4) * i / 8
                    pos = [(t, -s - 0.3), (s + 0.3, t), (t, s + 0.3), (-s - 0.3, t)][
                        side
                    ]
                    k.box(
                        g,
                        (pos[0], pos[1], th - 0.2),
                        (0.3, 0.3, 0.5),
                        "Ashlar",
                        color=tint,
                    )
        g.eave_z = th + 1.0
        tst = Style(roof=roof, pitch=64, roof_tint=st.roof_tint)
        zr = roof_hip(g, ts + 0.6, ts + 0.6, th + 1.0, tst, detail, overhang=0.2)
        g.eave_z = None
        _cross(g, (0.0, 0.0, zr), detail)
    return {"height": th + ts, "length": hall_l + ts, "depth": max(hall_d, ts)}


def market_hall(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Covered market (halle): oak posts on stone pads under a huge low tiled roof."""
    L = length or rng.uniform(18.0, 24.0)
    D = depth or rng.uniform(11.0, 14.0)
    roof = pick(rng, ["RoofFlat", "RoofTile"])
    st = Style(roof=roof, pitch=45, roof_tint=pick(rng, ROOF_TINTS[roof]))
    h = 3.2
    nx, ny = int(L / 3.6), int(D / 3.6)
    for i in range(nx + 1):
        for j in range(ny + 1):
            if 0 < i < nx and 0 < j < ny and (i + j) % 2:
                continue
            x, y = -L / 2 + L * i / nx, -D / 2 + D * j / ny
            k.box(g, (x, y, 0.15), (0.6, 0.6, 0.5), "Ashlar", color=WHITE)
            k.box(
                g,
                (x, y, 0.4 + h / 2),
                (0.3, 0.3, h),
                "Timber",
                grain=True,
                color=TIMBER_TINTS[1],
            )
            if detail == "high":
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    if not (-L / 2 <= x + dx <= L / 2 and -D / 2 <= y + dy <= D / 2):
                        continue
                    k.beam(
                        g,
                        (x, y, 0.4 + h - 0.9),
                        (x + dx * 0.9, y + dy * 0.9, 0.4 + h),
                        0.14,
                        0.14,
                        "Timber",
                        (dy, -dx, 0),
                        color=TIMBER_TINTS[1],
                    )
    for j in range(ny + 1):
        y = -D / 2 + D * j / ny
        k.box(
            g,
            (0.0, y, 0.4 + h + 0.15),
            (L + 0.2, 0.32, 0.3),
            "Timber",
            color=TIMBER_TINTS[1],
        )
    k.box(
        g, (0.0, 0.0, 0.02), (L + 1.0, D + 1.0, 0.1), "Ashlar", color=(0.8, 0.78, 0.74)
    )
    g.eave_z = None
    zr = roof_hip(g, L, D, 0.4 + h + 0.3, st, detail, end_run=D * 0.35, overhang=0.9)
    return {"height": zr, "length": L, "depth": D}


def well(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Village well: round rubble curb, two posts, a small roof, a windlass and a bucket."""
    k.cylinder(
        g,
        (0.0, 0.0, -0.3),
        1.0,
        1.2,
        "Rubble",
        sides=12 if detail == "high" else 8,
        color=WHITE,
    )
    g.poly(
        [
            (math.cos(a) * 0.8, math.sin(a) * 0.8, 0.91)
            for a in [2 * math.pi * i / 10 for i in range(10)]
        ],
        "Window",
    )
    for sx in (-1, 1):
        k.box(
            g,
            (sx * 0.85, 0.0, 1.4),
            (0.14, 0.14, 2.3),
            "Timber",
            grain=True,
            color=TIMBER_TINTS[0],
        )
    k.box(g, (0.0, 0.0, 1.9), (1.8, 0.12, 0.12), "Timber", color=TIMBER_TINTS[0])
    st = Style(roof="RoofFlat", pitch=45)
    roof_gable(g, 2.0, 1.4, 2.5, st, detail, overhang=0.25, gable_over=0.1)
    k.cylinder(g, (0.3, 0.0, 1.2), 0.18, 0.3, "Planks", sides=6)
    return {"height": 3.4, "length": 2.2, "depth": 2.2}


def windmill(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Post mill: timber trestle on stone piers, boarded body with a gable roof, four sails."""
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.box(g, (sx * 2.2, sy * 2.2, 0.3), (0.9, 0.9, 0.8), "Rubble", color=WHITE)
            k.beam(
                g,
                (sx * 2.2, sy * 2.2, 0.7),
                (0.0, 0.0, 3.4),
                0.25,
                0.25,
                "Timber",
                (sy, -sx, 0),
                color=TIMBER_TINTS[1],
            )
    k.box(
        g, (0.0, 0.0, 2.0), (0.5, 0.5, 4.0), "Timber", grain=True, color=TIMBER_TINTS[1]
    )
    bw, bd, bh = 3.4, 4.6, 5.2
    with frame(g, (0.0, 0.0, 3.8), rng.uniform(-0.6, 0.6)):
        k.box(
            g,
            (0.0, 0.0, bh / 2),
            (bw, bd, bh),
            "Planks",
            grain=True,
            color=(0.95, 0.9, 0.85),
        )
        with frame(g, (0.0, 0.0, 0.0), math.pi / 2):
            zr = roof_gable(
                g,
                bd,
                bw,
                bh,
                Style(roof="RoofFlat", pitch=38),
                detail,
                overhang=0.2,
                gable_over=0.25,
            )
        k.beam(
            g,
            (0.0, -bd / 2 + 0.5, 1.4),
            (0.0, bd / 2 + 2.6, -3.3),
            0.25,
            0.25,
            "Timber",
            (1, 0, 0),
            color=TIMBER_TINTS[0],
        )
        hub = (0.0, -bd / 2 - 0.5, bh * 0.72)
        k.box(
            g,
            (0.0, -bd / 2 - 0.25, bh * 0.72),
            (0.5, 0.9, 0.5),
            "Timber",
            color=TIMBER_TINTS[0],
        )
        phase = rng.uniform(0, math.pi / 2)
        for i in range(4):
            a = phase + i * math.pi / 2
            d = (math.cos(a), 0.0, math.sin(a))
            tip = k.add(hub, k.mul(d, 9.5))
            k.beam(g, hub, tip, 0.22, 0.18, "Timber", (0, -1, 0), color=TIMBER_TINTS[0])
            side = (-math.sin(a), 0.0, math.cos(a))
            p0 = k.add(k.add(hub, k.mul(d, 1.8)), (0.0, -0.1, 0.0))
            p1 = k.add(k.add(hub, k.mul(d, 9.3)), (0.0, -0.1, 0.0))
            q0, q1 = k.add(p0, k.mul(side, 1.7)), k.add(p1, k.mul(side, 1.7))
            g.quad(p0, p1, q1, q0, "Canvas", occlude=False)
            g.quad(q0, q1, p1, p0, "Canvas", occlude=False)
            if detail == "high":
                for t in range(1, 8):
                    a0 = k.lerp(p0, p1, t / 8)
                    k.beam(
                        g,
                        a0,
                        k.add(a0, k.mul(side, 1.75)),
                        0.06,
                        0.06,
                        "Timber",
                        (0, -1, 0),
                        color=TIMBER_TINTS[0],
                    )
    return {"height": 3.8 + zr + 4.0, "length": 7.0, "depth": 7.0}


def _rose(g, center, radius, normal_axis, sign, detail):
    """Rose window: dark disc with a stone ring, on a wall facing ``sign`` along X or Y."""
    n = 16 if detail == "high" else 10
    cx, cy, cz = center

    def P(a, r, d):
        u, v = math.cos(a) * r, math.sin(a) * r
        if normal_axis == "x":
            return (cx + sign * d, cy + u * sign, cz + v)
        return (cx - u * sign, cy + sign * d, cz + v)

    ring = [2 * math.pi * i / n for i in range(n)]
    g.poly([P(a, radius, 0.03) for a in ring], "Window")
    if detail == "high":
        for i, a in enumerate(ring):
            b = ring[(i + 1) % n]
            g.quad(
                P(a, radius, 0.06),
                P(b, radius, 0.06),
                P(b, radius * 1.18, 0.06),
                P(a, radius * 1.18, 0.06),
                "Ashlar",
                color=WHITE,
            )
            if i % 2 == 0:
                g.quad(
                    P(a - 0.03, radius * 0.15, 0.05),
                    P(a + 0.03, radius * 0.15, 0.05),
                    P(a + 0.03, radius, 0.05),
                    P(a - 0.03, radius, 0.05),
                    "Ashlar",
                    color=WHITE,
                )


def cathedral(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Gothic cathedral with twin west towers and a crossing spire.

    Clerestoried nave, aisles with flying buttresses, transept with rose windows, polygonal
    chevet.
    """
    L = length or rng.uniform(95.0, 115.0)
    W = depth or rng.uniform(28.0, 34.0)
    nave_w = W * 0.46
    aisle_w = (W - nave_w) / 2
    tint = pick(rng, STONE_TINTS[:2])
    st = Style(
        wall="Ashlar",
        roof="RoofSlate",
        pitch=56,
        stone_tint=tint,
        roof_tint=(0.85, 0.88, 0.95),
    )
    h = rng.uniform(28.0, 33.0)
    ah = h * 0.42
    z0 = 0.4
    tower = aisle_w + 2.5
    west = -L / 2
    apse_r = nave_w / 2
    body_len = L - apse_r
    k.box(
        g,
        (0.0, 0.0, (z0 - FOUNDATION_DEPTH) / 2),
        (L + 1.0, W + 3.0, z0 + FOUNDATION_DEPTH),
        "Ashlar",
        color=tint,
    )
    # Nave and choir: tall clerestory walls, windows above the aisle roofs.
    body_cx = west + body_len / 2
    with frame(g, (body_cx, 0.0, 0.0)):
        _stone_hall(
            g,
            rng,
            body_len,
            nave_w,
            z0,
            h,
            st,
            detail,
            lancets=True,
            ends=False,
            win_lo=(ah + 4.0) / h,
            win_hi=0.9,
            bay=6.5,
            win_w=2.2,
        )
        g.eave_z = h + z0
        zr = roof_gable(
            g, body_len, nave_w, h + z0, st, detail, overhang=0.3, gable_over=0.0
        )
        g.eave_z = None
    # West facade: portal, rose window, gable.
    fw = (
        [Opening(nave_w / 2 - 2.2, 0.0, nave_w / 2 + 2.2, 7.5, "door")]
        if detail == "high"
        else []
    )
    k.wall(
        g,
        (west, nave_w / 2, z0),
        (0.0, -1.0, 0.0),
        nave_w,
        h,
        "Ashlar",
        fw,
        depth=1.2,
        color=tint,
    )
    gable_wall(g, west, nave_w, h + z0, st.pitch, "Ashlar", -1, color=tint)
    _rose(g, (west, 0.0, z0 + h * 0.62), nave_w * 0.3, "x", -1, detail)
    if detail == "high":
        g.poly(
            [
                (west - 0.05, 2.2, z0 + 7.5),
                (west - 0.05, -2.2, z0 + 7.5),
                (west - 0.05, 0.0, z0 + 11.0),
            ],
            "Ashlar",
            color=tint,
        )
    # Aisles with lean-to roofs, buttress piers and flying buttresses.
    ax0 = west + tower
    alen = body_len - tower
    bays = max(2, int(alen / 6.5))
    for side in (-1, 1):
        yc = side * (nave_w / 2 + aisle_w / 2)
        with frame(g, (ax0 + alen / 2, yc, 0.0)):
            _stone_hall(
                g,
                rng,
                alen,
                aisle_w,
                z0,
                ah,
                st,
                detail,
                lancets=True,
                ends=True,
                bay=6.5,
                win_w=1.8,
                win_lo=0.3,
                win_hi=0.82,
            )
        rise = aisle_w * math.tan(math.radians(28))
        yi, yo = side * nave_w / 2, side * (W / 2 + 0.5)
        zt, zb = z0 + ah + rise, z0 + ah - 0.2
        quad = [
            (ax0, yo, zb),
            (ax0 + alen, yo, zb),
            (ax0 + alen, yi, zt),
            (ax0, yi, zt),
        ]
        if side > 0:
            quad = [quad[1], quad[0], quad[3], quad[2]]
        with tinted(g, st.roof_tint):
            k.slab(g, quad, 0.2, "RoofSlate", cell=2.0 if detail == "high" else 0.0)
        for b in range(bays + 1):
            x = ax0 + alen * b / bays
            pier_y = side * (W / 2 + 0.9)
            k.box(
                g,
                (x, pier_y, (z0 + ah + 6.0) / 2 - 1.0),
                (1.4, 1.8, z0 + ah + 8.0),
                "Ashlar",
                color=tint,
            )
            k.cone(
                g,
                (x, pier_y, z0 + ah + 7.0),
                0.9,
                4.0,
                "Ashlar",
                sides=4,
                phase=math.pi / 4,
                color=tint,
            )
            if detail == "high":
                k.beam(
                    g,
                    (x, pier_y - side * 0.5, z0 + ah + 5.0),
                    (x, side * nave_w / 2, z0 + h * 0.82),
                    0.9,
                    0.7,
                    "Ashlar",
                    (1.0, 0.0, 0.0),
                    color=tint,
                )
                k.beam(
                    g,
                    (x, pier_y - side * 0.5, z0 + ah + 2.0),
                    (x, side * nave_w / 2, z0 + h * 0.62),
                    0.7,
                    0.6,
                    "Ashlar",
                    (1.0, 0.0, 0.0),
                    color=tint,
                )
    # Transept: arms beyond the aisles, rose windows in the gables.
    tx = west + body_len * 0.58
    arm = W / 2 + 5.0
    with frame(g, (tx, 0.0, 0.0), math.pi / 2):
        for sign in (-1, 1):
            with frame(g, (sign * (nave_w / 2 + (arm - nave_w / 2) / 2), 0.0, 0.0)):
                _stone_hall(
                    g,
                    rng,
                    arm - nave_w / 2,
                    nave_w,
                    z0,
                    h,
                    st,
                    detail,
                    lancets=True,
                    ends=True,
                    bay=6.5,
                    win_w=2.0,
                    win_lo=0.4,
                    win_hi=0.85,
                )
        g.eave_z = h + z0
        roof_gable(g, arm * 2, nave_w, h + z0, st, detail, overhang=0.3, gable_over=0.2)
        for sign in (-1, 1):
            gable_wall(
                g, sign * arm, nave_w, h + z0, st.pitch, "Ashlar", sign, color=tint
            )
            _rose(g, (sign * arm, 0.0, z0 + h * 0.66), nave_w * 0.28, "x", sign, detail)
        g.eave_z = None
    # Chevet.
    with frame(g, (west + body_len, 0.0, 0.0)):
        _apse(g, 0.0, nave_w, z0, h, st, detail)
    # Twin west towers.
    top_kind = rng.random()
    for side in (-1, 1):
        with frame(g, (west + tower / 2, side * (nave_w / 2 + tower / 2 - 0.5), 0.0)):
            th = h + rng.uniform(22.0, 28.0)
            _tower(g, rng, tower, th, st, detail)
            if top_kind < 0.5:
                if detail == "high":
                    for i in range(8):
                        t = -tower / 2 + tower * (i + 0.5) / 8
                        for pos in (
                            (t, -tower / 2 - 0.1),
                            (t, tower / 2 + 0.1),
                            (-tower / 2 - 0.1, t),
                            (tower / 2 + 0.1, t),
                        ):
                            k.box(
                                g,
                                (pos[0], pos[1], th + 0.9),
                                (0.6, 0.6, 1.0),
                                "Ashlar",
                                color=tint,
                            )
            else:
                with tinted(g, st.roof_tint):
                    k.cone(
                        g,
                        (0.0, 0.0, th + 0.4),
                        tower * 0.55,
                        th * 0.75,
                        "RoofSlate",
                        sides=8,
                        phase=math.pi / 8,
                    )
    # Crossing spire (flèche) of lead-covered timber.
    spire_h = h * 1.3
    with tinted(g, (0.8, 0.82, 0.9)):
        k.box(g, (tx, 0.0, zr - 1.0), (3.0, 3.0, 5.0), "RoofSlate")
        k.cone(
            g,
            (tx, 0.0, zr + 1.5),
            2.2,
            spire_h,
            "RoofSlate",
            sides=8,
            phase=math.pi / 8,
        )
    _cross(g, (tx, 0.0, zr + 1.5 + spire_h), detail)
    return {"height": zr + spire_h, "length": L, "depth": W + 10.0}


# --- street furniture ------------------------------------------------------------------------
# Small props of lived-in streets (market stalls, carts, barrels, woodpiles). Same frame as the
# buildings: footprint centred on the origin, length along +x, front towards -y, ground at z = 0.

CLOTH_TINTS = [
    (1.0, 0.95, 0.85),  # undyed linen
    (0.95, 0.42, 0.32),  # madder red
    (0.5, 0.6, 0.85),  # woad blue
    (1.0, 0.8, 0.45),  # weld yellow
    (0.62, 0.75, 0.5),  # green
]
PRODUCE_TINTS = [
    (0.9, 0.35, 0.2),
    (0.55, 0.72, 0.3),
    (0.95, 0.7, 0.3),
    (0.6, 0.3, 0.35),
]
POTTERY = (0.95, 0.62, 0.45)


def _barrel(g, x, y, z=0.0, radius=0.3, height=0.85, detail="high", tint=None):
    """Standing oak barrel with two iron hoops."""
    sides = 10 if detail == "high" else 7
    wood = tint or TIMBER_TINTS[0]
    k.tube(
        g,
        (x, y, z),
        (x, y, z + height),
        radius,
        "Planks",
        sides,
        bulge=0.12,
        color=wood,
        grain=True,
    )
    if detail == "high":
        for t in (0.14, 0.86):
            r = radius * (1.0 + 0.12 * (1.0 - abs(t - 0.5) * 2.0)) + 0.012
            zt = z + height * t
            k.tube(
                g, (x, y, zt - 0.03), (x, y, zt + 0.03), r, "Iron", sides, caps=False
            )


def _lying_barrel(g, p0, p1, radius, detail):
    k.tube(
        g,
        p0,
        p1,
        radius,
        "Planks",
        10 if detail == "high" else 7,
        bulge=0.12,
        color=TIMBER_TINTS[1],
    )


def _crate(g, center, size, yaw=0.0, tint=None):
    k.box(g, center, size, "Planks", yaw, color=tint or TIMBER_TINTS[0])


def _sack(g, rng, x, y, z):
    k.box(
        g,
        (x, y, z + 0.2),
        (0.45, 0.32, 0.4),
        "Canvas",
        rng.uniform(-0.5, 0.5),
        color=(0.95, 0.9, 0.78),
    )


def _wheel(g, x, y, radius, width, detail):
    """Cart wheel (felloe as a thick disc, iron tyre, projecting hub)."""
    sides = 16 if detail == "high" else 10
    k.tube(
        g,
        (x, y - width / 2, radius),
        (x, y + width / 2, radius),
        radius,
        "Timber",
        sides,
        color=TIMBER_TINTS[3],
    )
    if detail == "high":
        k.tube(
            g,
            (x, y - width / 2 - 0.005, radius),
            (x, y + width / 2 + 0.005, radius),
            radius + 0.015,
            "Iron",
            sides,
            caps=False,
        )
    k.tube(
        g,
        (x, y - width, radius),
        (x, y + width, radius),
        0.1,
        "Timber",
        8,
        color=TIMBER_TINTS[1],
    )


def market_stall(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Market stall: four posts, a counter with its apron, a sloping dyed awning and goods."""
    L = length or rng.uniform(2.6, 3.4)
    D = depth or rng.uniform(1.6, 2.0)
    wood = pick(rng, TIMBER_TINTS)
    for sx in (-1, 1):
        for sy, h in ((-1, 2.05), (1, 2.45)):
            k.box(
                g,
                (sx * (L / 2 - 0.08), sy * (D / 2 - 0.08), h / 2),
                (0.1, 0.1, h),
                "Timber",
                grain=True,
                color=wood,
            )
    # Counter at the front, a plank apron down to the ground.
    k.box(g, (0.0, -D / 2 + 0.45, 0.86), (L - 0.1, 0.9, 0.06), "Planks", color=wood)
    k.box(
        g,
        (0.0, -D / 2 + 0.03, 0.43),
        (L - 0.2, 0.04, 0.8),
        "Planks",
        grain=True,
        color=wood,
    )
    # Awning: linen or dyed cloth, overhanging the customers.
    cloth = pick(rng, CLOTH_TINTS)
    k.slab(
        g,
        [
            (-L / 2 - 0.1, -D / 2 - 0.55, 1.95),
            (L / 2 + 0.1, -D / 2 - 0.55, 1.95),
            (L / 2 + 0.1, D / 2 + 0.1, 2.5),
            (-L / 2 - 0.1, D / 2 + 0.1, 2.5),
        ],
        0.03,
        "Canvas",
        color=cloth,
    )
    if detail == "high":
        # Scalloped valance along the front edge.
        k.box(
            g,
            (0.0, -D / 2 - 0.56, 1.8),
            (L + 0.2, 0.02, 0.28),
            "Canvas",
            color=cloth,
            back=True,
        )
    # Goods on the counter: crates of produce, bolts of cloth, baskets, pottery.
    trade = rng.randrange(4)
    x = -L / 2 + 0.35
    while x < L / 2 - 0.3:
        y = -D / 2 + 0.45 + rng.uniform(-0.12, 0.12)
        if trade == 0:
            _crate(g, (x, y, 1.0), (0.5, 0.38, 0.22), rng.uniform(-0.1, 0.1))
            k.box(
                g,
                (x, y, 1.1),
                (0.44, 0.32, 0.04),
                "Canvas",
                color=pick(rng, PRODUCE_TINTS),
            )
            x += 0.6
        elif trade == 1:
            k.tube(
                g,
                (x, y - 0.3, 0.97),
                (x, y + 0.3, 0.97),
                0.1,
                "Canvas",
                8,
                color=pick(rng, CLOTH_TINTS),
            )
            x += 0.24
        elif trade == 2:
            k.cylinder(
                g,
                (x, y, 0.89),
                0.2,
                0.2,
                "Thatch",
                sides=8,
                radius_top=0.24,
                color=(0.9, 0.8, 0.6),
            )
            k.cylinder(
                g,
                (x, y, 1.05),
                0.2,
                0.03,
                "Canvas",
                sides=8,
                color=pick(rng, PRODUCE_TINTS),
            )
            x += 0.5
        else:
            r = rng.uniform(0.1, 0.16)
            k.cylinder(
                g,
                (x, y, 0.89),
                r,
                rng.uniform(0.2, 0.34),
                "Plaster",
                sides=8,
                radius_top=r * 0.7,
                color=POTTERY,
            )
            x += 0.38
    # Stock behind the counter.
    _barrel(g, -L / 2 + 0.45, D / 2 - 0.4, detail=detail)
    _crate(
        g, (L / 2 - 0.45, D / 2 - 0.4, 0.2), (0.55, 0.45, 0.4), rng.uniform(-0.2, 0.2)
    )
    return {"height": 2.5, "length": L, "depth": D}


def cart(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Two-wheeled cart, unhitched, shafts resting on the ground; hay, barrels or sacks."""
    bed_l, bed_w, bed_z = 2.3, 1.15, 0.85
    x0 = 0.55  # bed centre (shafts run towards -x)
    wood = pick(rng, TIMBER_TINTS)
    k.box(g, (x0, 0.0, bed_z), (bed_l, bed_w, 0.08), "Planks", color=wood, bottom=True)
    for sy in (-1, 1):
        k.box(
            g,
            (x0, sy * (bed_w / 2 - 0.03), bed_z + 0.2),
            (bed_l, 0.05, 0.34),
            "Planks",
            color=wood,
        )
        # Shafts from under the bed to the ground in front.
        k.beam(
            g,
            (x0 + bed_l / 2 - 0.2, sy * 0.42, bed_z - 0.08),
            (x0 - bed_l / 2 - 1.9, sy * 0.36, 0.06),
            0.09,
            0.09,
            "Timber",
            (0.0, 0.0, 1.0),
            color=TIMBER_TINTS[1],
        )
        _wheel(g, x0 + 0.1, sy * (bed_w / 2 + 0.12), 0.62, 0.09, detail)
    k.box(
        g,
        (x0 + bed_l / 2 - 0.03, 0.0, bed_z + 0.2),
        (0.05, bed_w, 0.34),
        "Planks",
        color=wood,
    )
    k.tube(
        g,
        (x0 + 0.1, -bed_w / 2 - 0.2, 0.62),
        (x0 + 0.1, bed_w / 2 + 0.2, 0.62),
        0.05,
        "Timber",
        6,
    )
    load = rng.randrange(4)
    top = bed_z + 0.04
    if load == 0:
        hay = (0.95, 0.85, 0.6)
        k.box(
            g,
            (x0, 0.0, top + 0.25),
            (bed_l - 0.1, bed_w + 0.1, 0.5),
            "Thatch",
            color=hay,
        )
        k.prism_x(
            g,
            x0 - bed_l / 2 + 0.1,
            x0 + bed_l / 2 - 0.2,
            0.0,
            top + 0.5,
            bed_w / 2 + 0.05,
            0.45,
            "Thatch",
            color=hay,
        )
    elif load == 1:
        for bx in (-0.6, 0.1, 0.8):
            _barrel(g, x0 + bx, rng.uniform(-0.2, 0.2), top, 0.27, 0.75, detail)
    elif load == 2:
        for i in range(6):
            _sack(g, rng, x0 - 0.8 + (i % 3) * 0.75, -0.25 + (i // 3) * 0.5, top)
    return {"height": 1.9, "length": bed_l + 2.3, "depth": bed_w + 0.5}


def barrels(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Heap of barrels and crates against a wall (cellar delivery, cooper's yard)."""
    L = length or rng.uniform(1.8, 2.6)
    D = depth or 1.3
    x = -L / 2 + 0.35
    while x < L / 2 - 0.3:
        y = rng.uniform(-0.15, 0.2)
        if rng.random() < 0.7:
            _barrel(g, x, y, detail=detail, tint=pick(rng, TIMBER_TINTS[:2]))
            if rng.random() < 0.3:
                _barrel(g, x, y, 0.85, detail=detail)
            x += 0.68
        else:
            _crate(g, (x, y, 0.25), (0.6, 0.5, 0.5), rng.uniform(-0.3, 0.3))
            x += 0.7
    _lying_barrel(
        g, (-L / 2 + 0.3, -0.5, 0.3), (-L / 2 + 1.1, -0.45, 0.3), 0.28, detail
    )
    return {"height": 1.7, "length": L, "depth": D}


def woodpile(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Stacked firewood logs between stakes, with a chopping block."""
    L = length or rng.uniform(1.8, 2.6)
    r = 0.11
    rows = 4 if detail == "high" else 3
    for row in range(rows):
        n = 6 - row
        for i in range(n):
            y = (i - (n - 1) / 2) * r * 2.0
            z = r + row * r * 1.75
            jitter = rng.uniform(-0.08, 0.08)
            k.tube(
                g,
                (-L / 2 + jitter, y, z),
                (L / 2 + jitter, y, z),
                r * rng.uniform(0.85, 1.1),
                "Timber",
                7,
                color=pick(rng, TIMBER_TINTS),
            )
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.box(
                g,
                (sx * (L / 2 - 0.1), sy * 0.72, 0.45),
                (0.07, 0.07, 0.9),
                "Timber",
                grain=True,
                color=TIMBER_TINTS[3],
            )
    k.cylinder(
        g, (L / 2 + 0.6, -0.4, 0.0), 0.25, 0.5, "Timber", sides=9, color=TIMBER_TINTS[0]
    )
    return {"height": 0.9, "length": L + 0.9, "depth": 1.5}


# --- Bridges (lot EP3, ADR 0033) ---------------------------------------------------------------
#
# Modular pieces assembled in Godot (``battle_bridges.gd``): the deck runs along +X, the road
# surface is at z = 0 (Godot places it at the deck height of the simulation), width along Y.
# ``*_bay`` pieces are centred on the origin and repeated (then stretched along X) across the
# water; ``*_end`` pieces run from x = 0 (against the last bay) to x = +5 (on the bank), their
# ramp going down by the deck's height above the bank (2.4 m stone, 1.0 m wood, as in the
# simulation: ``hydro.rs`` ``STONE_DECK_RISE`` / ``WOOD_DECK_RISE``).

STONE_BAY = 9.0  # one arch and a pier
STONE_WIDTH = 6.0
STONE_RISE = 2.4
WOOD_BAY = 4.0
WOOD_WIDTH = 5.0
WOOD_RISE = 1.0
BRIDGE_END = 5.0
BRIDGE_BOTTOM = -7.0  # piers and walls go this deep (river bed well below)


def _face_x_z(g, y, x0, x1, z0a, z0b, z1a, z1b, mat, sign, **kw):
    """Quad in the plane y = ``y`` between (x0, z0a)-(x1, z0b) below and (x0, z1a)-(x1, z1b) above.

    ``sign`` -1 faces -Y, +1 faces +Y.
    """
    pts = [(x0, y, z0a), (x1, y, z0b), (x1, y, z1b), (x0, y, z1a)]
    if sign > 0:
        pts.reverse()
    g.poly(pts, mat, **kw)


def _arch_points(span, crown, spring, n):
    """Segmental arch intrados from x = -span/2 to +span/2 (z at each point)."""
    rise = crown - spring
    radius = (span * span / 4 + rise * rise) / (2 * rise)
    cz = crown - radius
    a0 = math.asin(min((span / 2) / radius, 1.0))
    pts = []
    for i in range(n + 1):
        a = -a0 + 2 * a0 * i / n
        pts.append((radius * math.sin(a), cz + radius * math.cos(a)))
    return pts, radius, cz


def bridge_stone_bay(g, rng, detail="high", length=None, depth=None, ruined=False):
    """One bay of a stone bridge: a segmental arch between two half piers with cutwaters,
    rubble spandrels, a voussoir ring, a string course, parapets with coping, cobbled road.
    """
    g.ground_ao = False
    L = length or STONE_BAY
    W = depth or STONE_WIDTH
    hl, hw = L / 2, W / 2
    pier = 1.0
    span = L - 2 * pier
    crown, spring = -0.8, -3.6
    n = 14 if detail == "high" else 7
    arch, radius, cz = _arch_points(span, crown, spring, n)
    stone = STONE_TINTS[rng.randrange(len(STONE_TINTS))]
    rubble = (0.9, 0.86, 0.8)
    with tinted(g, rubble):
        for sign in (-1, 1):
            y = sign * hw
            # Half piers down to the river bed.
            _face_x_z(
                g,
                y,
                -hl,
                -span / 2,
                BRIDGE_BOTTOM,
                BRIDGE_BOTTOM,
                0.0,
                0.0,
                "Rubble",
                sign,
            )
            _face_x_z(
                g,
                y,
                span / 2,
                hl,
                BRIDGE_BOTTOM,
                BRIDGE_BOTTOM,
                0.0,
                0.0,
                "Rubble",
                sign,
            )
            # Spandrel between the arch and the road.
            for (xa, za), (xb, zb) in zip(arch, arch[1:], strict=False):
                _face_x_z(g, y, xa, xb, za, zb, 0.0, 0.0, "Rubble", sign)
        # Pier faces under the springing (inside the arch opening).
        for sx in (-1, 1):
            x = sx * span / 2
            pts = [
                (x, -hw, BRIDGE_BOTTOM),
                (x, hw, BRIDGE_BOTTOM),
                (x, hw, spring),
                (x, -hw, spring),
            ]
            if sx < 0:
                pts.reverse()
            g.poly(pts, "Rubble")
    # Intrados (barrel vault underside).
    with tinted(g, stone):
        for (xa, za), (xb, zb) in zip(arch, arch[1:], strict=False):
            g.poly([(xa, -hw, za), (xa, hw, za), (xb, hw, zb), (xb, -hw, zb)], "Ashlar")
        # Voussoir ring, slightly proud of the spandrel, on both faces.
        ring = 0.55
        for sign in (-1, 1):
            y = sign * (hw + 0.06)
            for (xa, za), (xb, zb) in zip(arch, arch[1:], strict=False):
                # Outward radial offsets.
                def out(x, z):
                    dx, dz = x, z - cz
                    d = math.hypot(dx, dz) or 1.0
                    return (x + dx / d * ring, z + dz / d * ring)

                oa, ob = out(xa, za), out(xb, zb)
                pts = [(xa, y, za), (xb, y, zb), (ob[0], y, ob[1]), (oa[0], y, oa[1])]
                if sign > 0:
                    pts.reverse()
                g.poly(pts, "Ashlar")
            # String course under the parapet.
            k.box(g, (0.0, sign * (hw + 0.05), -0.12), (L, 0.2, 0.24), "Ashlar")
        # Cutwaters (half of each pointed nose at both ends of the bay), up to below the ring.
        top = spring + 1.1
        for sx in (-1, 1):
            xe = sx * hl
            xi = sx * (hl - pier)
            for sy in (-1, 1):
                ye = sy * hw
                apex = (xe, sy * (hw + 1.4))
                face = [
                    (xi, ye, BRIDGE_BOTTOM),
                    (apex[0], apex[1], BRIDGE_BOTTOM),
                    (apex[0], apex[1], top),
                    (xi, ye, top),
                ]
                if sx * sy > 0:
                    face.reverse()
                g.poly(face, "Ashlar")
                cap = [(xi, ye, top), (apex[0], apex[1], top), (xe, ye, top)]
                if sx * sy < 0:
                    cap.reverse()
                g.poly(cap, "Ashlar")
                # Sloped cap stone.
                cap_top = [(xi, ye, top), (apex[0], apex[1], top), (xe, ye, top + 0.5)]
                if sx * sy < 0:
                    cap_top.reverse()
                g.poly(cap_top, "Ashlar")
    # Parapets and coping.
    with tinted(g, rubble):
        for sign in (-1, 1):
            k.box(g, (0.0, sign * (hw - 0.22), 0.45), (L, 0.44, 0.9), "Rubble")
    with tinted(g, stone):
        for sign in (-1, 1):
            k.box(g, (0.0, sign * (hw - 0.22), 0.96), (L, 0.56, 0.14), "Ashlar")
    # Cobbled road.
    with tinted(g, (0.62, 0.6, 0.56)):
        g.poly(
            [
                (-hl, -hw + 0.44, 0.0),
                (hl, -hw + 0.44, 0.0),
                (hl, hw - 0.44, 0.0),
                (-hl, hw - 0.44, 0.0),
            ],
            "Rubble",
        )
    return {"height": 1.1 - BRIDGE_BOTTOM, "length": L, "depth": W + 2.8}


def bridge_stone_end(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Abutment of a stone bridge: a ramp from the deck (x = 0) down to the bank (x = +5), side
    walls down to the river bed, parapets following the slope, ending on a stone post.
    """
    g.ground_ao = False
    L = length or BRIDGE_END
    W = depth or STONE_WIDTH
    hw = W / 2
    drop = STONE_RISE
    stone = STONE_TINTS[rng.randrange(len(STONE_TINTS))]
    rubble = (0.9, 0.86, 0.8)

    def road(x):
        return -drop * (x / L)

    with tinted(g, rubble):
        for sign in (-1, 1):
            y = sign * hw
            _face_x_z(
                g, y, 0.0, L, BRIDGE_BOTTOM, BRIDGE_BOTTOM, 0.0, road(L), "Rubble", sign
            )
        # Parapets (sloping boxes as quads).
        for sign in (-1, 1):
            for y0, y1 in ((hw - 0.44, hw),):
                ya, yb = sign * y0, sign * y1
                lo, hi = min(ya, yb), max(ya, yb)
                # Outer and inner faces.
                for y, s in ((hi, 1), (lo, -1)):
                    _face_x_z(
                        g, y, 0.0, L, 0.0, road(L), 0.9, road(L) + 0.9, "Rubble", s
                    )
                # Top.
                g.poly(
                    [
                        (0.0, lo, 0.9),
                        (L, lo, road(L) + 0.9),
                        (L, hi, road(L) + 0.9),
                        (0.0, hi, 0.9),
                    ][::-1],
                    "Rubble",
                )
        # Wing wall face at the bank end (x = L), below the road.
        g.poly(
            [
                (L, -hw, BRIDGE_BOTTOM),
                (L, hw, BRIDGE_BOTTOM),
                (L, hw, road(L)),
                (L, -hw, road(L)),
            ],
            "Rubble",
        )
    with tinted(g, stone):
        for sign in (-1, 1):
            yc = sign * (hw - 0.22)
            # Coping along the slope.
            lo, hi = yc - 0.28, yc + 0.28
            g.poly(
                [
                    (0.0, lo, 1.04),
                    (L, lo, road(L) + 1.04),
                    (L, hi, road(L) + 1.04),
                    (0.0, hi, 1.04),
                ][::-1],
                "Ashlar",
            )
            for y, s in ((hi, 1), (lo, -1)):
                _face_x_z(
                    g, y, 0.0, L, 0.9, road(L) + 0.9, 1.04, road(L) + 1.04, "Ashlar", s
                )
            # End post.
            k.box(g, (L - 0.3, yc, road(L) + 0.7), (0.6, 0.62, 1.4), "Ashlar")
            k.box(g, (L - 0.3, yc, road(L) + 1.47), (0.72, 0.74, 0.14), "Ashlar")
            # String course.
            _face_x_z(
                g,
                sign * (hw + 0.05),
                0.0,
                L,
                -0.24,
                road(L) - 0.24,
                0.0,
                road(L),
                "Ashlar",
                sign,
            )
    with tinted(g, (0.62, 0.6, 0.56)):
        g.poly(
            [
                (0.0, -hw + 0.44, 0.0),
                (L, -hw + 0.44, road(L)),
                (L, hw - 0.44, road(L)),
                (0.0, hw - 0.44, 0.0),
            ],
            "Rubble",
        )
    return {"height": 1.5 - BRIDGE_BOTTOM, "length": L, "depth": W}


def _rail(g, x0, x1, z0, z1, y, tint):
    """Handrail and mid rail from x0 to x1 (sloping from z0 to z1), posts at both ends."""
    for h in (0.5, 0.98):
        k.beam(
            g,
            (x0, y, z0 + h),
            (x1, y, z1 + h),
            0.1,
            0.1,
            "Timber",
            (0.0, 1.0 if y > 0 else -1.0, 0.0),
            color=tint,
        )


def bridge_wood_bay(g, rng, detail="high", length=None, depth=None, ruined=False):
    """One bay of a wooden bridge: plank deck on stringers, a pile bent (three or four piles, cap
    beam, cross braces) at the +X end, railings with posts.
    """
    g.ground_ao = False
    L = length or WOOD_BAY
    W = depth or WOOD_WIDTH
    hl, hw = L / 2, W / 2
    tint = TIMBER_TINTS[rng.randrange(len(TIMBER_TINTS))]
    planks = (0.95, 0.9, 0.84)
    # Deck.
    with tinted(g, planks):
        k.box(g, (0.0, 0.0, -0.1), (L, W, 0.2), "Planks", bottom=True)
    # Stringers.
    count = 4 if W > 4.5 else 3
    for i in range(count):
        y = -hw + 0.4 + (W - 0.8) * i / (count - 1)
        k.box(g, (0.0, y, -0.4), (L, 0.26, 0.4), "Timber", bottom=True, color=tint)
    # Pile bent at +X.
    piles = (
        [-hw + 0.35, 0.0, hw - 0.35]
        if W <= 4.5
        else [-hw + 0.35, -hw / 3, hw / 3, hw - 0.35]
    )
    for y in piles:
        k.box(
            g,
            (hl - 0.2, y, (BRIDGE_BOTTOM - 0.6) / 2),
            (0.32, 0.32, -BRIDGE_BOTTOM - 0.6),
            "Timber",
            color=tint,
        )
    k.box(
        g,
        (hl - 0.2, 0.0, -0.75),
        (0.36, W + 0.3, 0.3),
        "Timber",
        bottom=True,
        color=tint,
    )
    # Cross braces between the outer piles.
    xb = hl - 0.02
    for ya, yb in ((-hw + 0.35, hw - 0.35), (hw - 0.35, -hw + 0.35)):
        k.beam(
            g,
            (xb, ya, -0.9),
            (xb, yb, -3.2),
            0.14,
            0.12,
            "Timber",
            (1.0, 0.0, 0.0),
            color=tint,
        )
    # Railing: posts at both ends and mid-bay, rails.
    for sign in (-1, 1):
        y = sign * (hw - 0.08)
        for x in (-hl + 0.1, 0.0):
            k.box(g, (x, y, 0.5), (0.14, 0.14, 1.0), "Timber", color=tint)
        _rail(g, -hl, hl, 0.0, 0.0, y, tint)
    return {"height": 1.0 - BRIDGE_BOTTOM, "length": L, "depth": W + 0.3}


def bridge_wood_end(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Landing of a wooden bridge: a plank ramp from the deck (x = 0) down to the bank (x = +5)
    on a sill beam and a pile bent, sloping railings ending on stout posts.
    """
    g.ground_ao = False
    L = length or BRIDGE_END
    W = depth or WOOD_WIDTH
    hw = W / 2
    drop = WOOD_RISE
    tint = TIMBER_TINTS[rng.randrange(len(TIMBER_TINTS))]
    planks = (0.95, 0.9, 0.84)

    def road(x):
        return -drop * (x / L)

    with tinted(g, planks):
        # Sloping deck (top and bottom).
        g.poly(
            [(0.0, -hw, 0.0), (L, -hw, road(L)), (L, hw, road(L)), (0.0, hw, 0.0)],
            "Planks",
        )
        g.poly(
            [
                (0.0, -hw, -0.2),
                (0.0, hw, -0.2),
                (L, hw, road(L) - 0.2),
                (L, -hw, road(L) - 0.2),
            ],
            "Planks",
        )
        for sign in (-1, 1):
            _face_x_z(
                g, sign * hw, 0.0, L, -0.2, road(L) - 0.2, 0.0, road(L), "Planks", sign
            )
    # Pile bent at mid-ramp and a sill on the bank.
    for y in (-hw + 0.35, hw - 0.35):
        k.box(
            g,
            (L * 0.45, y, (BRIDGE_BOTTOM + road(L * 0.45) - 0.3) / 2),
            (0.3, 0.3, road(L * 0.45) - 0.3 - BRIDGE_BOTTOM),
            "Timber",
            color=tint,
        )
    k.box(
        g,
        (L * 0.45, 0.0, road(L * 0.45) - 0.4),
        (0.34, W + 0.2, 0.26),
        "Timber",
        bottom=True,
        color=tint,
    )
    k.box(
        g,
        (L - 0.3, 0.0, road(L) - 0.45),
        (0.5, W + 0.4, 0.5),
        "Timber",
        bottom=True,
        color=tint,
    )
    for sign in (-1, 1):
        y = sign * (hw - 0.08)
        k.box(
            g,
            (L * 0.5, y, road(L * 0.5) + 0.5),
            (0.14, 0.14, 1.0),
            "Timber",
            color=tint,
        )
        k.box(g, (L - 0.15, y, road(L) + 0.6), (0.22, 0.22, 1.2), "Timber", color=tint)
        _rail(g, 0.0, L, 0.0, road(L), y, tint)
    return {"height": 1.2 - BRIDGE_BOTTOM, "length": L, "depth": W}


# --- battlefield decor (lot EP6) --------------------------------------------------------------
# Villages and battlefield dressing beyond the BR1/BR2 street furniture: watermill, camp tents
# and pavilions, hay, a baggage wagon, campfires, a churchyard wall and lychgate, graves and a
# vine row. Same frame as the buildings above unless noted.


def _mill_wheel(g, rng, x, y, radius, detail):
    """Undershot mill wheel, open construction (see-through): two thin felloe rims (short timber arcs), 8 spokes per rim to a central hub, radial paddle blades between the rims, a stub axle through the gable wall."""
    width = 0.55
    n_paddle = 14 if detail == "high" else 9
    n_rim = 16 if detail == "high" else 10
    hub_r = 0.14
    rim_r = 0.045
    hub_sides = 10 if detail == "high" else 7
    # Hub.
    k.tube(
        g,
        (x, y - width / 2, radius),
        (x, y + width / 2, radius),
        hub_r,
        "Timber",
        hub_sides,
        color=TIMBER_TINTS[0],
    )
    for sy in (-width / 2, width / 2):
        # Rim: a ring of short curved (straight-segment) timber arcs, open in the middle.
        pts = [
            (x + math.cos(a) * radius, y + sy, radius + math.sin(a) * radius)
            for a in (2 * math.pi * i / n_rim for i in range(n_rim))
        ]
        for p0, p1 in zip(pts, pts[1:] + pts[:1], strict=False):
            k.tube(g, p0, p1, rim_r, "Timber", 5, caps=False, color=TIMBER_TINTS[1])
        # Spokes: hub to rim, one per rim (8 each side).
        for i in range(8):
            a = 2 * math.pi * i / 8
            p1 = (x + math.cos(a) * radius, y + sy, radius + math.sin(a) * radius)
            k.beam(
                g,
                (x, y + sy, radius),
                p1,
                0.05,
                0.05,
                "Timber",
                (0.0, 1.0, 0.0),
                color=TIMBER_TINTS[0],
            )
    # Paddle blades between the two rims.
    half_t = math.pi * radius / n_paddle * 0.4
    out0, out1 = -0.03, 0.2
    for i in range(n_paddle):
        a = 2 * math.pi * i / n_paddle
        tx, tz = -math.sin(a), math.cos(a)
        ox, oz = math.cos(a), math.sin(a)
        cx, cz = x + ox * radius, radius + oz * radius

        def pt(sy, dt, do, cx=cx, cz=cz, tx=tx, tz=tz, ox=ox, oz=oz):
            return (
                cx + tx * dt + ox * do,
                y + sy,
                cz + tz * dt + oz * do,
            )

        pA, pB, pC, pD = (
            pt(-width / 2, half_t, out0),
            pt(-width / 2, -half_t, out0),
            pt(-width / 2, -half_t, out1),
            pt(-width / 2, half_t, out1),
        )
        pA2, pB2, pC2, pD2 = (
            pt(width / 2, half_t, out0),
            pt(width / 2, -half_t, out0),
            pt(width / 2, -half_t, out1),
            pt(width / 2, half_t, out1),
        )
        g.quad(pA, pB, pC, pD, "Planks", color=TIMBER_TINTS[2])
        g.quad(pD2, pC2, pB2, pA2, "Planks", color=TIMBER_TINTS[2])
        g.quad(pD, pC, pC2, pD2, "Planks", color=TIMBER_TINTS[2], occlude=False)
    k.tube(
        g,
        (x, y + width / 2 - 0.05, radius),
        (x, y + width / 2 + 0.6, radius),
        0.13,
        "Timber",
        8,
        color=TIMBER_TINTS[0],
    )


def watermill(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Water mill: miller's house (stone or half-timbered), an undershot wheel mounted against the -Y front wall near one corner (the water side, matching the kit's front convention), a mill race below.

    ``length``/``depth`` size the house alone (world X / world Y); the wheel projects further
    towards -Y (see the ``wheel_offset`` metric). Deviation from a literal reading of "pignon"
    (gable): a true gable-end wheel would need the ridge along world Y, giving a ~7 x 11 footprint
    instead of the requested 11 x 7 (length along X); this build keeps the requested footprint and
    the kit's universal -Y-front convention, and mounts the wheel at a front corner instead (see
    the wip note and final report).
    """
    ridge = length or rng.uniform(10.0, 12.0)  # world X (front width)
    gw = depth or rng.uniform(6.0, 7.0)  # world Y (front-to-back depth)
    stone = rng.random() < 0.5
    roof = "RoofTile" if stone else pick(rng, ["Thatch", "RoofFlat"])
    st = Style(
        wall="Rubble" if stone else "Plaster",
        frame=None if stone else pick(rng, ["rural", "close"]),
        roof=roof,
        shape="gable",
        pitch=rng.uniform(44, 50),
        floors=1,
        shutters=0.5,
        chimneys=1,
        wall_tint=pick(rng, PLASTER_TINTS),
        timber_tint=pick(rng, TIMBER_TINTS),
        roof_tint=pick(rng, ROOF_TINTS[roof]),
        stone_tint=pick(rng, STONE_TINTS),
        extras={"vent": True},
        ruined=ruined,
    )
    wheel_r = 1.5
    info = house(g, rng, ridge, gw, st, detail)
    if not ruined:
        wx = -ridge / 2 + gw * 0.9
        wy = -gw / 2 - wheel_r - 0.15
        _mill_wheel(g, rng, wx, wy, wheel_r, detail)
        k.box(g, (wx, wy - 1.0, -0.4), (0.9, 2.4, 0.8), "Planks", color=TIMBER_TINTS[1])
    out = {"height": info["height"], "length": info["length"], "depth": info["depth"]}
    if not ruined:
        out["wheel_offset"] = wheel_r + 0.3
    return out


def tent(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Ridge tent: canvas on a ridge pole and two masts, guy ropes and pegs, front flaps tied back; undyed, ochre or striped."""
    L = length or rng.uniform(3.6, 4.4)
    D = depth or rng.uniform(2.6, 3.2)
    H = rng.uniform(2.3, 2.7)
    variant = rng.randrange(3)
    tint = [(0.92, 0.87, 0.76), (0.82, 0.55, 0.28), (0.85, 0.8, 0.7)][variant]
    stripe = variant == 2
    strips = 8 if detail == "high" else 5
    for i in range(strips):
        x0 = -L / 2 + L * i / strips
        x1 = -L / 2 + L * (i + 1) / strips
        col = (0.72, 0.14, 0.12) if stripe and i % 2 == 0 else tint
        k.prism_x(g, x0, x1, 0.0, 0.0, D / 2, H, "Canvas", color=col)
    k.beam(
        g,
        (-L / 2 - 0.15, 0.0, H),
        (L / 2 + 0.15, 0.0, H),
        0.09,
        0.09,
        "Timber",
        (0, 0, 1),
        color=TIMBER_TINTS[0],
    )
    for sx in (-1, 1):
        k.beam(
            g,
            (sx * (L / 2 - 0.05), 0.0, H / 2),
            (sx * (L / 2 - 0.05), 0.0, H + 0.12),
            0.08,
            0.08,
            "Timber",
            (sx, 0, 0),
            color=TIMBER_TINTS[0],
        )
    for sy in (-1, 1):
        p0 = (-L / 2, sy * D / 2, 0.0)
        p1 = (-L / 2 - 0.5, sy * (D / 2 + 0.15), 0.05)
        p2 = (-L / 2 - 0.3, sy * (D / 2 - 0.05), H * 0.55)
        g.poly([p0, p1, p2], "Canvas", color=tint)
    for sx in (-1, 1):
        for sy in (-1, 1):
            base = (sx * L * 0.4, sy * (D / 2 + 0.05), H * 0.35)
            peg = (sx * L * 0.4 + sx * 0.7, sy * (D / 2 + 0.9), 0.0)
            k.tube(g, base, peg, 0.015, "Timber", 4, color=(0.5, 0.42, 0.3))
            k.box(g, peg, (0.05, 0.05, 0.18), "Timber", color=TIMBER_TINTS[3])
    return {"height": H, "length": L, "depth": D + 1.9}


def pavilion(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Round knight's pavilion: cylindrical canvas wall, conical roof, scalloped valance, mast and banner; undyed, red/white or blue/white stripes."""
    R = (length or rng.uniform(5.2, 6.0)) / 2
    wall_h = rng.uniform(1.9, 2.3)
    roof_h = rng.uniform(2.4, 2.9)
    variant = rng.randrange(3)
    sides = 16 if detail == "high" else 10
    if variant == 0:
        tint_a = tint_b = (0.9, 0.85, 0.74)
    elif variant == 1:
        tint_a, tint_b = (0.85, 0.12, 0.1), (0.93, 0.9, 0.84)
    else:
        tint_a, tint_b = (0.1, 0.2, 0.55), (0.93, 0.9, 0.84)
    for i in range(sides):
        a0, a1 = 2 * math.pi * i / sides, 2 * math.pi * (i + 1) / sides
        col = tint_a if i % 2 == 0 else tint_b
        p0 = (math.cos(a0) * R, math.sin(a0) * R, 0.0)
        p1 = (math.cos(a1) * R, math.sin(a1) * R, 0.0)
        p2 = (math.cos(a1) * R, math.sin(a1) * R, wall_h)
        p3 = (math.cos(a0) * R, math.sin(a0) * R, wall_h)
        g.poly([p0, p1, p2, p3], "Canvas", color=col)
    apex = (0.0, 0.0, wall_h + roof_h)
    for i in range(sides):
        a0, a1 = 2 * math.pi * i / sides, 2 * math.pi * (i + 1) / sides
        col = tint_a if i % 2 == 0 else tint_b
        p0 = (math.cos(a0) * (R + 0.08), math.sin(a0) * (R + 0.08), wall_h)
        p1 = (math.cos(a1) * (R + 0.08), math.sin(a1) * (R + 0.08), wall_h)
        g.poly([p0, p1, apex], "Canvas", occlude=False, color=col)
    if detail == "high":
        for i in range(sides):
            a0, a1 = 2 * math.pi * i / sides, 2 * math.pi * (i + 1) / sides
            a = (a0 + a1) / 2
            dip = (math.cos(a) * (R + 0.05), math.sin(a) * (R + 0.05), wall_h - 0.25)
            p0 = (math.cos(a0) * (R + 0.05), math.sin(a0) * (R + 0.05), wall_h)
            p1 = (math.cos(a1) * (R + 0.05), math.sin(a1) * (R + 0.05), wall_h)
            g.poly([p0, p1, dip], "Canvas", color=tint_a)
    k.beam(
        g,
        (0, 0, 0),
        (0, 0, wall_h + roof_h + 0.6),
        0.09,
        0.09,
        "Timber",
        (1, 0, 0),
        color=TIMBER_TINTS[0],
    )
    g.quad(
        (0.02, 0.0, wall_h + roof_h + 0.6),
        (0.02, 0.0, wall_h + roof_h + 0.3),
        (0.55, 0.0, wall_h + roof_h + 0.35),
        (0.4, 0.0, wall_h + roof_h + 0.55),
        "Banner",
        occlude=False,
    )
    return {"height": wall_h + roof_h + 0.6, "length": 2 * R, "depth": 2 * R}


def haystack(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Round beehive rick, rectangular thatched gerbier, or a small round stack."""
    variant = rng.randrange(3)
    hay = (0.86, 0.72, 0.32)
    sides = 12 if detail == "high" else 8
    if variant == 0:
        R = rng.uniform(1.9, 2.2)
        H = rng.uniform(3.6, 4.2)
        bands = 5
        r_prev, z = R, 0.0
        for i in range(bands):
            t = (i + 1) / bands
            r_next = R * (1.0 - t) ** 1.3
            h = H / bands
            k.cylinder(
                g,
                (0, 0, z),
                r_prev,
                h,
                "Thatch",
                sides=sides,
                radius_top=max(r_next, 0.05),
                top=(i == bands - 1),
                color=hay,
            )
            r_prev, z = r_next, z + h
        k.tube(
            g,
            (0, 0, z - 0.2),
            (0, 0, z + 0.6),
            0.05,
            "Timber",
            6,
            color=TIMBER_TINTS[0],
        )
        return {"height": z + 0.6, "length": 2 * R, "depth": 2 * R}
    if variant == 1:
        L = length or rng.uniform(3.5, 4.5)
        D = depth or rng.uniform(2.2, 2.8)
        wall_h = rng.uniform(2.2, 2.8)
        k.box(g, (0, 0, wall_h / 2), (L, D, wall_h), "Thatch", color=hay)
        st = Style(roof="Thatch", pitch=48, roof_tint=hay)
        zr = roof_gable(g, L, D, wall_h, st, detail, overhang=0.15, gable_over=0.1)
        return {"height": zr, "length": L, "depth": D}
    R = rng.uniform(1.1, 1.4)
    H = rng.uniform(1.6, 2.0)
    k.cylinder(
        g, (0, 0, 0), R, H * 0.6, "Thatch", sides=sides, radius_top=R * 0.55, color=hay
    )
    k.cylinder(
        g,
        (0, 0, H * 0.6),
        R * 0.55,
        H * 0.4,
        "Thatch",
        sides=sides,
        radius_top=0.05,
        color=hay,
    )
    return {"height": H, "length": 2 * R, "depth": 2 * R}


def wagon(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Four-wheeled baggage wagon: canvas tilt on arcs, or open loaded with barrels or crates and sacks; timon resting on the ground towards -X."""
    bed_l, bed_w, bed_z = 3.0, 1.5, 1.0
    x0 = 0.3
    wood = pick(rng, TIMBER_TINTS)
    k.box(g, (x0, 0.0, bed_z), (bed_l, bed_w, 0.09), "Planks", color=wood, bottom=True)
    for sy in (-1, 1):
        k.box(
            g,
            (x0, sy * (bed_w / 2 - 0.03), bed_z + 0.24),
            (bed_l, 0.06, 0.4),
            "Planks",
            color=wood,
        )
        k.beam(
            g,
            (x0 + bed_l / 2 - 0.2, sy * 0.5, bed_z - 0.1),
            (x0 - bed_l / 2 - 2.3, sy * 0.4, 0.06),
            0.1,
            0.1,
            "Timber",
            (0.0, 0.0, 1.0),
            color=TIMBER_TINTS[1],
        )
    for sx, sy in ((-1, -1), (-1, 1), (1, -1), (1, 1)):
        _wheel(
            g,
            x0 + sx * (bed_l / 2 - 0.35),
            sy * (bed_w / 2 + 0.14),
            0.66 if sx > 0 else 0.5,
            0.1,
            detail,
        )
    variant = rng.randrange(3)
    top = bed_z + 0.05
    if variant == 0:
        n_hoop = 4
        cloth = pick(rng, CLOTH_TINTS)
        xs = [-bed_l / 2 + 0.3 + bed_l * i / (n_hoop - 1) for i in range(n_hoop)]
        for hx in xs:
            k.beam(
                g,
                (x0 + hx, -bed_w / 2, top + 0.05),
                (x0 + hx, 0.0, top + 1.1),
                0.04,
                0.04,
                "Timber",
                (1, 0, 0),
                color=TIMBER_TINTS[0],
            )
            k.beam(
                g,
                (x0 + hx, 0.0, top + 1.1),
                (x0 + hx, bed_w / 2, top + 0.05),
                0.04,
                0.04,
                "Timber",
                (1, 0, 0),
                color=TIMBER_TINTS[0],
            )
        for i in range(n_hoop - 1):
            x_a, x_b = xs[i], xs[i + 1]
            for sign in (-1, 1):
                pts = [
                    (x0 + x_a, sign * bed_w / 2 * 0.98, top + 0.05),
                    (x0 + x_b, sign * bed_w / 2 * 0.98, top + 0.05),
                    (x0 + x_b, sign * bed_w * 0.15, top + 1.08),
                    (x0 + x_a, sign * bed_w * 0.15, top + 1.08),
                ]
                if sign > 0:
                    pts.reverse()
                g.poly(pts, "Canvas", color=cloth)
    elif variant == 1:
        for bx in (-0.9, -0.2, 0.5, 1.2):
            _barrel(g, x0 + bx, rng.uniform(-0.25, 0.25), top, 0.3, 0.85, detail)
    else:
        for i in range(4):
            _crate(
                g,
                (x0 - 1.0 + i * 0.7, rng.uniform(-0.3, 0.3), top + 0.22),
                (0.55, 0.45, 0.44),
                rng.uniform(-0.2, 0.2),
            )
        for i in range(3):
            _sack(g, rng, x0 - 0.6 + i * 0.6, bed_w / 2 - 0.35, top)
    return {"height": bed_z + 1.3, "length": bed_l + 2.6, "depth": bed_w + 0.5}


def campfire(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Camp fire: ring of stones and logs, with a tripod and cauldron or a spit."""
    R = 0.8
    n = 8 if detail == "high" else 6
    for i in range(n):
        a = 2 * math.pi * i / n + rng.uniform(-0.05, 0.05)
        r = R * rng.uniform(0.92, 1.05)
        k.cylinder(
            g,
            (math.cos(a) * r, math.sin(a) * r, 0.0),
            0.13,
            0.17,
            "Rubble",
            sides=5,
            color=pick(rng, STONE_TINTS),
        )
    for _ in range(4):
        a = rng.uniform(0, 2 * math.pi)
        length_ = rng.uniform(0.5, 0.7)
        p0 = (math.cos(a) * 0.1, math.sin(a) * 0.1, 0.05)
        p1 = (math.cos(a) * length_, math.sin(a) * length_, 0.1)
        k.tube(g, p0, p1, 0.045, "Timber", 5, color=pick(rng, TIMBER_TINTS))
    if rng.randrange(2) == 0:
        for a in (0.6, 2.6, 4.6):
            k.beam(
                g,
                (math.cos(a) * 0.6, math.sin(a) * 0.6, 0.0),
                (0.0, 0.0, 1.1),
                0.05,
                0.05,
                "Timber",
                (math.cos(a), math.sin(a), 0),
                color=TIMBER_TINTS[0],
            )
        k.tube(g, (0, 0, 0.55), (0, 0, 0.85), 0.24, "Iron", 8, caps=False)
        k.cylinder(g, (0, 0, 0.35), 0.24, 0.2, "Iron", sides=8, radius_top=0.2)
    else:
        for sx in (-1, 1):
            k.beam(
                g,
                (sx * 0.65, 0.0, 0.0),
                (sx * 0.15, 0.0, 0.55),
                0.05,
                0.05,
                "Timber",
                (0, 1, 0),
                color=TIMBER_TINTS[0],
            )
        k.tube(g, (-0.6, 0.0, 0.5), (0.6, 0.0, 0.5), 0.02, "Iron", 6, caps=False)
    return {"height": 1.1, "length": 1.6, "depth": 1.6}


def wall_run(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Dry-stone churchyard wall with coping, a 10 m run along X to butt end to end."""
    L = length or 10.0
    D = depth or 0.6
    H = 1.3
    tint = pick(rng, STONE_TINTS)
    k.box(g, (0, 0, H / 2), (L, D, H), "Rubble", color=tint)
    if detail == "high":
        n = max(1, int(L / 0.9))
        for i in range(n):
            x = -L / 2 + 0.45 + i * (L / n)
            k.box(
                g,
                (x, 0.0, H + 0.06),
                (0.55, D + 0.1, 0.12),
                "Ashlar",
                rng.uniform(-0.04, 0.04),
                color=tint,
            )
    else:
        k.box(g, (0, 0, H + 0.06), (L, D + 0.08, 0.12), "Ashlar", color=tint)
    return {"height": H + 0.12, "length": L, "depth": D}


def lychgate(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Covered lychgate: four corner posts, wall plates, a low gate, a small tiled roof."""
    L = length or 3.0
    D = depth or 2.5
    H = 2.4
    wood = pick(rng, TIMBER_TINTS[:2])
    for sx in (-1, 1):
        for sy in (-1, 1):
            k.box(
                g,
                (sx * (L / 2 - 0.15), sy * (D / 2 - 0.15), H / 2),
                (0.18, 0.18, H),
                "Timber",
                grain=True,
                color=wood,
            )
    for sy in (-1, 1):
        k.beam(
            g,
            (-L / 2 + 0.15, sy * (D / 2 - 0.15), H - 0.1),
            (L / 2 - 0.15, sy * (D / 2 - 0.15), H - 0.1),
            0.14,
            0.14,
            "Timber",
            (0, 1 if sy > 0 else -1, 0),
            color=wood,
        )
    k.beam(
        g,
        (0, -D / 2 + 0.15, H - 0.1),
        (0, D / 2 - 0.15, H - 0.1),
        0.14,
        0.14,
        "Timber",
        (0, 0, 1),
        color=wood,
    )
    for h in (0.5, 0.95):
        k.beam(
            g,
            (-L / 2 + 0.15, -D / 2 + 0.15, h),
            (L / 2 - 0.15, -D / 2 + 0.15, h),
            0.06,
            0.06,
            "Timber",
            (0, -1, 0),
            color=wood,
        )
    st = Style(roof="RoofTile", pitch=42, roof_tint=pick(rng, ROOF_TINTS["RoofTile"]))
    zr = roof_gable(g, L, D, H, st, detail, overhang=0.3, gable_over=0.2)
    return {"height": zr, "length": L, "depth": D}


def graves(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Scatter of graves in a churchyard patch: wooden and stone crosses, slabs, turf mounds."""
    L = length or 6.0
    D = depth or 4.0
    count = rng.randint(6, 9)
    placed = []
    tries = 0
    while len(placed) < count and tries < count * 6:
        tries += 1
        x = rng.uniform(-L / 2 + 0.5, L / 2 - 0.5)
        y = rng.uniform(-D / 2 + 0.5, D / 2 - 0.5)
        if all((x - px) ** 2 + (y - py) ** 2 > 0.7**2 for px, py in placed):
            placed.append((x, y))
            kind = rng.randrange(4)
            yaw = rng.uniform(-0.3, 0.3)
            if kind == 0:
                d = (math.cos(yaw), math.sin(yaw), 0.0)
                k.beam(
                    g,
                    (x, y, 0.0),
                    (x, y, 0.75),
                    0.07,
                    0.05,
                    "Timber",
                    d,
                    color=TIMBER_TINTS[1],
                )
                k.beam(
                    g,
                    (x - 0.22 * d[0], y - 0.22 * d[1], 0.5),
                    (x + 0.22 * d[0], y + 0.22 * d[1], 0.5),
                    0.05,
                    0.04,
                    "Timber",
                    (0, 0, 1),
                    color=TIMBER_TINTS[1],
                )
            elif kind == 1:
                k.box(
                    g,
                    (x, y, 0.35),
                    (0.14, 0.35, 0.7),
                    "Ashlar",
                    yaw,
                    color=pick(rng, STONE_TINTS),
                )
            elif kind == 2:
                k.box(
                    g,
                    (x, y, 0.06),
                    (0.65, 1.8, 0.12),
                    "Ashlar",
                    yaw,
                    color=pick(rng, STONE_TINTS),
                )
            else:
                k.box(
                    g,
                    (x, y, 0.1),
                    (0.7, 1.8, 0.2),
                    "Rubble",
                    yaw,
                    color=(0.42, 0.4, 0.28),
                )
    return {"height": 0.8, "length": L, "depth": D}


def vine_row(g, rng, detail="high", length=None, depth=None, ruined=False):
    """Row of vines on stakes and a wire, 10 m along X: leafy with grape bunches, or bare (winter) with just the stakes, ceps and wire."""
    L = length or 10.0
    H = 1.4
    n = max(3, int(L / 1.6))
    leafy = rng.randrange(2) == 0
    wood = TIMBER_TINTS[3]
    for i in range(n):
        x = -L / 2 + (i + 0.5) * (L / n)
        jitter = rng.uniform(-0.05, 0.05)
        k.beam(
            g,
            (x + jitter, 0.0, 0.0),
            (x + jitter, 0.0, H),
            0.045,
            0.045,
            "Timber",
            (1, 0, 0),
            color=wood,
        )
        stem_h = rng.uniform(0.25, 0.5)
        twist = rng.uniform(-0.3, 0.3)
        k.tube(
            g,
            (x, 0.02, 0.0),
            (x + twist, -0.05, stem_h),
            0.03,
            "Timber",
            5,
            color=(0.32, 0.24, 0.14),
        )
        k.tube(
            g,
            (x + twist, -0.05, stem_h),
            (x + twist * 0.4, 0.0, H * 0.7),
            0.02,
            "Timber",
            5,
            color=(0.32, 0.24, 0.14),
        )
    for row_h in (0.55, 1.05, H - 0.05):
        k.tube(
            g, (-L / 2, 0.0, row_h), (L / 2, 0.0, row_h), 0.008, "Iron", 4, caps=False
        )
    if leafy:
        for i in range(n):
            x0 = -L / 2 + i * (L / n)
            x1 = -L / 2 + (i + 1) * (L / n)
            z0 = 0.5 + rng.uniform(-0.06, 0.06)
            z1 = H + 0.15 + rng.uniform(-0.06, 0.06)
            for sign in (-1, 1):
                pts = [
                    (x0, sign * 0.02, z0),
                    (x1, sign * 0.02, z0),
                    (x1, sign * 0.32, z1),
                    (x0, sign * 0.32, z1),
                ]
                if sign > 0:
                    pts.reverse()
                g.poly(pts, "Canvas", color=(0.5, 1.0, 0.3))
            if rng.random() < 0.4:
                k.cylinder(
                    g,
                    (rng.uniform(x0, x1), rng.uniform(-0.1, 0.1), z0 + 0.1),
                    0.05,
                    0.12,
                    "Planks",
                    sides=5,
                    color=(0.28, 0.14, 0.32),
                )
    return {"height": H, "length": L, "depth": 0.8}


RECIPES = {
    "cottage": cottage,
    "longere": longere,
    "timber": timber_house,
    "townhouse": town_house,
    "stonehouse": stone_house,
    "barn": barn,
    "church": church,
    "cathedral": cathedral,
    "manor": manor,
    "hall": market_hall,
    "well": well,
    "windmill": windmill,
    "stall": market_stall,
    "cart": cart,
    "barrels": barrels,
    "woodpile": woodpile,
    "bridge_stone_bay": bridge_stone_bay,
    "bridge_stone_end": bridge_stone_end,
    "bridge_wood_bay": bridge_wood_bay,
    "bridge_wood_end": bridge_wood_end,
    "watermill": watermill,
    "tent": tent,
    "pavilion": pavilion,
    "haystack": haystack,
    "wagon": wagon,
    "campfire": campfire,
    "wall_run": wall_run,
    "lychgate": lychgate,
    "graves": graves,
    "vine_row": vine_row,
}


def build(
    kind: str, seed: int, detail: str = "high", ruined: bool = False, **dims
) -> tuple[Geometry, dict]:
    """Build one building of ``kind`` with a deterministic ``seed``."""
    rng = random.Random(seed)
    g = Geometry()
    g.uv_shift = (rng.uniform(0, 4), rng.uniform(0, 4))
    if ruined:
        g.char = 0.85
    g.seed_shift = rng.uniform(0, 1000)
    info = RECIPES[kind](g, rng, detail=detail, ruined=ruined, **dims)
    sag = SAG.get(kind, 0.0) * rng.uniform(0.5, 1.2)
    if sag and not ruined and "eave" in info:
        _settle(g, info, sag, rng)
    info["triangles"] = g.triangle_count()
    return g, info


# Settling of old timber buildings: ridge sag (m) per kind; stone buildings barely move.
SAG = {
    "cottage": 0.16,
    "longere": 0.14,
    "timber": 0.14,
    "townhouse": 0.1,
    "barn": 0.22,
    "stonehouse": 0.04,
}


def _settle(g: Geometry, info: dict, sag: float, rng: random.Random) -> None:
    """Sag the roof between the gables and lean the walls a little (irregular, lived-in look)."""
    eave, ridge = info["eave"], info["height"]
    along_y = info.get("ridge_axis") == "y"
    half = (info["depth"] if along_y else info["length"]) / 2 + 0.6
    lean_x, lean_y = rng.uniform(-0.006, 0.006), rng.uniform(-0.006, 0.006)

    def fn(p):
        x, y, z = p
        if z > 0.0:
            x += lean_x * z
            y += lean_y * z
        if z > eave - 0.5:
            s = min(max((z - eave + 0.5) / max(ridge - eave + 0.5, 0.1), 0.0), 1.0)
            a = (p[1] if along_y else p[0]) / half
            z -= sag * s * max(1.0 - a * a, 0.0)
        return (x, y, z)

    g.warp(fn)
