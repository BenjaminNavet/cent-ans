"""Blender smoke test: create a cube, export it to a temporary glTF, print OK."""

import sys
import tempfile
from pathlib import Path

import bpy

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.mesh.primitive_cube_add(size=2.0)

with tempfile.TemporaryDirectory() as tmp_dir:
    out_path = Path(tmp_dir) / "smoke.glb"
    bpy.ops.export_scene.gltf(filepath=str(out_path), export_format="GLB")
    if not out_path.exists() or out_path.stat().st_size == 0:
        print("FAIL: glTF export missing")
        sys.exit(1)

print("OK")
