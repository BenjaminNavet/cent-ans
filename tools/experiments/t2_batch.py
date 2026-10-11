"""T2: regenerate the DN buildings/props/siege models with TRELLIS.2 run locally (ADR 0342).

Two sub-commands:

``generate`` loads TRELLIS.2 once and turns every source image of the scope into
``~/dev/cent-ans-raw/dn/<id>/3d/trellis2__s42.glb``. It must run with the trellis-mac
interpreter and re-executes itself under ``tools/gpu_lock.sh``. Existing glbs are skipped
(resume), failures go to ``~/dev/cent-ans-raw/t2/failures.jsonl``, no ``.obj`` is kept::

    ~/dev/cent-ans-raw/trellis-mac/.venv/bin/python tools/experiments/t2_batch.py generate [--ids a,b]

``ingest`` runs ``cent-ans dn-ingest`` on each new glb with the class, dimension and yaw of the
current manifest entry, then ``fix_inverted_faces.py`` on the three LODs::

    python3 tools/experiments/t2_batch.py ingest [--ids a,b] [--manifest PATH]
"""

from __future__ import annotations

import argparse
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RAW = Path.home() / "dev/cent-ans-raw"
TRELLIS_MAC = RAW / "trellis-mac"
STATE_DIR = RAW / "t2"
SEED = 42
SCOPE_DIRS = ("buildings", "props", "siege")
KEEP_MODELS = ("fal-ai/trellis-2",)  # already TRELLIS.2 (their source images are gone)


def scope(ids: set[str]) -> list[dict]:
    """Manifest entries to regenerate, optionally restricted to ``ids``."""
    raise NotImplementedError


def raw_glb(asset_id: str) -> Path:
    """Where the TRELLIS.2 raw glb of ``asset_id`` is written."""
    return RAW / "dn" / asset_id / "3d" / f"trellis2__s{SEED}.glb"


def generate(args: argparse.Namespace) -> None:
    """Generate the missing raw glbs with one loaded pipeline."""
    raise NotImplementedError


def ingest(args: argparse.Namespace) -> None:
    """Ingest the raw glbs with the current manifest parameters, then fix inverted faces."""
    raise NotImplementedError


def main() -> None:
    """Parse the sub-command."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    gen = sub.add_parser("generate")
    gen.add_argument("--ids", default="")
    gen.add_argument("--min-free-gb", type=float, default=8.0)
    gen.add_argument("--max-attempts", type=int, default=2)
    ing = sub.add_parser("ingest")
    ing.add_argument("--ids", default="")
    ing.add_argument("--manifest", default=str(REPO / "data/art/dn_manifest.json"))
    ing.add_argument("-j", type=int, default=3)
    args = parser.parse_args()
    {"generate": generate, "ingest": ingest}[args.command](args)


if __name__ == "__main__":
    main()
