"""Parse `footprint <pid>` text output (stdin) into one JSON object: total and per-category MiB.

Usage: footprint <pid> | python3 -I footprint_parse.py
"""

import json
import re
import sys

UNITS = {"B": 1 / 1048576, "KB": 1 / 1024, "MB": 1.0, "GB": 1024.0}
ROW = re.compile(
    r"^\s*([\d.]+) (B|KB|MB|GB)\s+([\d.]+) (B|KB|MB|GB)\s+([\d.]+) (B|KB|MB|GB)\s+(\d+)\s+(.+?)\s*$"
)
PHYS = re.compile(r"phys_footprint:\s*([\d.]+) (B|KB|MB|GB)")


def main() -> None:
    """Read the text from stdin, print the JSON summary."""
    total = None
    categories: dict[str, dict[str, float]] = {}
    for line in sys.stdin:
        match = PHYS.search(line)
        if match:
            total = float(match.group(1)) * UNITS[match.group(2)]
            continue
        match = ROW.match(line)
        if match:
            dirty = float(match.group(1)) * UNITS[match.group(2)]
            clean = float(match.group(3)) * UNITS[match.group(4)]
            categories[match.group(8)] = {
                "dirty": round(dirty, 1),
                "clean": round(clean, 1),
                "regions": int(match.group(7)),
            }
    print(
        json.dumps(
            {
                "footprint_mib": round(total, 1) if total is not None else None,
                "categories": categories,
            }
        )
    )


if __name__ == "__main__":
    main()
