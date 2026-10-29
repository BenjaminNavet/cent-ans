#!/bin/zsh
# Memory measurement protocol of the Bevy client (lot MEM3, ADR 0786, docs/performance-memoire.md).
# Launches a release client hidden and muted (open -g, --offscreen), samples `footprint <pid>` at fixed
# instants and keeps `vmmap -summary` at the end. Repeat N times, one JSON line per sample.
#
# Usage: tools/bevy_bench/mem_protocol.sh <mode> <label> [runs]
#   mode  idle    campaign, nothing played; samples at t+8 s, 30, 60, 120, then kill
#         bench   campaign, full map bench (pan + descents, bench.sh --bench-map); sample every 15 s
#         battle  Crécy (defender) from the menus then the bench-battle (calm + melee); sample every 15 s
#         trip    campaign then battle then back is not scripted: use `battle` after `idle` for now
#   label names out/mem/<label>/ (samples.jsonl, vmmap-<run>-<t>.txt, run<N>.log)
# Env: BIN=<release binary> (default client/target/release/cent-ans), MEM_ENV="A=1 B=2" extra CENT_ANS_* vars,
#      SAMPLE_AT="8 30 60 120" (idle), SAMPLE_EVERY=15 (bench / battle), CALM_LOAD (tools/bevy_bench/calm.sh).
# Output: out/mem/<label>/samples.jsonl ({"run","t","load","footprint_mib","categories":{...}}) and a table on stdout.
set -u
here=${0:A:h}
root=${here:h:h}
calm_root=$root
source "$here/calm.sh"
if [[ -z ${BENCH_UNDER_HEAVY:-} ]]; then
	export BENCH_UNDER_HEAVY=1
	heavy_run "${0:A}" "$@"
	exit $?
fi
mode=${1:?mode: idle|bench|battle}
label=${2:?label}
runs=${3:-3}
bin=${BIN:-$root/client/target/release/cent-ans}
out=$root/out/mem/$label
mkdir -p "$out"
: > "$out/samples.jsonl"

make_app() { # $1 = dir
	local app=$1/CentAnsBevy.app
	mkdir -p "$app/Contents/MacOS"
	cp "$bin" "$app/Contents/MacOS/cent-ans"
	cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PLIST-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>cent-ans</string>
	<key>CFBundleIdentifier</key><string>fr.centans.bevy</string>
	<key>CFBundleName</key><string>CentAnsBevy</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleVersion</key><string>1</string>
	<key>LSUIElement</key><true/>
	<key>LSBackgroundOnly</key><true/>
</dict>
</plist>
PLIST
	codesign --force --sign - "$app" > /dev/null 2>&1 || true
	echo "$app"
}

sample() { # $1 run, $2 t, $3 pid, $4 vmmap-flag
	local json load
	load=$(uptime | sed -E 's/.*load averages?: *([0-9.,]+).*/\1/' | tr ',' '.')
	json=$(footprint "$3" 2> /dev/null | python3 -I "$here/footprint_parse.py")
	[[ -z $json ]] && return 1
	echo "{\"run\":$1,\"t\":$2,\"load\":$load,${json#\{}" >> "$out/samples.jsonl"
	python3 -I -c '
import json, sys
d = json.loads(sys.argv[1]); c = d["categories"]
def g(*names): return sum(c.get(n, {}).get("dirty", 0) for n in names)
# "IOAccelerator" without "(graphics)" is the mimalloc heap (VM tag 100 = VM_MEMORY_IOACCELERATOR, MEM5).
print("run %s t+%3ss footprint %7.0f MiB | GPU unmapped %6.0f mapped %5.0f | heap(mimalloc) %6.0f IOSurf %6.0f MALLOC_LARGE %6.0f SMALL %6.0f TINY %5.0f VM_ALLOC %6.0f mapped %6.0f"
  % (sys.argv[2], sys.argv[3], d["footprint_mib"], g("Owned physical footprint (unmapped) (graphics)"),
     g("IOAccelerator (graphics)"), g("IOAccelerator"), g("IOSurface"),
     g("MALLOC_LARGE", "MALLOC_LARGE (empty)"), g("MALLOC_SMALL"), g("MALLOC_TINY"),
     g("untagged (VM_ALLOCATE)"), g("mapped file")))' "$json" "$1" "$2"
	[[ ${4:-0} == 1 ]] && { vmmap -summary "$3" > "$out/vmmap-run$1-t$2.txt" 2> /dev/null; [[ -n ${VMMAP_FULL:-} ]] && vmmap -v "$3" > "$out/vmmapv-run$1-t$2.txt" 2> /dev/null; }
	return 0
}

for run in $(seq 1 $runs); do
	wait_calm || exit 1
	app=$(make_app "$out/app")
	log=$out/run$run.log
	envs=(--env CENT_ANS_MUTE=1 --env CENT_ANS_NO_BRIEFING=1 --env CENT_ANS_ROOT=$root --env CENT_ANS_PROFILE=release)
	for kv in ${=MEM_ENV:-}; do envs+=(--env "$kv"); done
	case $mode in
	idle)
		args=(--state ${STATE:-Campaign} --exit-after 1000000 --offscreen)
		;;
	bench)
		args=(--state Campaign --bench-map --bench-out "$out/bench-run$run.json" --exit-after 20000 --offscreen)
		;;
	battle)
		envs+=(--env CENT_ANS_NO_DEPLOY=1)
		args=(--state MainMenu --bench-battle --bench-out "$out/battle-run$run.json" --script "$here/battle_open.txt" --exit-after 40000 --offscreen)
		;;
	*) echo "unknown mode $mode" >&2; exit 2 ;;
	esac
	open -g -n -W "${envs[@]}" --stdout "$log" --stderr "$log" "$app" --args "${args[@]}" &
	opener=$!
	pid=""
	for _ in $(seq 1 100); do
		pid=$(pgrep -f "$app/Contents/MacOS/cent-ans" | head -1)
		[[ -n $pid ]] && break
		sleep 0.2
	done
	[[ -z $pid ]] && { echo "run $run: client did not start" >&2; continue; }
	start=$SECONDS
	if [[ $mode == idle ]]; then
		last=${${=SAMPLE_AT:-8 30 60 120}[-1]}
		for t in ${=SAMPLE_AT:-8 30 60 120}; do
			while (( SECONDS - start < t )); do sleep 0.5; done
			sample $run $t $pid $(( t == last ? 1 : 0 )) || break
		done
		kill $pid 2> /dev/null
	else
		next=${SAMPLE_EVERY:-15}
		(( first = 8 ))
		while kill -0 $pid 2> /dev/null; do
			t=$(( SECONDS - start ))
			if (( t >= first )); then
				sample $run $t $pid 0 || break
				first=$(( t + next ))
			fi
			sleep 1
		done
	fi
	wait $opener 2> /dev/null
	rm -rf "$out/app"
done
echo "samples: $out/samples.jsonl"
