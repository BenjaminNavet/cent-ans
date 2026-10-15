#!/bin/zsh
# Schedule-shuffling check (ADR 0364, dev builds only): runs smoke scenarios with the system order
# of every schedule randomised (`CENT_ANS_SHUFFLE_SEED`, constraints kept, single-threaded executor,
# so a seed reproduces its order). A failure means a missing ordering constraint between systems.
# Offscreen and muted: no window, no sound. Compare with the unshuffled run (seed 0 = off).
#
# Usage: client/tests/shuffle.sh [-s "1 2 3"] [scenario ...]   (default scenarios: main_menu campaign battle codex faction_select)
# Env:   SHUFFLE_SEEDS="1 2 3"  SHUFFLE_TIMEOUT=<s per run, default 150>  SHUFFLE_LOGS=<dir, default out/shuffle>
# Exit code: 0 all passed, 1 at least one failed.
set -u
here=${0:A:h}
client=${here:h}
root=${client:h}
seeds=(${=SHUFFLE_SEEDS:-1 2 3})
limit=${SHUFFLE_TIMEOUT:-150}
logs=${SHUFFLE_LOGS:-$root/out/shuffle}
mkdir -p "$logs"
if [[ ${1:-} == -s ]]; then seeds=(${=2}); shift 2; fi
names=("$@")
(( ${#names} )) || names=(main_menu campaign battle codex faction_select)

export CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug CENT_ANS_MUTE=1 CENT_ANS_NO_BRIEFING=1 CENT_ANS_NO_DEPLOY=1
(cd "$client" && cargo build --features dev) >&2 || { echo "SHUFFLE build failed"; exit 1; }

# scenario -> extra environment (same as smoke.sh)
typeset -A scenario_env
scenario_env[campaign_battle]="CENT_ANS_STAGE_BATTLE=1 CENT_ANS_FLOW_SIM_TO_END=1 CENT_ANS_LOADING_SECONDS=0"

failed=()
for seed in $seeds; do
	for name in $names; do
		script="$here/smoke/$name.txt"
		log="$logs/seed${seed}_$name.log"
		[[ -f $script ]] || { echo "SHUFFLE $name: no such scenario"; failed+=("$name"); continue; }
		(cd "$root" && for pair in ${=scenario_env[$name]:-}; do export "$pair"; done
			CENT_ANS_SHUFFLE_SEED=$seed exec "$client/run_bg.sh" --offscreen --state MainMenu \
			--exit-after 6000 --script "$script") > "$log" 2>&1 &
		pid=$!
		( sleep $limit; pkill -f "CentAnsBevy.app/Contents/MacOS/cent-ans"; kill $pid ) >/dev/null 2>&1 &
		dog=$!
		wait $pid; code=$?
		pkill -P $dog 2>/dev/null; kill $dog 2>/dev/null; wait $dog 2>/dev/null
		reasons=()
		(( code != 0 )) && reasons+=("exit $code")
		grep -q "panicked at" "$log" && reasons+=("panic")
		grep -q "AGENT_FATAL" "$log" && reasons+=("AGENT_FATAL")
		grep -q -E "AGENT_EXIT (script|bench)" "$log" || reasons+=("script did not reach exit")
		nerrors=$(grep -c -E "(^|[^A-Za-z])ERROR([^A-Za-z]|$)" "$log")
		(( nerrors > 0 )) && reasons+=("$nerrors ERROR line(s)")
		if (( ${#reasons} )); then
			echo "SHUFFLE FAIL seed=$seed $name: ${(j:, :)reasons} -> $log"
			grep -m3 -E "panicked at|AGENT_FATAL|ERROR" "$log" | cut -c1-300 | sed 's/^/    /'
			failed+=("$seed:$name")
		else
			echo "SHUFFLE ok   seed=$seed $name"
		fi
	done
done
(( ${#failed} )) && { echo "SHUFFLE: ${#failed} failed: ${failed[*]}"; exit 1; }
echo "SHUFFLE: all passed"
