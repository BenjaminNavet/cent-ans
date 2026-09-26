#!/usr/bin/env bash
# EP1 (ADR 0076): A/B battle benchmark at massive scale. Prints one BENCH_JSON line per run.
# Usage: tools/bench_ep1.sh <label> <extra args after --...>
# Example: tools/bench_ep1.sh budget --units=63 --bench-at=90
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
label="$1"
shift
out="$(godot --path "$ROOT/game" --disable-vsync --resolution 1600x900 res://scenes/battle/battle.tscn -- \
	--benchmark --quality=high --bench-timeout=600 "$@" 2>&1)"
line="$(grep '^BENCH_JSON' <<<"$out" | tail -1)"
echo "$label $* :: ${line#BENCH_JSON }"
