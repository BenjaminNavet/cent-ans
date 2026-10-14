#!/bin/zsh
# Lance Godot en fenêtre SANS prendre le focus (macOS : `open -g -j`, application cachée, fenêtre
# hors écran, sans son ; `open -g` seul coupait le focus du joueur le 14/10, et `-j` aussi lors
# d'une série de lancements le même jour : Godot macOS s'active lui-même au démarrage. À ne lancer
# que si le joueur est d'accord pour être interrompu), pour les bancs, captures et
# sondes lancés par un agent pendant que le joueur travaille. Attend la fin, puis écrit le journal
# de Godot sur la sortie standard (filtrable par grep comme une sortie directe).
# Usage : tools/godot_bg.sh --path game res://scenes/campaign_map.tscn -- --bench-map ...
# `--path` relatif est résolu depuis le répertoire courant (`open` lance depuis /).
# Limite : le code de sortie de Godot n'est pas transmis ; les tests headless n'ouvrent pas de
# fenêtre et se lancent directement avec `godot --headless`.
set -u
args=()
has_position=0
while (( $# )); do
	if [[ "$1" == "--path" && $# -ge 2 ]]; then
		args+=("--path" "${2:A}")
		shift 2
	else
		[[ "$1" == "--position" ]] && has_position=1
		args+=("$1")
		shift
	fi
done
godot_bin=$(readlink -f "$(command -v godot)")
app=${godot_bin%%/Contents/*}
log=$(mktemp -t godot_bg)
pos=()
(( has_position )) || pos=(--position 4000,4000)
open -g -j -n -W -a "$app" --args --log-file "$log" --audio-driver Dummy "${pos[@]}" "${args[@]}"
cat "$log"
rm -f "$log"
