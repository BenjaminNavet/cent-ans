#!/bin/zsh
# Smoke tests of the Bevy client: every scenario of tests/smoke/*.txt runs in the background
# window of `client/run_bg.sh` (open -g, no focus), and fails on a panic, an AGENT_FATAL, an ERROR
# log line, a missing `AGENT_EXIT script` marker (crash without panic, hang, safety net fired) or a
# watchdog timeout. ~4 minutes for the nine scenarios once the debug build is warm.
#
# Usage: client/tests/smoke.sh [scenario ...]        (default: all)
# Env:   SMOKE_PROFILE=debug|release   (default debug: overflow checks and debug assertions on)
#        SMOKE_TIMEOUT=<seconds>       per scenario (default 240)
#        SMOKE_LOGS=<dir>              where the logs go (default out/smoke/logs, repo root)
# Exit code: 0 all passed, 1 at least one failed.
set -u
here=${0:A:h}
client=${here:h}
root=${client:h}
profile=${SMOKE_PROFILE:-debug}
limit=${SMOKE_TIMEOUT:-240}
logs=${SMOKE_LOGS:-$root/out/smoke/logs}
mkdir -p "$logs" "$root/out/smoke"

if [[ $profile == debug ]]; then
	export CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug
else
	export CENT_ANS_PROFILE=release
	unset CENT_ANS_FEATURES
fi
# Same hooks for every scenario: no sound, no briefing, no deployment phase, no saves written.
export CENT_ANS_MUTE=1 CENT_ANS_NO_BRIEFING=1 CENT_ANS_NO_DEPLOY=1

# scenario -> extra environment (one line `NAME=value ...`) and CLI arguments.
typeset -A scenario_env scenario_args
scenario_env[campaign_battle]="CENT_ANS_STAGE_BATTLE=1 CENT_ANS_FLOW_SIM_TO_END=1 CENT_ANS_LOADING_SECONDS=0"
scenario_args[campaign_turns]="--faction fac_england"

if (( $# )); then names=("$@"); else names=(${(f)"$(cd "$here/smoke" && ls *.txt | sed 's/\.txt$//')"}); fi

# Build once (run_bg.sh builds again, which is then a no-op).
build=(cargo build)
[[ $profile == release ]] && build+=(--release)
[[ $profile == debug ]] && build+=(--features dev)
(cd "$client" && "${build[@]}") >&2 || { echo "SMOKE build failed"; exit 1; }

failed=()
for name in $names; do
	script="$here/smoke/$name.txt"
	log="$logs/$name.log"
	[[ -f $script ]] || { echo "SMOKE $name: no such scenario"; failed+=($name); continue; }
	start=$SECONDS
	(
		cd "$root"
		for pair in ${=scenario_env[$name]:-}; do export "$pair"; done
		exec "$client/run_bg.sh" --state MainMenu ${=scenario_args[$name]:-} \
			--exit-after 6000 --script "$script"
	) > "$log" 2>&1 &
	pid=$!
	# Watchdog: `open -W` hides the app's status and can hang with it; kill after the limit.
	( sleep $limit; pkill -f "CentAnsBevy.app/Contents/MacOS/cent-ans" 2>/dev/null; kill $pid 2>/dev/null ) &
	dog=$!
	wait $pid
	status=$?
	kill $dog 2>/dev/null; wait $dog 2>/dev/null
	elapsed=$((SECONDS - start))
	reasons=()
	(( status != 0 )) && reasons+=("run_bg exit $status")
	grep -q "panicked at" "$log" && reasons+=("panic")
	grep -q "AGENT_FATAL" "$log" && reasons+=("AGENT_FATAL")
	grep -q "AGENT_EXIT script" "$log" || reasons+=("script did not reach exit")
	nerrors=$(grep -c -E "(^|[^A-Za-z])ERROR([^A-Za-z]|$)" "$log")
	(( nerrors > 0 )) && reasons+=("$nerrors ERROR line(s)")
	if (( ${#reasons} )); then
		echo "SMOKE FAIL $name (${elapsed}s): ${(j:, :)reasons} -> $log"
		grep -m5 -E "panicked at|AGENT_FATAL|ERROR" "$log" | cut -c1-300 | sed 's/^/    /'
		failed+=($name)
	else
		echo "SMOKE ok   $name (${elapsed}s)"
	fi
done
if (( ${#failed} )); then
	echo "SMOKE: ${#failed} failed: ${failed[*]}"
	exit 1
fi
echo "SMOKE: all ${#names} passed"
