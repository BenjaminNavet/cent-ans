"""Builds the shared P0 world: scene.json v2 plus binary height field and instance lists.

Both engines read the same files, so they draw exactly the same positions:
- ``assets/heightmap.f32``: ``N × N`` little-endian f32 heights in metres, row-major, row = z.
  Sample (i, j) sits at ``x = -size/2 + j * size/(N-1)``, ``z = -size/2 + i * size/(N-1)``.
- ``assets/army.f32``, ``assets/grass.f32``, ``assets/firs.f32``: records of 6 little-endian f32
  ``x, y, z, yaw_deg, scale, model_index`` (``model_index`` indexes ``army.models``; 0 elsewhere).

Run from anywhere: ``python3 tools/proto_bevy/make_world.py``.
"""

import json
import math
import random
import struct
from array import array
from pathlib import Path

import terrain

ROOT = Path(__file__).resolve().parent
ASSETS = ROOT / "assets"
SIZE_M = 1000.0
GRID = 1025
FARM_RADIUS_M = 60.0
SEED = 1337


def smoothstep(edge0: float, edge1: float, value: float) -> float:
    """Return the Hermite smoothstep of ``value`` between the two edges."""
    t = min(1.0, max(0.0, (value - edge0) / (edge1 - edge0)))
    return t * t * (3.0 - 2.0 * t)


def height(x: float, z: float) -> float:
    """Return the height in metres: the v1 farm field near the origin, broad hills beyond."""
    hills = 22.0 * math.sin(x * 0.009 + 1.3) * math.cos(z * 0.011) + 9.0 * math.sin(
        x * 0.023 - z * 0.017
    )
    hills += 0.05 * max(0.0, -z - 150.0)
    blend = smoothstep(FARM_RADIUS_M, 220.0, math.hypot(x, z))
    return terrain.height(x, z) + blend * hills


def write_records(path: Path, records: list[tuple[float, ...]]) -> None:
    """Write records of 6 f32 values, little-endian."""
    with path.open("wb") as handle:
        for record in records:
            handle.write(struct.pack("<6f", *record))


def build_heightmap() -> None:
    """Write the f32 height field."""
    step = SIZE_M / (GRID - 1)
    values = array("f")
    for row in range(GRID):
        z = -SIZE_M / 2 + row * step
        values.extend(height(-SIZE_M / 2 + col * step, z) for col in range(GRID))
    if values.itemsize != 4:
        raise RuntimeError("array('f') is not 32-bit on this platform")
    (ASSETS / "heightmap.f32").write_bytes(values.tobytes())


def build_army(rng: random.Random, army: dict) -> list[tuple[float, ...]]:
    """Return soldiers in blocks laid out in two lines across the field in front of the farm."""
    records = []
    per_block = army["count"] // army["blocks"]
    width = army["block_width"]
    spacing = army["spacing_m"]
    for block in range(army["blocks"]):
        line, slot = divmod(block, army["blocks"] // 2)
        origin_x = -120.0 + slot * 60.0
        origin_z = -90.0 - line * 70.0
        for index in range(per_block):
            rank, file = divmod(index, width)
            x = origin_x + file * spacing + rng.uniform(-0.15, 0.15)
            z = origin_z - rank * spacing + rng.uniform(-0.15, 0.15)
            records.append(
                (
                    x,
                    height(x, z),
                    z,
                    army["yaw"],
                    1.0,
                    float(rng.randrange(len(army["models"]))),
                )
            )
    return records


def build_scatter(
    rng: random.Random,
    count: int,
    inner_m: float,
    outer_m: float,
    scale: tuple[float, float],
) -> list[tuple[float, ...]]:
    """Return ``count`` uniformly scattered instances in the ring ``inner_m``..``outer_m``."""
    records = []
    while len(records) < count:
        x, z = rng.uniform(-outer_m, outer_m), rng.uniform(-outer_m, outer_m)
        if not inner_m <= math.hypot(x, z) <= outer_m:
            continue
        records.append(
            (x, height(x, z), z, rng.uniform(0.0, 360.0), rng.uniform(*scale), 0.0)
        )
    return records


def main() -> None:
    """Rewrite scene.json as v2 and regenerate the binary world files."""
    scene_path = ROOT / "scene.json"
    scene = json.loads(scene_path.read_text())
    dropped = {
        "ground",
        "grass_medium_01",
        "fir_tree_01",
        "infantry_0",
        "infantry_1",
        "infantry_2",
    }
    scene["placements"] = [p for p in scene["placements"] if p["asset"] not in dropped]
    scene["assets"] = {k: v for k, v in scene["assets"].items() if k != "ground"}
    scene["version"] = 2
    scene["fog"]["density"] = (
        0.002  # v1 value (0.012) hid everything beyond ~150 m on the 1 km field
    )
    scene["heightmap"] = {
        "file": "assets/heightmap.f32",
        "grid": GRID,
        "size_m": SIZE_M,
        "texture": "grass_path_2",
        "uv_tile_m": terrain.UV_TILE_M,
    }
    scene["army"] = {
        "file": "assets/army.f32",
        "count": 5000,
        "blocks": 10,
        "block_width": 25,
        "spacing_m": 1.2,
        "yaw": 0.0,
        "models": ["infantry_0", "infantry_1", "infantry_2"],
        "walk": {"bob_m": 0.06, "sway_deg": 3.0, "hz": 1.8},
    }
    scene["vegetation"] = {
        "grass": {
            "file": "assets/grass.f32",
            "asset": "grass_medium_01",
            "node": "grass_medium_01_geonodes_mid_a_LOD2",
            "count": 200000,
            "inner_m": 0.0,
            "outer_m": 150.0,
            "fraction": 1.0,
        },
        "firs": {
            "file": "assets/firs.f32",
            "asset": "fir_tree_01",
            "node": "fir_tree_01_a_LOD2",
            "count": 2000,
            "inner_m": 120.0,
            "outer_m": 450.0,
            "fraction": 1.0,
        },
    }
    scene["camera_path"] = [
        {"t": 0.0, "pos": [0.0, 120.0, 160.0], "look_at": [0.0, 0.0, -80.0]},
        {"t": 10.0, "pos": [40.0, 30.0, 40.0], "look_at": [0.0, 2.0, -100.0]},
        {
            "t": 20.0,
            "pos": scene["camera"]["pos"],
            "look_at": scene["camera"]["look_at"],
        },
    ]
    scene["ui_panel"] = {
        "title": "Comté de Ponthieu",
        "body": "Revenus : 412 livres par an. Ordre public : stable. Garnison : 120 hommes.",
        "button": "Lever l'ost",
        "tooltip": "Convoque les vassaux : +500 hommes, -40 livres par tour.",
    }
    scene["quality"] = "comparable"
    scene["bench"] = {
        "warmup_frames": 120,
        "duration_s": 20.0,
        "vsync": False,
        "resolution": [1600, 900],
    }

    rng = random.Random(SEED)
    ASSETS.mkdir(exist_ok=True)
    build_heightmap()
    write_records(ASSETS / "army.f32", build_army(rng, scene["army"]))
    grass, firs = scene["vegetation"]["grass"], scene["vegetation"]["firs"]
    write_records(
        ASSETS / "grass.f32",
        build_scatter(
            rng, grass["count"], grass["inner_m"], grass["outer_m"], (0.8, 1.2)
        ),
    )
    write_records(
        ASSETS / "firs.f32",
        build_scatter(rng, firs["count"], firs["inner_m"], firs["outer_m"], (0.8, 1.4)),
    )
    scene_path.write_text(json.dumps(scene, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
