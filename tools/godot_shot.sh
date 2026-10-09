#!/bin/zsh
# Lance Godot dans un conteneur Linux (Xvfb + rendu logiciel Mesa) : aucune fenêtre sur le Mac,
# donc aucun vol de focus, même pendant que le joueur joue (chantier CF). Pour les
# captures `*_shot.gd`, sondes et bancs qui demandent un vrai rendu ; même interface que
# tools/godot_bg.sh, mais le code de sortie de Godot est transmis.
# Usage : tools/godot_shot.sh [--no-build] --path game --script res://tests/as5_shot.gd
# - user:// est celui du jeu sur l'hôte (les captures arrivent au même endroit qu'avant ; les
#   chemins du conteneur sont réécrits en chemins hôte dans le journal) ; caches de shaders à part.
# - `.godot` du conteneur : copie du `.godot` de l'hôte (clone APFS puis rsync), jamais partagée.
# - GDExtension Linux compilée dans le conteneur (core/build.sh, cible dans ~/.cache), sauf
#   --no-build. Rendu lent (processeur) : bon pour juger une UI ou une composition, pas un bench.
# Variables : CENT_ANS_SHOT_SCREEN (défaut 1920x1080), CENT_ANS_SHOT_DRIVER (vulkan | opengl3).
set -eu -o pipefail

build=1
godot_args=()
game_dir=""
while (( $# )); do
	case "$1" in
		--no-build) build=0; shift ;;
		--path)
			(( $# >= 2 )) || { echo "godot_shot: --path sans valeur" >&2; exit 2; }
			game_dir=${2:A}; shift 2 ;;
		*) godot_args+=("$1"); shift ;;
	esac
done
[[ -n $game_dir ]] || game_dir=${PWD:A}/game
[[ -f $game_dir/project.godot ]] || { echo "godot_shot: pas de project.godot dans $game_dir" >&2; exit 2; }
repo=$(git -C "$game_dir" rev-parse --show-toplevel)
game_rel=${game_dir#$repo/}

if ! docker info >/dev/null 2>&1; then
	echo "godot_shot: démarrage de Docker Desktop (peut prendre le focus ~1 s, une seule fois)" >&2
	docker desktop start --timeout 180 >&2
fi

godot_version=$(godot --version | sed -E 's/^([0-9.]+)\.stable.*/\1/')
image="cent-ans-shot:$godot_version"
if ! docker image inspect "$image" >/dev/null 2>&1; then
	echo "godot_shot: construction de l'image $image (une fois)" >&2
	docker build -q --build-arg GODOT="$godot_version" -t "$image" "$repo/tools/godot_shot" >&2
fi

cache=$HOME/.cache/cent-ans-shot/$(print -rn -- "$game_dir" | shasum | cut -c1-12)
mkdir -p "$cache/target" "$cache/cargo-registry" "$cache/shader_cache" "$cache/vulkan"
# Le .godot de l'hôte sert de point de départ ; les imports propres au conteneur s'y ajoutent.
if [[ ! -d $cache/godot ]]; then
	[[ -d $game_dir/.godot ]] && cp -cR "$game_dir/.godot" "$cache/godot" || mkdir -p "$cache/godot"
elif [[ -d $game_dir/.godot ]]; then
	rsync -a "$game_dir/.godot/" "$cache/godot/"
fi

user_host="$HOME/Library/Application Support/Godot/app_userdata/Cent Ans"
user_ct="/root/.local/share/godot/app_userdata/Cent Ans"
mkdir -p "$user_host"

steps=()
(( build )) && steps+=("CARGO_TARGET_DIR=/cache/target /repo/core/build.sh >&2")
steps+=('exec xvfb-run -a -s "-screen 0 ${SCREEN}x24" godot --rendering-driver "$DRIVER" --path "/repo/$GAME_REL" "$@"')

docker run --rm --init \
	-e SCREEN="${CENT_ANS_SHOT_SCREEN:-1920x1080}" \
	-e DRIVER="${CENT_ANS_SHOT_DRIVER:-vulkan}" \
	-e GAME_REL="$game_rel" \
	-e CENT_ANS_MAX_ERRORS -e CENT_ANS_TEST_TIMEOUT_S \
	-v "$repo:/repo" \
	-v "$cache/godot:/repo/$game_rel/.godot" \
	-v "$cache/target:/cache/target" \
	-v "$cache/cargo-registry:/usr/local/cargo/registry" \
	-v "$user_host:$user_ct" \
	-v "$cache/shader_cache:$user_ct/shader_cache" \
	-v "$cache/vulkan:$user_ct/vulkan" \
	"$image" bash -c "set -e; ${(j:; :)steps}" godot_shot "${godot_args[@]}" 2>&1 \
	| sed -e "s#$user_ct#$user_host#g" -e "s#/repo/#$repo/#g"
