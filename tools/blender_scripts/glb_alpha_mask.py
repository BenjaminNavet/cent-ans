"""Set alphaMode MASK (cutoff 0.5) and doubleSided on GLB materials matching a substring.

Usage: python3 glb_alpha_mask.py file.glb substr[,substr...]

Plain Python (no Blender): rewrites the JSON chunk of a binary glTF in place.
"""

import json
import struct
import sys

path, subs = sys.argv[1], sys.argv[2].split(",")
with open(path, "rb") as handle:
    data = handle.read()
json_len = struct.unpack("<I", data[12:16])[0]
doc = json.loads(data[20 : 20 + json_len])
rest = data[20 + json_len :]
for mat in doc.get("materials", []):
    if any(s in mat.get("name", "") for s in subs):
        mat["alphaMode"] = "MASK"
        mat["alphaCutoff"] = 0.5
        mat["doubleSided"] = True
        print("MASK", mat["name"])
raw = json.dumps(doc, separators=(",", ":")).encode()
raw += b" " * ((4 - len(raw) % 4) % 4)
total = 12 + 8 + len(raw) + len(rest)
header = struct.pack("<III", 0x46546C67, 2, total) + struct.pack("<II", len(raw), 0x4E4F534A)
with open(path, "wb") as handle:
    handle.write(header + raw + rest)
