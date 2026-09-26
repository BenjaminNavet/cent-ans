"""Lot FG4: fine horse in production for the mounted battle figures (``cavalry`` rig).

Run from the repository root:

    blender -b --factory-startup --python tools/blender_scripts/battle_fine_cavalry.py -- \
        [--only cavalry_0,standard_1] [--render DIR] [--no-export]

Builds the CC0 "Rigged Horse" (``battle_fine_horse``) once, derives the horse types
(destrier, rouncey, jennet), then for every mounted recipe of ``battle_skinned_figures``
assembles horse + harness + the current Quaternius rider and exports three LODs in the
``CAM1`` format to ``game/assets/models/battle_fine/`` with a manifest fragment
``manifest_horse.json`` (merged by the ``--fine-figures`` loader, lot FG1).
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import battle_skinned as bs  # noqa: E402

OUT_DIR = os.path.join(bs.ROOT, "game", "assets", "models", "battle_fine")
MANIFEST = os.path.join(OUT_DIR, "manifest_horse.json")


def main():
    """Parse the arguments and run the export (skeleton)."""
    raise SystemExit("FG4: not implemented yet")


if __name__ == "__main__":
    main()
