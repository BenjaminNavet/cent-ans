#!/bin/zsh
# Automatic performance guard of the Bevy client (lot B5, rewritten by PERF-COMPTEURS, ADR 0827).
# Muted and hidden window ONLY (bench.sh -> client/run_bg.sh, CENT_ANS_MUTE=1, --offscreen): never
# steals the focus.
#
# Usage: tools/bevy_bench/perf_guard.sh [map|battle|all]
# Runs the bench (tools/bevy_bench/bench.sh, build with the cargo feature `perf-counters`) and checks the
# WORK counters of each phase against tools/bevy_bench/perf_thresholds.json (v2): draw calls,
# triangles, pipelines created, CPU ms of the process and of the main thread (all in the `work`
# object of the report), plus the memory of the end of the bench. Frame times are printed for
# information, never compared: they depend on how busy the machine is, the counters do not, so
# there is no load check and no skip (exit codes: 0 ok, 1 threshold exceeded / bench failed /
# counter missing, 2 usage).
# Map: panorama d=30 (pan) and the three descents. Battle: Crecy, calm + melee.
# Recalibrate with perf_calibrate.py after 3 runs (see README.md).
set -u
here=${0:A:h}
root=${here:h:h}
mode=${1:-map}
[[ $mode == map || $mode == battle || $mode == all ]] || { echo "usage: perf_guard.sh [map|battle|all]" >&2; exit 2; }
conf="$here/perf_thresholds.json"
export CENT_ANS_MUTE=1
# Counters do not depend on the load: no wait for a quiet machine (bench.sh -> calm.sh).
export CALM_SKIP=1
rc=0
check() { # <kind: map|battle> <report>
	python3 "$here/perf_check.py" "$conf" "$1" "$2" || rc=1
}
if [[ $mode == map || $mode == all ]]; then
	"$here/bench.sh" guard --bench-distance 30 --offscreen > /dev/null || rc=1
	check map "$root/out/bench/map_guard.json"
fi
if [[ $mode == battle || $mode == all ]]; then
	"$here/bench.sh" guard --bench-battle --offscreen > /dev/null || rc=1
	check battle "$root/out/bench/battle_guard.json"
fi
exit $rc
