"""Condense the `memlog` lines of the client logs of a mem_protocol.sh label (CENT_ANS_MEMLOG=<s>).

Usage: python3 -I memlog_view.py <out/mem/label> [every_n_lines]
Prints, per run, t / frame / footprint / metal / images / gpu_est every N lines.
"""

import re
import sys
from pathlib import Path

LINE = re.compile(r"memlog t=(\d+)s frame=(\d+) footprint=(-?\d+) metal=(-?\d+) images=(\d+) gpu_est=(\d+) meshes=(\d+)")
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def main() -> None:
    """Print the condensed series of every run log in the directory."""
    directory = Path(sys.argv[1])
    step = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    for log in sorted(directory.glob("run*.log")):
        rows = [m.groups() for line in log.read_text(errors="replace").splitlines() if (m := LINE.search(ANSI.sub("", line)))]
        print(log.name, f"({len(rows)} lines)")
        print("  t  frame footprint metal images gpu_est")
        for row in rows[::step]:
            print("  " + " ".join(row[:6]))


if __name__ == "__main__":
    main()
