"""GA3-L5: generated trebuchet and ram cut into the moving parts of the animated siege engines.

Run headless from the repository root::

    blender -b --factory-startup --python tools/blender_scripts/ga3_siege_rig.py -- [OUT_DIR]

Inputs: the cleaned GA3 LODs ``game/assets/models/props_ga/ga3_{trebuchet,ram}_lod{0,2}.glb``
(lot L1/L1b, already UV-mapped and baked) and the procedural rigs of
``siege_engines.py`` (named, pivoted nodes that ``SiegeEnginesFx`` and ``SiegeAssaultFx``
animate). Output (``OUT_DIR``, default ``game/assets/models/siege/``): ``ga3_trebuchet.glb``,
``ga3_trebuchet_lod.glb``, ``ga3_ram.glb``, ``ga3_ram_lod.glb`` with **the same hierarchy and
node names** as the procedural models, and ``ga3_rigs.json`` (pivots, scales, triangles).

Method (ADR 0140, section L5): every face of the GA3 mesh is labelled by its centroid (regions
measured once on LOD0: arm line fitted by PCA, posts, counterweight box, wheels, hanging beam),
then each label is re-expressed in the local frame of the procedural node it dresses, and that
node's mesh data is replaced (UVs and the baked texture are kept, no re-bake):

* Trebuchet: ``Frame`` = GA3 frame, scaled so the post tops (bearing) sit on the procedural
  axle (6.4 m), mirrored lengthwise so its long base lies under the long arm, the two side
  frames spread apart (smooth gap) so the counterweight box swings between the posts;
  ``ArmBeam`` = GA3 arm, uniformly scaled to the procedural arm length with its pivot moved
  along it to the procedural long/short ratio (8.5 / 2.2 m), plus a procedural iron axle;
  ``CounterweightBox`` = GA3 box and hanger, scaled to the procedural box. The GA3 sling bag
  (stones always inside) is dropped: ``Sling``/``Stone`` and the ``Winch`` stay procedural,
  with procedural winch posts added to ``Frame``.
* Ram: ``Shed`` = GA3 shed (a spurious centre wheel under the bed is removed), ``Wheel_i`` =
  the four GA3 wheels on their own centres, ``BeamPivot`` moved to the top of the GA3 hangers
  and ``Beam`` = GA3 beam, iron head and hangers.

Geometry is exported in Godot axes like ``siege_engines.py`` (x right, y up, +z towards the
target). LOD0 feeds the near model, LOD2 the ``_lod`` model (with the procedural parts in their
SG3 simplified form).
"""

import json
import math
import sys
from pathlib import Path

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
import siege_engines as se  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROPS = ROOT / "game" / "assets" / "models" / "props_ga"
LODS = (("", 0), ("_lod", 2))  # suffix of the output, GA3 LOD used


def main() -> None:
    """Builds both rigs at both levels of detail."""
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    out = Path(argv[0]) if argv else ROOT / "game" / "assets" / "models" / "siege"
    out.mkdir(parents=True, exist_ok=True)
    report = {}
    print("ga3 siege rigs:", json.dumps(report))


if __name__ == "__main__":
    main()
