"""Write a relief pyramid manifest listing the tiles present in a cache (ZG2 trials).

Usage: python manifest_from_cache.py <dir>/relief_pyramid.json [template manifest]
The tiles are read from `<dir>/pyramid/E{k}/{col}_{row}.png` (a symlink to the shared cache
works); run the game with `--pyramid-dir=<dir>`.
"""

import json
import os
import sys

src = sys.argv[2] if len(sys.argv) > 2 else "data/map/relief_pyramid.json"
out = sys.argv[1]
with open(src) as f:
    m = json.load(f)
root = os.path.join(os.path.dirname(out), "pyramid")
for e in m["levels"]:
    d = os.path.join(root, f"E{e['level']}")
    rows = {}
    if os.path.isdir(d):
        for f in os.listdir(d):
            if f.endswith(".png") and os.path.getsize(os.path.join(d, f)) > 0:
                c, r = map(int, f[:-4].split("_"))
                rows.setdefault(r, []).append(c)
    rle = []
    for r in sorted(rows):
        runs = []
        for c in sorted(rows[r]):
            if runs and runs[-1][0] + runs[-1][1] == c:
                runs[-1][1] += 1
            else:
                runs.append([c, 1])
        rle.append({"row": r, "runs": runs})
    e["tiles_rle"] = rle
    print(e["level"], sum(len(v) for v in rows.values()))
with open(out, "w") as f:
    json.dump(m, f)
