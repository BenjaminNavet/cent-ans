#!/bin/zsh
# Foreground variant of tools/godot_bg.sh for the night bench (player away, windows allowed
# 14/10): the Godot window is on screen and frontmost so macOS never treats it as occluded.
# Muted audio; the log is printed on stdout when Godot exits (no exit code).
set -u
args=()
while (( $# )); do
	if [[ "$1" == "--path" && $# -ge 2 ]]; then
		args+=("--path" "${2:A}")
		shift 2
	else
		args+=("$1")
		shift
	fi
done
godot_bin=$(readlink -f "$(command -v godot)")
app=${godot_bin%%/Contents/*}
log=$(mktemp -t godot_fg)
open -n -W -a "$app" --args --log-file "$log" --audio-driver Dummy --position 0,30 "${args[@]}"
cat "$log"
rm -f "$log"
