#!/bin/zsh
# Automatic performance guard of the Bevy client (lot B5). Muted and hidden window ONLY
# (bench.sh -> client/run_bg.sh, CENT_ANS_MUTE=1, --offscreen): never steals the focus.
#
# Usage: tools/bevy_bench/perf_guard.sh [map|battle|all] [--force]
# Runs the existing bench (tools/bevy_bench/bench.sh, see README.md) and compares its percentiles with
# the thresholds of tools/bevy_bench/perf_thresholds.json (documented there). Exit code 0 = within
# thresholds, 1 = threshold exceeded or bench failed, 3 = skipped because the machine is loaded
# (1-minute load average > max_load, default 6; --force measures anyway).
# Map: panorama d=30 (pan only). Battle: Crecy, calm + melee.
set -u
here=${0:A:h}
root=${here:h:h}
mode=${1:-map}
[[ $mode == map || $mode == battle || $mode == all ]] || { echo "usage: perf_guard.sh [map|battle|all] [--force]" >&2; exit 2; }
force=0; [[ ${2:-} == --force ]] && force=1
conf="$here/perf_thresholds.json"
max_load=$(python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['max_load'])" "$conf")
load=$(uptime | sed -E 's/.*load averages?: *([0-9]+[.,]?[0-9]*).*/\1/' | tr ',' '.')
if (( force == 0 )) && python3 -c "import sys;sys.exit(0 if float(sys.argv[1]) > float(sys.argv[2]) else 1)" "$load" "$max_load"; then
	echo "perf_guard SKIPPED: load average $load > $max_load (use --force to measure anyway)"
	exit 3
fi
export CENT_ANS_MUTE=1
status=0
check() { # <kind: map|battle> <report>
	python3 - "$conf" "$1" "$2" "$load" <<'PY' || status=1
import json, sys
conf, kind, report, load = sys.argv[1:5]
limits = json.load(open(conf))[kind]
rep = json.load(open(report))
bad = 0
for phase, stats in limits.items():
    if phase.startswith("_"):
        continue
    got = rep.get(phase)
    if not got:
        print(f"FAIL {kind}.{phase}: phase absent from the report")
        bad += 1
        continue
    for stat, limit in stats.items():
        value = got["frame_ms_" + stat]
        ok = value <= limit
        bad += not ok
        print(f"{'ok  ' if ok else 'FAIL'} {kind}.{phase}.{stat} = {value:.2f} ms (max {limit}) [load {load}]")
sys.exit(1 if bad else 0)
PY
}
if [[ $mode == map || $mode == all ]]; then
	"$here/bench.sh" guard --bench-distance 30 --bench-pan-only --offscreen > /dev/null || status=1
	check map "$root/out/bench/map_guard.json"
fi
if [[ $mode == battle || $mode == all ]]; then
	"$here/bench.sh" guard --bench-battle --offscreen > /dev/null || status=1
	check battle "$root/out/bench/battle_guard.json"
fi
exit $status
