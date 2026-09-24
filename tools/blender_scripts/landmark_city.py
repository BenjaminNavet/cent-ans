"""Landmark city generator (lot L1): Paris first, from ``data/landmarks/<id>.json``.

Run headless:
    blender --background --python landmark_city.py -- <landmark.json> <out.glb>

Skeleton: the implementation follows in the next commits.
"""

import json
import sys
from pathlib import Path


def main() -> None:
    """Parse ``-- <landmark.json> <out.glb>`` and build the model."""
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if len(args) < 2:
        raise SystemExit("usage: landmark_city.py -- <landmark.json> <out.glb>")
    landmark = json.loads(Path(args[0]).read_text(encoding="utf-8"))
    print(f"LANDMARK {landmark['id']} (skeleton)")
    print("OK")


if __name__ == "__main__":
    main()
