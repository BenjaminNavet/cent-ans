#!/bin/zsh
# A/B perf helper: builds once (release), runs A B A B ... alternated, prints mean p50/p95 per
# variant and the delta B-A.
# Usage: tools/bevy_bench/ab.sh <map|battle> <label> "<envA>" "<envB>" [pairs=3]
#   envA / envB: space-separated VAR=value assignments, e.g. "CENT_ANS_PERF_OFF=shadows" or "".
# Reads frame_ms_p50/p95 of `pan` (map) or `calm` + `melee` (battle). Reports: out/bench/ab_<label>_*.json
set -u
here=${0:A:h}
root=${here:h:h}
(( $# >= 4 )) || { echo "usage: ab.sh <map|battle> <label> \"<envA>\" \"<envB>\" [pairs]" >&2; exit 2; }
mode=$1 label=$2 env_a=$3 env_b=$4 pairs=${5:-3}
[[ $mode == map || $mode == battle ]] || { echo "mode must be map or battle" >&2; exit 2; }
extra=()
[[ $mode == battle ]] && extra=(--bench-battle)
# One build for all runs (run_bg.sh rebuilds only what changed).
(cd "$root/client" && cargo build --release) >&2 || exit 1
for ((i = 1; i <= pairs; i++)); do
	for variant in A B; do
		if [[ $variant == A ]]; then assigns=$env_a; else assigns=$env_b; fi
		run="ab_${label}_${variant}${i}"
		echo "== $run ($assigns)" >&2
		# shellcheck disable=SC2086
		env ${=assigns} "$here/bench.sh" "$run" "${extra[@]}" > /dev/null || echo "run $run failed" >&2
	done
done
python3 - "$root/out/bench" "$mode" "$label" "$pairs" <<'PY'
import json, os, statistics, sys
d, mode, label, pairs = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
phases = ["pan"] if mode == "map" else ["calm", "melee"]
res = {}
for v in "AB":
    for i in range(1, pairs + 1):
        p = os.path.join(d, f"{mode}_ab_{label}_{v}{i}.json")
        if not os.path.exists(p):
            continue
        j = json.load(open(p))
        for ph in phases:
            if j.get(ph):
                for k in ("p50", "p95"):
                    res.setdefault((ph, v, k), []).append(j[ph]["frame_ms_" + k])
print(f"{'phase':8}{'stat':5}{'A (ms)':>9}{'B (ms)':>9}{'delta':>9}{'n':>4}")
for ph in phases:
    for k in ("p50", "p95"):
        a, b = res.get((ph, "A", k), []), res.get((ph, "B", k), [])
        if a and b:
            ma, mb = statistics.mean(a), statistics.mean(b)
            print(f"{ph:8}{k:5}{ma:9.2f}{mb:9.2f}{mb - ma:+9.2f}{min(len(a), len(b)):4}")
PY
