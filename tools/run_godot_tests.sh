#!/usr/bin/env bash
# Lance chaque game/tests/*_test.gd en headless, l'un après l'autre, et récapitule pass/fail.
# Usage : tools/run_godot_tests.sh [motif ...]   (motif = sous-chaîne du nom ; sans motif : tous)
# Variables : LOG_DIR (journaux par test, défaut /tmp/cent-ans-gdtests), CENT_ANS_TEST_TIMEOUT_S
# (garde-fou par test, défaut 600 s). Code de sortie : 0 si tout passe, 1 sinon.
set -uo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
log_dir="${LOG_DIR:-/tmp/cent-ans-gdtests}"
export CENT_ANS_TEST_TIMEOUT_S="${CENT_ANS_TEST_TIMEOUT_S:-600}"
mkdir -p "$log_dir"

passed=0
failed_names=()
for script in "$root"/game/tests/*_test.gd; do
	name="$(basename "$script" .gd)"
	if [ "$#" -gt 0 ]; then
		match=0
		for pattern in "$@"; do
			[[ "$name" == *"$pattern"* ]] && match=1
		done
		[ "$match" -eq 1 ] || continue
	fi
	started=$SECONDS
	godot --headless --path "$root/game" --script "res://tests/$name.gd" >"$log_dir/$name.log" 2>&1
	code=$?
	if [ "$code" -eq 0 ]; then
		passed=$((passed + 1))
		echo "PASS $name ($((SECONDS - started)) s)"
	else
		failed_names+=("$name")
		echo "FAIL $name (code $code, $((SECONDS - started)) s)"
	fi
done

echo "== ${passed} passés, ${#failed_names[@]} échoués"
for name in "${failed_names[@]+"${failed_names[@]}"}"; do
	echo "  - $name  ($log_dir/$name.log)"
done
[ "${#failed_names[@]}" -eq 0 ]
