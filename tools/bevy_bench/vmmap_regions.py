"""Group the regions of a `vmmap -v` dump (mem_protocol.sh, VMMAP_FULL=1) by type and size class.

Usage: python3 -I vmmap_regions.py <vmmapv.txt> [type-prefix ...]
Prints for each region type (default: the IOAccelerator ones) the count and dirty MiB per size class.
"""

import re
import sys
from collections import defaultdict

ROW = re.compile(r"^(\S.*?)\s{2,}([0-9a-f]+)-([0-9a-f]+)\s+\[\s*([\d.]+[KMG]?)\s+([\d.]+[KMG]?)\s+([\d.]+[KMG]?)\s+([\d.]+[KMG]?)\]")
UNIT = {"K": 1 / 1024, "M": 1.0, "G": 1024.0, "": 1 / 1048576}


def mib(text: str) -> float:
    """Convert vmmap's size text (`48K`, `2.4G`, `0`) to MiB."""
    unit = text[-1] if text[-1] in "KMG" else ""
    return float(text.rstrip("KMG")) * UNIT[unit]


def main() -> None:
    """Print the histogram."""
    prefixes = sys.argv[2:] or ["IOAccelerator"]
    classes: dict[str, dict[str, list[float]]] = defaultdict(lambda: defaultdict(lambda: [0, 0.0, 0.0]))
    for line in open(sys.argv[1], errors="replace"):
        match = ROW.match(line)
        if not match or not any(match.group(1).startswith(p) for p in prefixes):
            continue
        virtual, resident, dirty = mib(match.group(4)), mib(match.group(5)), mib(match.group(6))
        bucket = "<64K" if virtual < 0.0625 else "<1M" if virtual < 1 else "<16M" if virtual < 16 else "<128M" if virtual < 128 else ">=128M"
        row = classes[match.group(1)][bucket]
        row[0] += 1
        row[1] += virtual
        row[2] += dirty
    for kind, buckets in classes.items():
        print(kind)
        for bucket, (count, virtual, dirty) in sorted(buckets.items()):
            print(f"  {bucket:7s} regions {int(count):6d}  virtual {virtual:9.1f}  dirty {dirty:9.1f} MiB")


if __name__ == "__main__":
    main()
