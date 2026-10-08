#!/bin/zsh
# Lance Godot en fenêtre SANS prendre le focus (macOS : `open -g`), pour les bancs, captures et
# sondes lancés par un agent pendant que le joueur travaille. Attend la fin, puis écrit le journal
# de Godot sur la sortie standard (filtrable par grep comme une sortie directe).
# Usage : tools/godot_bg.sh --path game res://scenes/campaign_map.tscn -- --bench-map ...
# `--path` relatif est résolu depuis le répertoire courant (`open` lance depuis /).
# Limite : le code de sortie de Godot n'est pas transmis ; les tests headless n'ouvrent pas de
# fenêtre et se lancent directement avec `godot --headless`.
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
log=$(mktemp -t godot_bg)
open -g -n -W -a "$app" --args --log-file "$log" "${args[@]}"
cat "$log"
rm -f "$log"
