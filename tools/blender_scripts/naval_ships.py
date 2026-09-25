"""Naval battle ships (lot NV1): cog, nef, galley and barge at full scale, glTF binaries.

Run headless:
    blender -b --python tools/blender_scripts/naval_ships.py -- game/assets/models/naval [name ...]

Metric models (1 unit = 1 m, waterline at z = 0), built Z up and X forward in Blender (Y up,
+X forward in Godot). Dimensions follow ``data/naval/ships/*.json``: length, beam, freeboard
(main deck height) and castle heights, so that the crews of the battle stand on the decks
the simulation assumes. Detail over the campaign fleet (``campaign_fleet.py``): clinker
strakes, deck planking, crenellated castles, bulwark rail hung with pavises, shrouds and
stays, rudder, anchor; galleys and barges get oars and a spur (galley).

Materials (surfaces in Godot, replaced by name): ``Sail`` (subdivided sail, UVs over the whole
sail, top-left = (0, 0): ``campaign_sail.gdshader`` gives faction colour and arms),
``Banner`` (castle parapets, tinted to the faction), ``Shield`` (pavises along the rail,
tinted), ``Oar`` (oars: UV.x = pivot x in metres, UV.y = side, ``naval_oar.gdshader`` rows
them), ``Hull``, ``Wood``, ``Deck``, ``Rope``, ``Iron``.
"""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import models  # noqa: E402
from campaign_fleet import clinker_strakes, lofted_hull, rod  # noqa: E402
from models import box, cylinder, mesh_object  # noqa: E402

models.PALETTE["Sail"] = ((0.66, 0.61, 0.49), 0.9, 0.0)
models.PALETTE["Hull"] = ((0.15, 0.10, 0.06), 0.85, 0.0)
models.PALETTE["Deck"] = ((0.36, 0.27, 0.17), 0.9, 0.0)
models.PALETTE["Rope"] = ((0.30, 0.24, 0.15), 0.95, 0.0)
models.PALETTE["Shield"] = ((0.62, 0.1, 0.08), 0.8, 0.0)
models.PALETTE["Oar"] = ((0.33, 0.25, 0.15), 0.85, 0.0)
models.PALETTE["Iron"] = ((0.2, 0.2, 0.21), 0.5, 0.7)


def uv_object(obj, uv_of):
    """Gives ``obj`` a UV layer computed per vertex by ``uv_of(vertex)``."""
    layer = obj.data.uv_layers.new(name="UVMap")
    for poly in obj.data.polygons:
        for loop_index in poly.loop_indices:
            vertex = obj.data.vertices[obj.data.loops[loop_index].vertex_index].co
            layer.data[loop_index].uv = uv_of(vertex)
    return obj


def grid_sail(width, top, bottom, x, rows=8, cols=8, taper=0.0, name="sail"):
    """Square (or tapering) sail in the YZ plane; UV (0, 0) top-left in Godot."""
    verts, uvs = [], []
    for r in range(rows + 1):
        v = r / rows
        z = top - (top - bottom) * v
        half = width / 2 * (1.0 - taper * (1.0 - v))
        for c in range(cols + 1):
            u = c / cols
            verts.append((x, -half + 2 * half * u, z))
            uvs.append((u, 1.0 - v))
    faces = []
    for r in range(rows):
        for c in range(cols):
            a = r * (cols + 1) + c
            faces.append((a, a + cols + 1, a + cols + 2, a + 1))
    obj = mesh_object(name, verts, faces, "Sail")
    layer = obj.data.uv_layers.new(name="UVMap")
    for poly in obj.data.polygons:
        for loop_index in poly.loop_indices:
            layer.data[loop_index].uv = uvs[obj.data.loops[loop_index].vertex_index]
    return obj


def lateen_sail(x0, x1, z0, z1, foot_x, foot_z, rows=8, cols=8):
    """Triangular lateen sail under a slanted yard from (x0, z0) to (x1, z1)."""
    verts, uvs = [], []
    for r in range(rows + 1):
        v = r / rows
        for c in range(cols + 1):
            u = c / cols
            # Along the yard at the top, converging to the clew at the foot.
            top_x = x0 + (x1 - x0) * u
            top_z = z0 + (z1 - z0) * u
            x = top_x + (foot_x - top_x) * v * (1.0 - u * 0.2)
            z = top_z + (foot_z - top_z) * v
            verts.append((x, 0.0, z))
            uvs.append((u, 1.0 - v))
    faces = []
    for r in range(rows):
        for c in range(cols):
            a = r * (cols + 1) + c
            faces.append((a, a + cols + 1, a + cols + 2, a + 1))
    obj = mesh_object("lateen", verts, faces, "Sail")
    layer = obj.data.uv_layers.new(name="UVMap")
    for poly in obj.data.polygons:
        for loop_index in poly.loop_indices:
            layer.data[loop_index].uv = uvs[obj.data.loops[loop_index].vertex_index]
    return obj


def scaled_sections(profile, length, beam, freeboard, draft):
    """Sections ``(x, half_width, top, keel)`` from a unit profile (x in -1..1)."""
    return [
        (x * length / 2, w * beam / 2, freeboard + t, -draft * k)
        for x, w, t, k in profile
    ]


def deck(parts, x0, x1, width, z):
    """Planked deck: a slab and plank seams."""
    parts.append(box((x1 - x0, width, 0.12), ((x0 + x1) / 2, 0, z - 0.06), "Deck"))
    count = max(2, int(width / 0.9))
    for i in range(1, count):
        y = -width / 2 + width * i / count
        parts.append(box((x1 - x0, 0.03, 0.02), ((x0 + x1) / 2, y, z + 0.005), "Wood"))


def bulwark(parts, x0, x1, half_width, z, height=1.0, shields=True, spacing=1.15):
    """Rail along both sides; pavises (shields) hung outboard."""
    for side in (-1, 1):
        parts.append(
            box(
                (x1 - x0, 0.12, height),
                ((x0 + x1) / 2, side * half_width, z + height / 2),
                "Wood",
            )
        )
        parts.append(
            box(
                (x1 - x0, 0.22, 0.08),
                ((x0 + x1) / 2, side * half_width, z + height),
                "Hull",
            )
        )
        if not shields:
            continue
        count = int((x1 - x0) / spacing)
        for i in range(count):
            x = x0 + spacing * (i + 0.5)
            parts.append(
                box(
                    (0.62, 0.06, 0.95),
                    (x, side * (half_width + 0.1), z + height * 0.55),
                    "Shield",
                )
            )


def castle(parts, x, length, width, deck_z, height, parapet=1.1):
    """Castle: timber box on the deck, platform, crenellated parapet (Banner)."""
    top = deck_z + height
    parts.append(box((length, width, height), (x, 0, deck_z + height / 2), "Wood"))
    parts.append(box((length + 0.3, width + 0.3, 0.18), (x, 0, top - 0.09), "Deck"))
    # Posts at the corners.
    for sx in (-1, 1):
        for sy in (-1, 1):
            parts.append(
                box(
                    (0.22, 0.22, height + parapet),
                    (
                        x + sx * length / 2,
                        sy * width / 2,
                        deck_z + (height + parapet) / 2,
                    ),
                    "Hull",
                )
            )
    # Parapet with merlons, painted.
    merlon = 0.55
    for side in (-1, 1):
        parts.append(
            box(
                (length, 0.1, parapet * 0.45),
                (x, side * width / 2, top + parapet * 0.22),
                "Banner",
            )
        )
        count = max(2, int(length / (merlon * 2)))
        for i in range(count):
            mx = x - length / 2 + length * (i + 0.5) / count
            parts.append(
                box(
                    (merlon, 0.12, parapet * 0.55),
                    (mx, side * width / 2, top + parapet * 0.72),
                    "Banner",
                )
            )
    for end in (-1, 1):
        parts.append(
            box(
                (0.1, width, parapet * 0.45),
                (x + end * length / 2, 0, top + parapet * 0.22),
                "Banner",
            )
        )
        count = max(2, int(width / (merlon * 2)))
        for i in range(count):
            my = -width / 2 + width * (i + 0.5) / count
            parts.append(
                box(
                    (0.12, merlon, parapet * 0.55),
                    (x + end * length / 2, my, top + parapet * 0.72),
                    "Banner",
                )
            )
    # Ladder to the platform.
    parts.append(
        box(
            (0.08, 0.7, height * 1.1),
            (x - math.copysign(length / 2 + 0.3, x), 0.9, deck_z + height / 2),
            "Wood",
            rotation=(0, math.copysign(0.3, x), 0),
        )
    )


def mast(parts, x, deck_z, height, yard, yard_z, beam, stays):
    """Mast with top (crow's nest), yard, shrouds and stays."""
    top = deck_z + height
    parts.append(cylinder(0.28, height + 1.0, (x, 0, deck_z + height / 2), "Wood", 10))
    parts.append(cylinder(0.9, 0.9, (x, 0, top - 1.8), "Wood", 12))  # hune
    parts.append(cylinder(0.95, 0.15, (x, 0, top - 2.3), "Hull", 12))
    parts.append(
        rod(
            (x + 0.35, -yard / 2, yard_z), (x + 0.35, yard / 2, yard_z), 0.14, "Wood", 6
        )
    )
    for side in (-1, 1):
        for i, dx in enumerate((-1.6, -0.6, 0.4, 1.4)):
            parts.append(
                rod(
                    (x, 0, top - 2.6),
                    (x + dx, side * beam / 2, deck_z + 0.9),
                    0.035,
                    "Rope",
                )
            )
            if i < 3:
                # Ratlines.
                for k in range(1, 6):
                    t = k / 6
                    a = (
                        x + (dx) * t,
                        side * beam / 2 * t,
                        top - 2.6 - (top - 3.5 - deck_z) * t,
                    )
                    b = (
                        x + (dx + 1.0) * t,
                        side * beam / 2 * t,
                        top - 2.6 - (top - 3.5 - deck_z) * t,
                    )
                    parts.append(rod(a, b, 0.02, "Rope", 3))
    for end in stays:
        parts.append(rod((x, 0, top - 2.4), end, 0.04, "Rope"))


def rudder(parts, x, deck_z, draft):
    """Gouvernail d'étambot et sa barre."""
    parts.append(
        box(
            (1.6, 0.18, deck_z + draft),
            (x, 0, (deck_z - draft) / 2),
            "Hull",
            rotation=(0, math.radians(8), 0),
        )
    )
    parts.append(box((2.2, 0.14, 0.14), (x + 1.6, 0, deck_z + 0.7), "Wood"))  # barre


def anchor(parts, x, side_y, z):
    """Ancre pendue au bordé."""
    parts.append(box((0.12, 0.12, 1.6), (x, side_y, z), "Iron"))
    parts.append(box((1.0, 0.12, 0.12), (x, side_y, z - 0.75), "Iron"))


def oars(parts, x0, x1, count, half_width, rail_z, length):
    """Oars on both sides; UV.x = pivot x, UV.y = 1 (port) / 0 (starboard)."""
    for side in (-1, 1):
        for i in range(count):
            x = x0 + (x1 - x0) * (i + 0.5) / count
            pivot = (x, side * half_width, rail_z)
            blade = (
                x - 1.2,
                side * (half_width + length * 0.8),
                rail_z - length * 0.55,
            )
            loom = rod(
                (x + 0.4, side * (half_width - 1.4), rail_z + 0.5),
                blade,
                0.07,
                "Oar",
                4,
            )
            paddle = box((0.9, 0.05, 0.28), blade, "Oar")
            for part in (loom, paddle):
                uv_object(
                    part, lambda _v, p=pivot, s=side: (p[0], 1.0 if s > 0 else 0.0)
                )
                parts.append(part)


# ----- ships -----------------------------------------------------------------------------

COG_PROFILE = [
    (-1.0, 0.1, 1.2, 0.6),
    (-0.88, 0.55, 0.9, 0.85),
    (-0.6, 0.92, 0.35, 1.0),
    (-0.1, 1.0, 0.0, 1.0),
    (0.35, 0.98, 0.05, 1.0),
    (0.7, 0.78, 0.4, 0.95),
    (0.9, 0.38, 0.9, 0.8),
    (1.04, 0.06, 1.4, 0.5),
]


def build_cog():
    """Cog, 24 m: high flat-sided hull, fore and aft castles, one square sail."""
    length, beam, fb, draft = 24.0, 7.5, 2.8, 2.2
    sections = scaled_sections(COG_PROFILE, length, beam, fb, draft)
    parts = [lofted_hull(sections, ring=10)]
    parts += clinker_strakes(sections, 5)
    deck(parts, -9.5, 9.5, beam - 0.7, fb)
    bulwark(parts, -7.0, 6.5, beam / 2 - 0.35, fb)
    castle(parts, -8.6, 6.0, 6.3, fb, 2.4)
    castle(parts, 9.0, 4.2, 4.6, fb, 2.0)
    rudder(parts, -12.4, fb, draft)
    anchor(parts, 10.5, 2.6, fb - 0.6)
    mast(
        parts,
        0.6,
        fb,
        21.0,
        15.5,
        fb + 17.8,
        beam,
        [(12.6, 0, fb + 1.6), (-11.0, 0, fb + 3.2)],
    )
    parts.append(grid_sail(14.6, fb + 17.6, fb + 5.2, 0.95))
    return parts


NEF_PROFILE = [
    (-1.0, 0.08, 1.4, 0.6),
    (-0.9, 0.5, 1.0, 0.85),
    (-0.65, 0.86, 0.35, 1.0),
    (-0.2, 1.0, 0.0, 1.0),
    (0.3, 1.0, 0.0, 1.0),
    (0.7, 0.8, 0.3, 0.95),
    (0.92, 0.42, 0.8, 0.8),
    (1.03, 0.06, 1.3, 0.55),
]


def build_nef():
    """Nef, 30 m: rounder, taller castles, main and fore masts, bowsprit."""
    length, beam, fb, draft = 30.0, 9.0, 3.4, 2.6
    sections = scaled_sections(NEF_PROFILE, length, beam, fb, draft)
    parts = [lofted_hull(sections, ring=10)]
    parts += clinker_strakes(sections, 6)
    deck(parts, -12.0, 12.0, beam - 0.8, fb)
    bulwark(parts, -8.5, 8.2, beam / 2 - 0.4, fb)
    castle(parts, -10.6, 7.5, 7.8, fb, 3.0)
    castle(parts, 11.2, 5.2, 5.8, fb, 2.4)
    parts.append(
        rod((13.5, 0, fb + 2.0), (20.0, 0, fb + 4.6), 0.25, "Wood", 8)
    )  # beaupré
    rudder(parts, -15.4, fb, draft)
    anchor(parts, 13.0, 3.2, fb - 0.8)
    mast(
        parts,
        1.2,
        fb,
        25.0,
        18.0,
        fb + 21.5,
        beam,
        [(20.0, 0, fb + 4.6), (-13.0, 0, fb + 4.0)],
    )
    parts.append(grid_sail(17.0, fb + 21.3, fb + 6.0, 1.55))
    mast(
        parts,
        8.6,
        fb + 2.4,
        13.0,
        8.0,
        fb + 2.4 + 11.0,
        beam * 0.7,
        [(19.0, 0, fb + 4.4)],
    )
    parts.append(
        grid_sail(7.6, fb + 13.2, fb + 5.8, 8.95, rows=6, cols=6, name="foresail")
    )
    return parts


GALLEY_PROFILE = [
    (-1.0, 0.12, 0.9, 0.35),
    (-0.85, 0.7, 0.45, 0.9),
    (-0.55, 0.96, 0.1, 1.0),
    (0.0, 1.0, 0.0, 1.0),
    (0.5, 0.95, 0.05, 1.0),
    (0.82, 0.62, 0.3, 0.85),
    (0.97, 0.18, 0.55, 0.5),
]


def build_galley():
    """Galley, 40 m: long and low, rowing benches, 25 oars a side, spur, lateen sail."""
    length, beam, fb, draft = 40.0, 5.5, 1.2, 1.2
    sections = scaled_sections(GALLEY_PROFILE, length, beam, fb, draft)
    parts = [lofted_hull(sections, ring=8)]
    parts += clinker_strakes(sections, 3)
    deck(parts, -17.0, 17.5, beam - 0.5, fb)
    # Outrigger frame (apostis) carrying the oars, wider than the hull.
    for side in (-1, 1):
        parts.append(
            box((30.0, 0.35, 0.25), (0.0, side * (beam / 2 + 0.9), fb + 0.5), "Hull")
        )
    # Central gangway (corsia) and benches.
    parts.append(box((30.0, 1.1, 0.35), (0.0, 0.0, fb + 0.35), "Deck"))
    for i in range(25):
        x = -14.5 + 29.0 * (i + 0.5) / 25
        for side in (-1, 1):
            parts.append(
                box(
                    (0.35, beam / 2 - 0.8, 0.2),
                    (x, side * (beam / 4 + 0.3), fb + 0.35),
                    "Wood",
                )
            )
    bulwark(parts, -15.0, 15.5, beam / 2 + 1.05, fb + 0.4, height=0.8, spacing=1.0)
    oars(parts, -14.5, 14.5, 25, beam / 2 + 1.05, fb + 0.6, 7.5)
    # Rambade (fighting platform at the bow) and aft deck with a canopy.
    castle(parts, 16.4, 3.4, beam - 0.4, fb, 1.0, parapet=0.9)
    parts.append(box((5.5, beam - 0.3, 0.8), (-16.0, 0, fb + 0.4), "Wood"))
    for sx in (-1, 1):
        for sy in (-1, 1):
            parts.append(
                cylinder(0.08, 2.2, (-16.0 + sx * 2.5, sy * 2.2, fb + 1.9), "Wood", 5)
            )
    parts.append(box((5.4, 4.8, 0.08), (-16.0, 0, fb + 3.0), "Banner"))
    # Spur (éperon), ironshod.
    parts.append(rod((18.5, 0, fb + 0.2), (25.0, 0, fb + 0.9), 0.35, "Wood", 6))
    parts.append(rod((24.0, 0, fb + 0.8), (26.0, 0, fb + 1.0), 0.2, "Iron", 5))
    rudder(parts, -20.0, fb, draft)
    # Mast raked forward, lateen yard.
    parts.append(rod((5.0, 0, fb), (5.4, 0, fb + 15.0), 0.25, "Wood", 8))
    parts.append(rod((-6.0, 0, fb + 5.5), (19.0, 0, fb + 17.5), 0.16, "Wood", 6))
    parts.append(lateen_sail(-5.5, 18.5, fb + 5.6, fb + 17.3, 9.0, fb + 2.4))
    for end in ((18.0, 0, fb + 0.8), (-12.0, 0, fb + 0.8)):
        parts.append(rod((5.3, 0, fb + 14.5), end, 0.035, "Rope"))
    return parts


BARGE_PROFILE = [
    (-1.0, 0.12, 0.8, 0.5),
    (-0.8, 0.7, 0.4, 0.9),
    (-0.4, 0.97, 0.05, 1.0),
    (0.2, 1.0, 0.0, 1.0),
    (0.65, 0.8, 0.25, 0.95),
    (0.9, 0.4, 0.6, 0.8),
    (1.03, 0.06, 1.0, 0.5),
]


def build_barge():
    """Barge (balinger), 20 m: one square sail, small aftcastle, 8 oars a side."""
    length, beam, fb, draft = 20.0, 5.0, 1.6, 1.4
    sections = scaled_sections(BARGE_PROFILE, length, beam, fb, draft)
    parts = [lofted_hull(sections, ring=8)]
    parts += clinker_strakes(sections, 4)
    deck(parts, -8.0, 8.0, beam - 0.6, fb)
    bulwark(parts, -5.5, 7.5, beam / 2 - 0.3, fb, height=0.8)
    castle(parts, -7.2, 4.0, 4.2, fb, 1.2, parapet=0.9)
    oars(parts, -4.0, 6.5, 8, beam / 2 - 0.2, fb + 0.7, 4.5)
    rudder(parts, -10.3, fb, draft)
    mast(
        parts,
        1.0,
        fb,
        15.0,
        11.0,
        fb + 12.5,
        beam,
        [(10.6, 0, fb + 1.2), (-9.0, 0, fb + 2.0)],
    )
    parts.append(grid_sail(10.2, fb + 12.3, fb + 3.6, 1.3, rows=6, cols=6))
    return parts


MODELS = {
    "cog": build_cog,
    "nef": build_nef,
    "galley": build_galley,
    "barge": build_barge,
}


def main() -> None:
    """Parse ``-- <out_dir> [names...]`` and export the requested models."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out_dir = Path(args[0]) if args else Path.cwd() / "naval"
    names = args[1:] or list(MODELS)
    models.MODELS.update(MODELS)
    for name in names:
        print(f"MODEL {name} {models.export_model(name, out_dir)}")
    print("OK")


if __name__ == "__main__":
    main()
