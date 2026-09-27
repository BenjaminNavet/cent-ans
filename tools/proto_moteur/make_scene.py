"""Generate scene.json: the placements, camera and sun shared by the Godot and Unreal renders.

Axes are Godot/glTF (metres, Y up, -Z away from the camera); yaw in degrees around Y.
"""

import json
import math
import random
from pathlib import Path

import terrain

HERE = Path(__file__).resolve().parent
GAME_ASSETS = "game/assets"
# Game models converted to plain PBR glTF by materialize.py (see build_assets.sh).
SOURCE_FILES = {
    "infantry_0": f"{GAME_ASSETS}/models/battle/infantry_0.glb",
    "infantry_1": f"{GAME_ASSETS}/models/battle/infantry_1.glb",
    "infantry_2": f"{GAME_ASSETS}/models/battle/infantry_2.glb",
    "wall_run_0": f"{GAME_ASSETS}/models/buildings/wall_run_0.glb",
    "barn_0": f"{GAME_ASSETS}/models/buildings/barn_0.glb",
    "haystack_0": f"{GAME_ASSETS}/models/buildings/haystack_0.glb",
    "cart_0": f"{GAME_ASSETS}/models/buildings/cart_0.glb",
    "fir_tree_01": f"{GAME_ASSETS}/third_party/vegetation/fir_tree_01/fir_tree_01.glb",
    "grass_medium_01": f"{GAME_ASSETS}/third_party/vegetation/grass_medium_01/grass_medium_01.glb",
}
ASSET_FILES = {name: f"tools/proto_moteur/assets/{name}.glb" for name in ["ground", *SOURCE_FILES]}
GRASS_NODES = [f"grass_medium_01_geonodes_{kind}_LOD2" for kind in ("large_a", "large_b", "mid_a", "mid_b", "tall_a", "tall_b")]
FIR_NODES = [f"fir_tree_01_{variant}_LOD2" for variant in "abc"]


def place(asset: str, x: float, z: float, yaw: float = 0.0, scale: float = 1.0, node: str = "") -> dict:
    """Return one placement standing on the ground."""
    return {"asset": asset, "node": node, "pos": [round(x, 3), round(terrain.height(x, z), 3), round(z, 3)], "yaw": round(yaw, 1), "scale": round(scale, 3)}


def build() -> dict:
    """Lay out the tiny section: two ranks before a wall, a barn, firs and meadow grass."""
    rng = random.Random(1346)
    placements = [{"asset": "ground", "node": "", "pos": [0, 0, 0], "yaw": 0, "scale": 1}]
    for rank in range(2):
        for file in range(6):
            x = -2.75 + file * 1.1 + rng.uniform(-0.08, 0.08)
            placements.append(place(f"infantry_{(file + rank) % 3}", x, -1.0 - rank * 1.3, rng.uniform(-6, 6)))
    placements += [
        place("wall_run_0", -3.0, -9.0, 4),
        place("wall_run_0", 6.9, -8.4, 8),
        place("barn_0", -14.0, -19.0, 25),
        place("haystack_0", 8.0, -14.0, 0),
        place("haystack_0", 11.5, -16.5, 40, 0.9),
        place("cart_0", 4.5, -5.0, 70),
    ]
    for index in range(9):
        x = -24 + index * 6 + rng.uniform(-2, 2)
        z = -30 - rng.uniform(0, 8)
        placements.append(place("fir_tree_01", x, z, rng.uniform(0, 360), rng.uniform(0.9, 1.2), FIR_NODES[index % 3]))
    for _ in range(1400):
        radius = 24 * math.sqrt(rng.random())
        angle = rng.uniform(0, math.tau)
        x, z = radius * math.cos(angle), 2 + radius * math.sin(angle) * 0.9 - 6
        placements.append(place("grass_medium_01", x, z, rng.uniform(0, 360), rng.uniform(2.2, 3.4), rng.choice(GRASS_NODES)))
    eye_x, eye_z = 3.2, 9.0
    return {
        "assets": ASSET_FILES,
        "placements": placements,
        "camera": {"pos": [eye_x, terrain.height(eye_x, eye_z) + 1.7, eye_z], "look_at": [-1.0, 1.4, -6.0], "fov_y": 40},
        "sun": {"elevation": 14, "azimuth": 235, "color": [1.0, 0.86, 0.68], "lux": 60000},
        "fog": {"density": 0.012, "color": [0.72, 0.76, 0.82]},
        "resolution": [1920, 1080],
    }


if __name__ == "__main__":
    (HERE / "sources.txt").write_text("".join(f"{name} {path}\n" for name, path in SOURCE_FILES.items()))
    (HERE / "scene.json").write_text(json.dumps(build(), indent=1))
    print("PROTO scene.json written")
