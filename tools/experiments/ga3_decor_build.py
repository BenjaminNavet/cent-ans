"""GA3-L1: clean every catalogue object into game LODs and write the props manifest.

Run from the repository root once the raw meshes exist (``ga3_fal_decor.py --catalog``)::

    python tools/experiments/ga3_decor_build.py [--only church,well] [--raw ~/dev/cent-ans-raw/ga3]

For each object of ``data/art/ga3_decor.json``: ``blender -b`` runs
``tools/blender_scripts/ga3_cleanup.py`` on ``RAW/<raw>/<glb>`` with the object's size, triangle
cap, texture size and the catalogue's albedo grading, writing
``game/assets/models/props_ga/ga3_<id>_lod{0,1,2}.glb``. Then
``game/assets/models/props_ga/manifest.json`` gathers, per object, the kit type it stands in
for, its share, its measured footprint (``length`` along X, ``depth`` along Z, ``height``) and
the triangles of each LOD; ``Ga3Kit`` (``game/scripts/visual/ga3_kit.gd``) reads it.
"""

import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CATALOG = ROOT / "data" / "art" / "ga3_decor.json"
OUT_DIR = ROOT / "game" / "assets" / "models" / "props_ga"
CLEANUP = ROOT / "tools" / "blender_scripts" / "ga3_cleanup.py"


def raw_mesh(entry: dict, raw_root: Path) -> Path:
    """Raw glb chosen for a catalogue entry."""
    folder = entry.get("raw") or entry.get("source") or f"l1/{entry['id']}"
    default = "trellis.glb" if entry["model"] == "multi" else f"{entry['model']}.glb"
    return raw_root / folder / entry.get("glb", default)


def cleanup_command(entry: dict, source: Path, cleanup: dict, stats: Path) -> list[str]:
    """Blender command line cleaning ``source`` for ``entry``."""
    command = [
        "blender",
        "-b",
        "--factory-startup",
        "--python",
        str(CLEANUP),
        "--",
        str(source),
        str(OUT_DIR),
        f"ga3_{entry['id']}",
        "--length",
        str(entry["size_m"]),
        "--scale-axis",
        entry["scale_axis"],
        "--lod0",
        str(entry["lod0"]),
        "--tex",
        str(entry["tex"]),
        "--auto-levels",
        str(cleanup.get("auto_levels", 0.0)),
        "--gamma",
        str(cleanup.get("gamma", 1.0)),
        "--normal",
        cleanup.get("normal", "auto"),
        "--yaw",
        str(entry.get("yaw_deg", 0.0)),
        "--stats",
        str(stats),
    ]
    if entry.get("align", True):
        command.append("--align")
    if entry.get("strip_base", 0.0) > 0.0:
        command += ["--strip-base", str(entry["strip_base"])]
    if "island_min" in entry:
        command += ["--island-min", str(entry["island_min"])]
    return command


def manifest_entry(entry: dict, stats: dict) -> dict:
    """Manifest record: Blender size (x, y, z) becomes Godot length (X), depth (Z), height (Y)."""
    size = stats["lods"][0]["size"]
    return {
        "kit_kind": entry["kit_kind"],
        "fit": entry["fit"],
        "share": entry["share"],
        "wired": entry["wired"],
        "length": size[0],
        "depth": size[1],
        "height": size[2],
        "triangles": [lod["triangles"] for lod in stats["lods"]],
    }


def main() -> None:
    """Clean the selected objects and rewrite the manifest."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", default="")
    parser.add_argument("--raw", default=str(Path.home() / "dev/cent-ans-raw/ga3"))
    args = parser.parse_args()
    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    only = set(filter(None, args.only.split(",")))
    manifest_path = OUT_DIR / "manifest.json"
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    for entry in catalog["objects"]:
        if only and entry["id"] not in only:
            continue
        source = raw_mesh(entry, Path(args.raw))
        stats_path = source.parent / f"stats_{entry['id']}.json"
        command = cleanup_command(entry, source, catalog["cleanup"], stats_path)
        log = source.parent / f"cleanup_{entry['id']}.log"
        with log.open("w") as handle:
            subprocess.run(command, check=True, stdout=handle, stderr=subprocess.STDOUT)
        stats = json.loads(stats_path.read_text())
        manifest[f"ga3_{entry['id']}"] = manifest_entry(entry, stats)
        print(f"GA3 {entry['id']}: {manifest[f'ga3_{entry["id"]}']}")
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
