"""Rebuild LOD1-2 of already ingested assets with ``voxel_bake`` (ADR 0790), LOD0 untouched.

Each asset of the given classes is re-ingested into a scratch folder with its manifest
parameters; only its LOD1-2 files replace the shipped ones, and the manifest gets their
triangle counts and ``lod_methods``. Usage:
``uv run --project tools python tools/experiments/lod_voxel_rebake.py <scratch_dir> [-j 4] [ids...]``
"""

import argparse
import json
import shutil
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from cent_ans_tools import dn_ingest

CLASSES = {"house", "major_building", "rock", "bridge", "landmark_city"}


def rebake(asset: dict, classes: dict, scratch: Path) -> tuple[str, str, dict | None]:
    """Re-ingest one asset in ``scratch``; return (id, status, new manifest fields)."""
    asset_id = asset["id"]
    raw = Path(asset["raw"])
    if not raw.exists():
        return asset_id, "raw absent", None
    spec = classes["classes"][asset["class"]]
    axis = spec["scale_axis"]
    work = Path(tempfile.mkdtemp(dir=scratch, prefix=f"{asset_id}_"))
    job = dn_ingest.build_job(
        asset_id=asset_id,
        asset_class=asset["class"],
        raw=raw,
        classes=classes,
        out_dir=work,
        yaw_deg=asset.get("yaw_deg", 0.0),
        tex=asset.get("tex"),
        faithful=asset.get("faithful", False),
        lods=len(asset["files"]),
        **{axis: asset["dimensions_m"][axis]},
    )
    if list(job["lod_tris"][1:]) != list(asset["triangles"][1:]) and any(
        new < old * 0.96
        for new, old in zip(job["lod_tris"][1:], asset["triangles"][1:], strict=True)
    ):
        return (
            asset_id,
            f"budgets différents {asset['triangles']} / {job['lod_tris']}",
            None,
        )
    try:
        result = dn_ingest.run_ingest(job, classes)
    except Exception as error:  # noqa: BLE001 - report and continue the batch
        return asset_id, f"échec {error}", None
    if problems := dn_ingest.check_result(job, result, classes):
        return asset_id, "; ".join(problems), None
    category = Path(job["out_dir"])
    if "voxel_bake" not in result["lod_methods"]:
        shutil.rmtree(work)
        return asset_id, "repli collapse (pièces fines)", None
    for lod, rel in enumerate(asset["files"]):
        if lod:
            shutil.copyfile(
                category / f"{asset_id}_lod{lod}.glb", dn_ingest.MODELS_DIR / rel
            )
    shutil.rmtree(work)
    return (
        asset_id,
        "ok",
        {
            "triangles": [asset["triangles"][0], *result["triangles"][1:]],
            "lod_methods": result["lod_methods"],
        },
    )


def main() -> None:
    """Rebake the selected assets and update the manifest once at the end."""
    parser = argparse.ArgumentParser()
    parser.add_argument("scratch", type=Path)
    parser.add_argument("-j", type=int, default=4)
    parser.add_argument("ids", nargs="*")
    args = parser.parse_args()
    args.scratch.mkdir(parents=True, exist_ok=True)
    classes = dn_ingest.load_classes()
    document = json.loads(dn_ingest.MANIFEST_PATH.read_text())
    assets = [
        a
        for a in document["assets"].values()
        if (a["id"] in args.ids if args.ids else a["class"] in CLASSES)
    ]
    report = {}
    with ThreadPoolExecutor(args.j) as pool:
        for asset_id, status, fields in pool.map(
            lambda a: rebake(a, classes, args.scratch), assets
        ):
            report[asset_id] = status
            print(asset_id, status, flush=True)
            if fields:
                dn_ingest.update_manifest({**document["assets"][asset_id], **fields})
    (args.scratch / "report.json").write_text(
        json.dumps(report, indent=1, ensure_ascii=False)
    )
    print(
        sum(s == "ok" for s in report.values()), "/", len(report), "ok", file=sys.stderr
    )


if __name__ == "__main__":
    main()
