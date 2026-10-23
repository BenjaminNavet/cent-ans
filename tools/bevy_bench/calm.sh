#!/bin/zsh
# Sourced by bench.sh / ab.sh (ADR 0774): a perf measurement is only worth something on a quiet
# machine. `wait_calm` blocks until the 1-minute load average is below CALM_LOAD (default 15) and no
# other `cent-ans` client runs, and logs what it waits for (one line per CALM_LOG_EVERY seconds).
# Env: CALM_LOAD=15  CALM_MAX_WAIT=1800 (seconds; then return 3)  CALM_LOG_EVERY=30  CALM_SKIP=1
# `heavy_run <command...>` runs a command through tools/heavy.sh (shared slots), else directly.
# The caller sets `calm_root` (repo root) before sourcing.

heavy_run() {
	local heavy="${HEAVY_SH:-${calm_root:?}/tools/heavy.sh}"
	if [[ -x $heavy ]]; then "$heavy" "$@"; else "$@"; fi
}

wait_calm() {
	[[ ${CALM_SKIP:-0} == 1 ]] && return 0
	local limit=${CALM_LOAD:-15} max=${CALM_MAX_WAIT:-1800} every=${CALM_LOG_EVERY:-30}
	local waited=0 load clients busy
	while true; do
		load=$(uptime | sed -E 's/.*load averages?: *([0-9.,]+).*/\1/' | tr ',' '.')
		clients=$(pgrep -f "CentAnsBevy.app/Contents/MacOS/cent-ans" | wc -l | tr -d ' ')
		busy=$(awk -v l="$load" -v m="$limit" 'BEGIN { print (l >= m) ? 1 : 0 }')
		if (( busy == 0 && clients == 0 )); then
			echo "calm: ok after ${waited}s (load $load < $limit, no other client)" >&2
			return 0
		fi
		if (( waited >= max )); then
			echo "calm: still busy after ${waited}s (load $load, clients $clients): giving up" >&2
			return 3
		fi
		if (( waited % every == 0 )); then
			echo "calm: waiting ${waited}s/${max}s (load $load, limit $limit; other cent-ans clients: $clients)" >&2
		fi
		sleep 5
		waited=$((waited + 5))
	done
}
