"""Lot CV1 settlement models: settlements that grow with their level (campaign map).

Run headless (after ``settlements.py``, whose helpers and export are reused):
    blender --background --python settlements_cv1.py -- <out_dir> [name ...]

* ``bourg_a`` / ``bourg_b``: open market town without walls, market hall on its square;
* ``cite_a`` / ``cite_b``: great walled city with cathedral, market hall, castle and suburbs
  outside several gates;
* ``windmill_body`` / ``windmill_sails``: post mill, sails apart (hub at the origin, in the
  glTF XY plane) so that Godot turns them around Z in a shader.

Kept apart from ``settlements.py`` (lot BR1 rewrites its buildings under the same GLB names).
A line ``MODEL <name> <triangles>`` is printed per model and ``OK`` at the end.
"""

import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import models as m  # noqa: E402
import settlements as s  # noqa: E402

F = m.FOUNDATION


def market_hall(x, y, angle, length=0.36, width=0.2, height=0.07):
    """Covered market (halle): timber posts under a large tiled roof."""
    parts = []
    ca, sa = math.cos(angle), math.sin(angle)
    for i in range(4):
        for j in (-1, 1):
            dx = (i / 3 - 0.5) * length * 0.9
            dy = j * width * 0.42
            parts.append(
                m.box(
                    (0.018, 0.018, height + F),
                    (x + dx * ca - dy * sa, y + dx * sa + dy * ca, (height - F) / 2),
                    "Wood",
                )
            )
    parts.append(
        m.hip_roof(
            length * 1.08,
            width * 1.15,
            width * 0.6,
            (x, y, height),
            "TileOld",
            angle=angle,
        )
    )
    return parts


def build_bourg(seed, count, spread, church_angle, hall_angle):
    """Open market town: church, market square and hall, some stone houses, no walls."""
    rng = random.Random(seed)
    parts = m.ground_patch(spread * 0.75, "Dirt", 12)
    parts += m.church(0.28, 0.22, church_angle, 1.0, roof="Slate")
    parts += market_hall(-0.08, -0.1, hall_angle)
    parts += m.scatter_houses(
        rng,
        count,
        spread,
        [(0.28, 0.22, 0.34), (-0.08, -0.1, 0.3)],
        1.3,
        roofs=("Tile", "TileOld", "Thatch", "Slate"),
        walls=("Plaster", "Timber", "Stone"),
        size_scale=1.05,
    )
    for dx, dy, angle in ((-0.95, 0.3, 0.4), (0.9, -0.5, 2.1), (0.2, 1.0, 1.2)):
        parts += m.house(
            (dx, dy, 0.0), (0.28, 0.15, 0.11), "Thatch", "Wood", angle=angle
        )
    parts += s.field_strips(-0.4, -1.15, 0.3, 6, 0.6, 0.07)
    parts += s.banner(0.28 - 0.25, 0.22, 0.42 + 0.3, 0.2)
    return parts


def build_cite(seed, sides, radius, cathedral_angle, gates):
    """Great city: walls, cathedral, market hall, castle, suburbs before several gates."""
    rng = random.Random(seed)
    points = m.ring_points(radius, sides, rng, 0.08)
    parts = m.ground_patch(radius * 0.97)
    parts += m.town_walls(points, 0.24, 0.08, gates=gates)
    parts += m.cathedral_building(-0.1, 0.1, cathedral_angle, 0.6)
    parts += market_hall(0.45, -0.25, 0.4, 0.42, 0.22, 0.08)
    cx, cy = points[sides // 2]
    castle_x, castle_y = cx * 0.62, cy * 0.62
    parts += m.castle_keep(castle_x, castle_y, 0.75)
    parts += s.banner(castle_x + 0.03, castle_y + 0.02, 0.75 * 0.72 + 0.02)
    parts += m.scatter_houses(
        rng,
        64,
        radius * 0.92,
        [(-0.1, 0.1, 0.62), (0.45, -0.25, 0.3), (castle_x, castle_y, 0.42)],
        1.25,
        inside=[(x * 0.92, y * 0.92) for x, y in points],
        size_scale=0.95,
    )
    # Faubourgs hors les murs devant chaque porte, le long de la route.
    for gate in gates:
        gx, gy = points[gate]
        base = math.atan2(gy, gx)
        for k in range(7):
            t = 1.1 + 0.11 * k
            ang = base + rng.uniform(-0.2, 0.2)
            side = rng.choice((-1, 1)) * 0.09
            hx = t * radius * math.cos(ang) - side * math.sin(ang)
            hy = t * radius * math.sin(ang) + side * math.cos(ang)
            parts += m.house(
                (hx, hy, 0.0),
                (0.17, 0.11, 0.1),
                rng.choice(("Thatch", "TileOld", "Tile")),
                rng.choice(("Plaster", "Timber")),
                angle=ang + math.pi / 2,
            )
    return parts


def build_windmill_body():
    """Post mill body on its trestle; the sails hub sits at (0, -0.07, 0.3)."""
    parts = [
        m.box((0.06, 0.06, 0.012), (0, 0, 0.0), "Wood", rotation=(0, 0, 0.785)),
        m.cylinder(0.012, 0.16 + F, (0, 0, (0.16 - F) / 2), "Wood", 6),
    ]
    for ang in (0.785, 2.356, 3.927, 5.498):
        parts.append(
            m.box(
                (0.1, 0.01, 0.01),
                (0.035 * math.cos(ang), 0.035 * math.sin(ang), 0.05),
                "Wood",
                rotation=(0, 0.6, ang),
            )
        )
    parts.append(m.box((0.09, 0.11, 0.14), (0, 0, 0.23), "Timber"))
    parts.append(
        m.gable_roof(0.09, 0.11, 0.06, (0, 0, 0.30), "Wood", angle=math.pi / 2)
    )
    parts.append(
        m.cylinder(0.01, 0.05, (0, -0.07, 0.3), "Wood", 6, rotation=(1.5708, 0, 0))
    )
    return parts


def build_windmill_sails():
    """Four lattice sails around the hub at the origin, in the XZ plane (faces -Y)."""
    parts = [m.cylinder(0.014, 0.02, (0, 0, 0), "Wood", 6, rotation=(1.5708, 0, 0))]
    for k in range(4):
        ang = k * math.pi / 2 + 0.3
        ca, sa = math.cos(ang), math.sin(ang)
        parts.append(
            m.box(
                (0.3, 0.008, 0.008),
                (0.15 * ca, 0, 0.15 * sa),
                "Wood",
                rotation=(0, -ang, 0),
            )
        )
        ox, oz = 0.17 * ca - 0.03 * sa, 0.17 * sa + 0.03 * ca
        parts.append(
            m.box(
                (0.24, 0.004, 0.05), (ox, -0.004, oz), "Canvas", rotation=(0, -ang, 0)
            )
        )
    return parts


MODELS = {
    "bourg_a": lambda: build_bourg(501, 24, 0.95, 0.2, 0.5),
    "bourg_b": lambda: build_bourg(733, 20, 0.85, -1.1, -0.3),
    "cite_a": lambda: build_cite(5151, 12, 1.5, 0.25, (0, 4, 8)),
    "cite_b": lambda: build_cite(6262, 11, 1.4, -0.2, (1, 5, 9)),
    "windmill_body": build_windmill_body,
    "windmill_sails": build_windmill_sails,
}


def main() -> None:
    """Parse ``-- <out_dir> [names...]`` and export the requested models."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(args[0]) if args else Path.cwd() / "settlements"
    names = args[1:] or list(MODELS)
    s.MODELS.update(MODELS)
    for name in names:
        print(f"MODEL {name} {s.export_model(name, out_dir)}")
    print("OK")


if __name__ == "__main__":
    main()
