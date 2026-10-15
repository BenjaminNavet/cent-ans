#!/bin/zsh
# Jeu de vues de référence Godot | Bevy (lot refshots, migration Bevy) : capture chaque vue de
# tools/refshots/views.txt dans les deux clients puis écrit une planche par vue (Godot à gauche,
# Bevy à droite) dans <out>/montage/. Images jamais commitées.
#
# Usage : tools/compare_views.sh [--out <dossier>] [--side godot|bevy|both] [--only <vue>]
#                                [--bevy-window] [--montage-only]
# Env :   GODOT_GAME   dossier du projet Godot déjà importé (défaut : <dépôt>/game ; un worktree
#                      sans .godot/ doit pointer sur le checkout principal)
#         BEVY_CLIENT  dossier du client Bevy (défaut : <dépôt>/client)
#
# Les deux applications se lancent fenêtrées sans focus (tools/godot_bg.sh, client/run_bg.sh),
# une à la fois : macOS cesse de dessiner une fenêtre couverte (image noire). Chaque image est
# vérifiée (non uniforme) et la vue relancée une fois sinon. Bevy rend hors écran (--offscreen)
# sauf --bevy-window.
set -u
here=${0:A:h}
repo=${here:h}
views_file=$here/refshots/views.txt
out=${TMPDIR:-/tmp}/cent_ans_refshots
side=both
only=""
bevy_window=0
montage_only=0
while (( $# )); do
	case "$1" in
	--out) out=${2:A}; shift 2 ;;
	--side) side=$2; shift 2 ;;
	--only) only=$2; shift 2 ;;
	--bevy-window) bevy_window=1; shift ;; # explicit opt-in only (steals focus)
	--montage-only) montage_only=1; shift ;;
	*) echo "compare_views: option inconnue $1" >&2; exit 2 ;;
	esac
done
godot_game=${GODOT_GAME:-$repo/game}
bevy_client=${BEVY_CLIENT:-$repo/client}
mkdir -p $out/godot $out/bevy $out/logs
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

# --- Godot ------------------------------------------------------------------------------------
godot_group() { # groupe [vue]
	local group=$1 one=${2:-}
	local -a extra=()
	[[ -n $one ]] && extra=(--only=$one)
	$repo/tools/godot_bg.sh --path $godot_game --resolution 1280x800 \
		--script $here/refshots/godot_refshots.gd -- \
		--views-file=$views_file --out=$out/godot --group=$group "${extra[@]}" \
		>> $out/logs/godot_$group.log 2>&1
}
run_godot() {
	local -A groups
	local n g
	for n in $names; do g=$(view_line $n | awk '{print $2}'); groups[$g]=1; done
	for g in ${(k)groups}; do
		echo "godot: groupe $g"; : > $out/logs/godot_$g.log
		for n in $names; do [[ $(view_line $n | awk '{print $2}') == $g ]] && rm -f $out/godot/$n.png; done
		if [[ -n $only ]]; then godot_group $g $only; else godot_group $g; fi
		for n in $names; do
			[[ $(view_line $n | awk '{print $2}') == $g ]] || continue
			if ! check $out/godot/$n.png; then
				echo "godot: $n vide, nouvel essai"
				godot_group $g $n
				check $out/godot/$n.png || echo "godot: $n ÉCHEC (voir logs/godot_$g.log)"
			fi
		done
	done
}

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
	[[ $side == godot || $side == both ]] && run_godot
	[[ $side == bevy || $side == both ]] && run_bevy
fi
python3 $here/refshots/montage.py montage $out $names
