#!/usr/bin/env bash
# Build every prototype asset: ground texture and mesh, plain-PBR copies of the game models, scene.json.
set -euo pipefail
cd "$(dirname "$0")"
REPO=$(cd ../.. && pwd)
BLENDER=${BLENDER:-/Applications/Blender.app/Contents/MacOS/Blender}
[ -f assets/grass_path_2_Diffuse.jpg ] || ./fetch_textures.sh
"$BLENDER" -b --python make_ground.py > /dev/null
python3 make_scene.py
args=()
while read -r name path; do
	case "$name" in
		fir_tree_01|grass_medium_01) cp "$REPO/$path" "assets/$name.glb" ;;
		*) args+=("$name" "$REPO/$path" "$PWD/assets/$name.glb") ;;
	esac
done < sources.txt
"$BLENDER" -b --python materialize.py -- "${args[@]}" 2>&1 | grep -E "PROTO|Error"
