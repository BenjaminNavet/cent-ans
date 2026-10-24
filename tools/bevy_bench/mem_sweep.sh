#!/bin/zsh
# Attribution sweep (MEM3): one idle campaign run per env setting, prints the lowest footprint of the
# samples (the transient peaks of the GPU command queue are left out) and the last Metal allocation.
# Usage: tools/bevy_bench/mem_sweep.sh <label> "<ENV=1 ENV2=2>" "<other settings>" ...   (BIN, STATE as mem_protocol.sh)
set -u
here=${0:A:h}
root=${here:h:h}
label=$1
shift
i=0
for setting in "$@"; do
	i=$((i + 1))
	SAMPLE_AT=${SAMPLE_AT:-"30 40 50"} MEM_ENV="CENT_ANS_MEMLOG=2 $setting" "$here/mem_protocol.sh" ${MODE:-idle} "$label-$i" 1 > /dev/null 2>&1
	python3 -I - "$root/out/mem/$label-$i" "$setting" << 'PY'
import json, re, sys
from pathlib import Path
d, setting = Path(sys.argv[1]), sys.argv[2]
rows = [json.loads(l) for l in (d / "samples.jsonl").read_text().splitlines() if l.strip()]
ansi = re.compile(r"\x1b\[[0-9;]*m")
metal = "?"
for line in (d / "run1.log").read_text(errors="replace").splitlines():
    m = re.search(r"memlog .* metal=(-?\d+) images=(\d+) gpu_est=(\d+)", ansi.sub("", line))
    if m:
        metal = f"metal={m.group(1)} images={m.group(2)} gpu_est={m.group(3)}"
fp = [r["footprint_mib"] for r in rows]
print(f"{setting or '(base)':45s} footprint min {min(fp):6.0f} max {max(fp):6.0f}  {metal}" if fp else f"{setting}: no sample")
PY
done
