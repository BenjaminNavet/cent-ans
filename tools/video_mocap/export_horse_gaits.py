"""Lot AS8b: merge the `track_quadruped.py fit` outputs into the versioned gait table.

    python3 tools/video_mocap/export_horse_gaits.py trot.json gallop.json OUT.json
"""

import json
import sys
from pathlib import Path

DOC = (
    "Courbes d'allures du cheval mesurées par tools/video_mocap/track_quadruped.py (lot AS8b) "
    "sur des planches de Muybridge (domaine public). Par membre : u = sabot en avant de la "
    "racine du membre, d = sabot sous la racine, en longueurs de jambe, sur un cycle de "
    "`samples` pas ; upper/lower = angles (rad depuis la verticale, avant positif) des deux "
    "segments ; rise = élévation du corps (longueurs de jambe), pitch = cabré (rad). "
    "Attribution : tools/blender_scripts/data/SOURCE.md."
)


def rounded(values, digits=4):
    return [round(v, digits) for v in values]


def main(trot_path, gallop_path, out_path):
    out = {"_doc": DOC, "gaits": {}}
    for name, path in (("trot", trot_path), ("gallop", gallop_path)):
        d = json.loads(Path(path).read_text())
        legs = {}
        for leg, curves in d["legs"].items():
            legs[leg] = {
                "u": rounded(curves["u"]),
                "d": rounded(curves["d"]),
                "h": rounded(curves["h"]),
                "upper": rounded(d["joints"][leg]["upper"], 3),
                "lower": rounded(d["joints"][leg]["lower"], 3),
            }
        out["gaits"][name] = {
            "source": d["source"],
            "frames_measured": d["frames"],
            "period_frames": d["period_frames"],
            "samples": d["samples"],
            "planted_tol": d["planted_tol"],
            "duty": {k: round(v["duty"], 2) for k, v in d["stance"].items()},
            "legs": legs,
            "rise": rounded(d["rise"]),
            "pitch": rounded(d["pitch"]),
        }
    Path(out_path).write_text(json.dumps(out, separators=(",", ":"), ensure_ascii=False))


if __name__ == "__main__":
    main(*sys.argv[1:4])
