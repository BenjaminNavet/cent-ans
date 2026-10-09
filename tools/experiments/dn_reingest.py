"""Re-run ``cent-ans dn-ingest`` for manifest assets with the parameters recorded in the manifest.

Used after an ingest-tool fix (DN-TROUS: exact weld before decimation). The manifest itself is
left untouched (a scratch copy absorbs the update). Usage::

    uv run --project tools python tools/experiments/dn_reingest.py [--ids a,b] [--campaign] [-j 4]
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def main() -> None:
    """Re-ingest the selected assets in parallel."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--ids", default="")
    parser.add_argument("--campaign", action="store_true", help="only models of dn_campaign_models.json")
    parser.add_argument("--out-dir", default="", help="ingest root (default game/assets/models/dn)")
    parser.add_argument("-j", type=int, default=4)
    parser.add_argument(
        "--real-manifest", action="store_true", help="update data/art/dn_manifest.json (new raw)"
    )
    parser.add_argument("--raw-override", default="", help="JSON {id: raw glb path}")
    args = parser.parse_args()
    manifest_path = REPO / "data/art/dn_manifest.json"
    assets = json.loads(manifest_path.read_text())["assets"]
    classes = json.loads((REPO / "data/art/dn_ingest_classes.json").read_text())["classes"]
    override = json.loads(args.raw_override) if args.raw_override else {}
    wanted = set(filter(None, args.ids.split(",")))
    if args.campaign:
        table = json.loads((REPO / "data/art/dn_campaign_models.json").read_text())["table"]
        wanted |= {e["path"].split("/")[-1] for fam in table.values() for lst in fam.values() for e in lst}
    scratch = manifest_path
    if not args.real_manifest:
        scratch = Path(tempfile.mkdtemp()) / "manifest.json"
        shutil.copy(manifest_path, scratch)

    def run(asset_id: str) -> tuple[str, int]:
        a = assets[asset_id]
        axis = classes[a["class"]]["scale_axis"]
        command = [
            "uv", "run", "--project", str(REPO / "tools"), "cent-ans", "dn-ingest",
            override.get(asset_id, a["raw"]), "--id", asset_id, "--class", a["class"],
            f"--{axis}", str(a["dimensions_m"][axis]),
            "--yaw", "0.0" if asset_id in override else str(a.get("yaw_deg", 0.0)),
            "--lods", str(len(a["files"])), "--tex", str(a["tex"]),
            "--manifest", str(scratch),
        ]  # fmt: skip
        if args.out_dir:
            command += ["--out-dir", args.out_dir]
        gamma = (a.get("grade") or {}).get("gamma")
        if gamma and asset_id not in override:
            command += ["--gamma", str(gamma)]
        if asset_id in override:
            command += ["--model-3d", "fal-ai/trellis-2" if "fal2" in override[asset_id] else "fal-ai/trellis"]
        done = subprocess.run(command, capture_output=True, text=True)
        print(asset_id, done.returncode, (done.stdout.strip().splitlines() or [""])[-1][:100], flush=True)
        return asset_id, done.returncode

    ids = [i for i in assets if not wanted or i in wanted]
    with ThreadPoolExecutor(args.j) as pool:
        results = list(pool.map(run, ids))
    print("failed:", [i for i, rc in results if rc not in (0, 2)], "budget-flag(rc2):", [i for i, rc in results if rc == 2])


if __name__ == "__main__":
    main()
