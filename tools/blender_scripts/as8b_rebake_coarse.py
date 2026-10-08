"""Lot AS8b: re-bake only the coarse (Quaternius) `cavalry` bone texture.

    blender -b --factory-startup --python tools/blender_scripts/as8b_rebake_coarse.py

The fine rig is baked by ``battle_fine.py -- rigs``. The coarse figures (``--coarse-figures``)
read ``game/assets/models/battle_skinned/cavalry.bones.bin`` and the ``rigs.cavalry`` entry of
its manifest: both are rewritten here, the meshes are left alone.
"""

import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned as bs  # noqa: E402
import battle_skinned_cavalry as cavalry  # noqa: E402

os.makedirs(bs.OUT_DIR, exist_ok=True)
path = os.path.join(bs.OUT_DIR, "manifest.json")
with open(path) as f:
    manifest = json.load(f)
rig = cavalry.bake_cavalry_rig()
manifest["rigs"]["cavalry"] = rig.manifest()
with open(path, "w") as f:
    json.dump(manifest, f, indent=1, sort_keys=True)
print("OK coarse cavalry", len(manifest["rigs"]["cavalry"]["clips"]), "clips")
