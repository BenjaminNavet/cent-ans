"""Copy the scene's glTF assets and scene.json into the standalone Godot project."""

import json
import shutil
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]

scene = json.loads((HERE / "scene.json").read_text())
target = HERE / "godot" / "assets"
target.mkdir(parents=True, exist_ok=True)
for name, source in scene["assets"].items():
    shutil.copy2(REPO / source, target / f"{name}.glb")
shutil.copy2(HERE / "scene.json", target / "scene.json")
print("PROTO godot assets synced")
