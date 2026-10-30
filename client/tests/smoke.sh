#!/bin/zsh
# Smoke tests of the Bevy client: every scenario of tests/smoke/*.txt runs in the background
# window of `client/run_bg.sh` (open -g, no focus), and fails on a panic, an AGENT_FATAL, an ERROR
# log line, a missing `AGENT_EXIT script|bench` marker (crash without panic, hang, safety net fired), a
# watchdog timeout, or a dump whose `state` is not the one the scenario must reach (`scenario_expect`:
# a battle that never started left the client on the menu and used to pass). ~4 minutes for the nine scenarios once the debug build is warm.
#
# Usage: client/tests/smoke.sh [scenario ...]        (default: all)
# Env:   SMOKE_PROFILE=debug|release   (default debug: overflow checks and debug assertions on)
#        SMOKE_TIMEOUT=<seconds>       per scenario (default 240)
#        SMOKE_UNIT=1                  run the window-less tests first (tests/test.sh: nextest, else cargo test)
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
# Own preferences folder: the first battle launched from the menu asks « Jouer le didacticiel de
# bataille ? » (battle_prologue_invite) and waits for an answer, depending on the player's real
# `battle_prologue.json`. The smoke must not depend on it: invitation refused for good.
prefs=$(mktemp -d "${TMPDIR:-/tmp}/cent-ans-smoke-prefs.XXXXXX")
echo '{"never_ask": true, "done": false}' > "$prefs/battle_prologue.json"
export CENT_ANS_PREFS=$prefs
trap 'rm -rf "$prefs"' EXIT

# scenario -> extra environment (one line `NAME=value ...`) and CLI arguments.
typeset -A scenario_env scenario_args
scenario_env[campaign_battle]="CENT_ANS_STAGE_BATTLE=1 CENT_ANS_FLOW_SIM_TO_END=1 CENT_ANS_LOADING_SECONDS=0"
scenario_args[campaign_turns]="--faction fac_england"
scenario_args[campaign_attention]="--faction fac_england"
scenario_args[campaign_audience]="--faction fac_england"
scenario_args[campaign_descent]="--bench-map --bench-descent-only"

# scenario -> `dump-file=State` pairs: the final state each dump must report (AppState name).
typeset -A scenario_expect
scenario_expect[battle]="battle=Battle"
scenario_expect[crecy]="crecy=Battle"
scenario_expect[siege]="siege=Battle"
scenario_expect[campaign]="campaign=Campaign campaign_panels=Campaign"
scenario_expect[campaign_battle]="campaign_battle_after=Campaign campaign_battle_turn=Campaign"
scenario_expect[campaign_turns]="turns_0=Campaign turns_1=Campaign"
scenario_expect[campaign_attention]="attention_0=Campaign attention_10=Campaign"
scenario_expect[campaign_audience]="audience_card=Campaign audience_rest=Campaign"
scenario_expect[codex]="codex=Codex"
scenario_expect[faction_select]="faction_select=FactionSelect"
scenario_expect[faction_works]="works_0=Campaign works_1=Campaign works_2=Campaign"
scenario_expect[main_menu]="main_menu=MainMenu"

# scenario -> `dump-file:section.path` pairs whose JSON value must be non-empty (XP-POURQUOI: the
# Récit of the season must already explain something on the third turn).
typeset -A scenario_nonempty
scenario_nonempty[campaign_turns]="turns_1:season_story.story.why"
scenario_nonempty[campaign_audience]="audience_card:agenda.agenda.affairs"

# scenario -> `dump-file:section.path=N` pairs whose JSON list must hold at most N items (XP-AUDIENCE:
# never more than three Affairs pushed at once).
typeset -A scenario_maxlen
scenario_maxlen[campaign_attention]="attention_0:agenda.agenda.affairs=3 attention_6:agenda.agenda.affairs=3 attention_10:agenda.agenda.affairs=3"
scenario_maxlen[campaign_audience]="audience_card:agenda.agenda.affairs=3 audience_rest:agenda.agenda.affairs=3"

if (( $# )); then names=("$@"); else names=(${(f)"$(cd "$here/smoke" && ls *.txt | sed 's/\.txt$//')"}); fi

if [[ ${SMOKE_UNIT:-0} == 1 ]]; then
	"$here/test.sh" >&2 || { echo "SMOKE unit tests failed"; exit 1; }
fi

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
	# A stale dump of an earlier run must not satisfy the state check.
	for pair in ${=scenario_expect[$name]:-}; do rm -f "$root/out/smoke/${pair%%=*}.json"; done
	(
		cd "$root"
		for pair in ${=scenario_env[$name]:-}; do export "$pair"; done
		exec "$client/run_bg.sh" --state MainMenu ${=scenario_args[$name]:-} \
			--exit-after 6000 --script "$script"
	) > "$log" 2>&1 &
	pid=$!
	# Watchdog: `open -W` hides the app's status and can hang with it; kill after the limit.
	( sleep $limit; pkill -f "CentAnsBevy.app/Contents/MacOS/cent-ans"; kill $pid ) >/dev/null 2>&1 &
	dog=$!
	wait $pid
	code=$?
	pkill -P $dog 2>/dev/null; kill $dog 2>/dev/null; wait $dog 2>/dev/null
	elapsed=$((SECONDS - start))
	reasons=()
	(( code != 0 )) && reasons+=("run_bg exit $code")
	grep -q "panicked at" "$log" && reasons+=("panic")
	grep -q "AGENT_FATAL" "$log" && reasons+=("AGENT_FATAL")
	# A wgpu shader error does not stop the game: black ground with no other symptom (ADR 0774).
	grep -q -E "failed to create shader module|no definition in scope|Shader compilation error" "$log" && reasons+=("shader error")
	grep -q -E "AGENT_EXIT (script|bench)" "$log" || reasons+=("script did not reach exit")
	nerrors=$(grep -c -E "(^|[^A-Za-z])ERROR([^A-Za-z]|$)" "$log")
	(( nerrors > 0 )) && reasons+=("$nerrors ERROR line(s)")
	for pair in ${=scenario_expect[$name]:-}; do
		dump="$root/out/smoke/${pair%%=*}.json"
		want=${pair#*=}
		got=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("state"))' "$dump" 2>/dev/null) || got="no dump"
		[[ $got == $want ]] || reasons+=("${pair%%=*}.json: state $got, expected $want")
	done
	for pair in ${=scenario_nonempty[$name]:-}; do
		dump="$root/out/smoke/${pair%%:*}.json"
		jpath=${pair#*:}
		python3 -c '
import json, sys
node = json.load(open(sys.argv[1]))
for key in sys.argv[2].split("."):
    node = node[key]
sys.exit(0 if node else 1)' "$dump" "$jpath" 2>/dev/null || reasons+=("${pair%%:*}.json: ${jpath} is empty or missing")
	done
	for pair in ${=scenario_maxlen[$name]:-}; do
		dump="$root/out/smoke/${pair%%:*}.json"
		rest=${pair#*:}
		jpath=${rest%=*}
		limit_items=${rest##*=}
		python3 -c '
import json, sys
node = json.load(open(sys.argv[1]))
for key in sys.argv[2].split("."):
    node = node[key] if node else []
sys.exit(0 if node is None or len(node) <= int(sys.argv[3]) else 1)' "$dump" "$jpath" "$limit_items" 2>/dev/null || reasons+=("${pair%%:*}.json: ${jpath} holds more than ${limit_items} items")
	done
	if (( ${#reasons} )); then
		echo "SMOKE FAIL $name (${elapsed}s): ${(j:, :)reasons} -> $log"
		grep -m5 -E "panicked at|AGENT_FATAL|ERROR|shader module|no definition in scope|Shader compilation error" "$log" | cut -c1-300 | sed 's/^/    /'
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
