#!/bin/zsh
# Campaign map bench of the Bevy client (port of game/scripts/dev/map_bench.gd), release build.
# Runs `cent-ans --bench-map` in the background window of client/run_bg.sh (no focus stolen), vsync
# off, and prints the JSON report (also written to out/bench/map_<label>.json at the repo root).
#
# Usage: tools/bevy_bench/bench.sh [label] [options of --bench-map: --bench-distance N
#        --bench-seconds N --bench-pan-only --bench-descent-only] [--faction F] [--offscreen]
# Env:   BENCH_TIMEOUT=<seconds> (default 300; raise it when the build is not warm: it covers the build)
#        BENCH_FEATURES=profile  per-system timings in the report (client feature `profile`)
# Exit code: 0 report written, 1 panic / AGENT_FATAL / no report / timeout.
set -u
here=${0:A:h}
root=${here:h:h}
label=${1:-release}
(( $# )) && shift
out="$root/out/bench/map_$label.json"
log="$root/out/bench/map_$label.log"
mkdir -p "$root/out/bench"
rm -f "$out"
export CENT_ANS_PROFILE=release CENT_ANS_MUTE=1 CENT_ANS_NO_BRIEFING=1
# BENCH_FEATURES=profile adds the per-system timings ("systems" section) to the report.
if [[ -n "${BENCH_FEATURES:-}" ]]; then export CENT_ANS_FEATURES=$BENCH_FEATURES; else unset CENT_ANS_FEATURES; fi
cd "$root"
"$root/client/run_bg.sh" --state Campaign --bench-map --bench-out "$out" \
	--exit-after 20000 "$@" > "$log" 2>&1 &
pid=$!
( sleep ${BENCH_TIMEOUT:-300}; pkill -f "CentAnsBevy.app/Contents/MacOS/cent-ans"; kill $pid ) >/dev/null 2>&1 &
dog=$!
wait $pid
code=$?
pkill -P $dog 2>/dev/null; kill $dog 2>/dev/null; wait $dog 2>/dev/null
if (( code != 0 )) || ! grep -q "AGENT_EXIT bench" "$log" || [[ ! -s $out ]]; then
	echo "BENCH FAILED (exit $code), log: $log" >&2
	grep -m5 -E "panicked at|AGENT_FATAL|ERROR|AGENT_EXIT" "$log" | cut -c1-300 >&2
	exit 1
fi
cat "$out"
