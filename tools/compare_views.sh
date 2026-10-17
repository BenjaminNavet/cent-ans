#!/bin/zsh
# Jeu de vues de référence du client Bevy (lot refshots ; le côté Godot a été retiré, ADR 0366) :
# capture chaque vue de tools/refshots/views.txt puis écrit une planche par vue dans
# <out>/montage/ (avec <out>/reference/<vue>.png à gauche s'il existe). Images jamais commitées.
#
# Usage : tools/compare_views.sh [--out <dossier>] [--only <vue>] [--bevy-window] [--montage-only]
# Env :   BEVY_CLIENT  dossier du client Bevy (défaut : <dépôt>/client)
#
# Chaque image est vérifiée (non uniforme) et la vue relancée une fois sinon. Bevy rend hors
# écran (--offscreen) sauf --bevy-window. Préférer les dumps d'état aux captures (CLAUDE.md).
set -u
here=${0:A:h}
repo=${here:h}
views_file=$here/refshots/views.txt
out=${TMPDIR:-/tmp}/cent_ans_refshots
only=""
bevy_window=0
montage_only=0
while (( $# )); do
	case "$1" in
	--out) out=${2:A}; shift 2 ;;
	--only) only=$2; shift 2 ;;
	--bevy-window) bevy_window=1; shift ;; # explicit opt-in only (steals focus)
	--montage-only) montage_only=1; shift ;;
	*) echo "compare_views: option inconnue $1" >&2; exit 2 ;;
	esac
done
bevy_client=${BEVY_CLIENT:-$repo/client}
mkdir -p $out/bevy $out/logs
check() { python3 $here/refshots/montage.py check "$1"; }

# Vues : "nom genre args…" (lignes non vides, sans commentaire).
typeset -a names
while IFS= read -r line; do
	[[ -z "${line// }" || $line == \#* ]] && continue
	name=${line%% *}
	[[ -n $only && $name != $only ]] && continue
	names+=($name)
done < $views_file
view_line() { grep -E "^$1 " $views_file; }

# --- Bevy -------------------------------------------------------------------------------------
bevy_view() { # nom
	local name=$1
	local -a parts=(${=$(view_line $name)})
	local kind=${parts[2]} script=$out/logs/bevy_$name.txt shot=$out/bevy/$name.png
	local -a args=(--seed 1337 --faction fac_france --script $script)
	(( bevy_window )) && args+=(--window)
	local camera="" units_view=""
	case $kind in
	campaign)
		camera="${parts[3]},${parts[4]},${parts[5]}"
		args+=(--state Campaign)
		printf 'wait 600\nshot %s\nwait 5\nexit\n' $shot > $script
		;;
	battle)
		args+=(--state Battle)
		[[ ${parts[3]} == low ]] && units_view="0,${parts[4]}"
		printf 'click speed:pause\nwait 300\nshot %s\nwait 5\nexit\n' $shot > $script
		;;
	siege)
		args+=(--state MainMenu)
		printf 'wait 60\nclick Batailles\nwait 10\nclick Batailles de démonstration\nwait 30\nclick Demo_%s\nclick speed:pause\nwait 300\nshot %s\nwait 5\nexit\n' \
			${parts[3]} $shot > $script
		;;
	esac
	rm -f $shot
	(
		export CENT_ANS_SETTINGS_PATH=$out/logs/bevy_settings.json CENT_ANS_PREFS=$out/logs/prefs \
			CENT_ANS_SAVES=$out/logs/saves CENT_ANS_REPLAYS=$out/logs/replays CENT_ANS_TUTORIAL=off \
			CENT_ANS_MUTE=1 CENT_ANS_NO_DEPLOY=1 CENT_ANS_NO_SPEECH=1 CENT_ANS_NO_BRIEFING=1 \
			CENT_ANS_LOADING_SECONDS=0
		[[ -n $camera ]] && export CENT_ANS_CAMERA=$camera
		[[ -n $units_view ]] && export CENT_ANS_UNITS_VIEW=$units_view
		$bevy_client/run_bg.sh "${args[@]}"
	) > $out/logs/bevy_$name.log 2>&1
}
run_bevy() {
	local n
	for n in $names; do
		echo "bevy: $n"
		bevy_view $n
		if ! check $out/bevy/$n.png; then
			echo "bevy: $n vide, nouvel essai"
			bevy_view $n
			check $out/bevy/$n.png || echo "bevy: $n ÉCHEC (voir logs/bevy_$n.log)"
		fi
	done
}

if (( ! montage_only )); then
	run_bevy
fi
python3 $here/refshots/montage.py montage $out $names
