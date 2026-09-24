#!/bin/bash
# Rebuild the Poly Haven vegetation GLBs of game/assets/third_party/vegetation/.
# Usage: polyhaven_vegetation.sh <work_dir>
#   <work_dir>/<asset>/ must hold the Poly Haven .blend + textures/ (API: api.polyhaven.com/files/<asset>),
#   grass at 2k, trees at 1k. Needs blender and ImageMagick (magick).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="$1"
OUT="$HERE/../../game/assets/third_party/vegetation"
TREE_MAX_TRIS=30000

prep_tree() {
  local t=$1 src=$WORK/$1/textures out=$WORK/$1/prep
  mkdir -p "$out"
  magick "$src/${t}_twig_diff_1k.png" "$src/${t}_twig_alpha_1k.png" -alpha off -compose CopyOpacity -composite -depth 8 "$out/${t}_twig_diff_alpha.png"
  magick "$src/${t}_twig_nor_gl_1k.png" -depth 8 -quality 92 "$out/${t}_twig_nor_gl.jpg"
  for p in bark trunk_a trunk_b trunk_c; do
    magick "$src/${t}_${p}_diff_1k.png" -depth 8 -quality 90 "$out/${t}_${p}_diff.jpg"
    magick "$src/${t}_${p}_nor_gl_1k.png" -depth 8 -quality 92 "$out/${t}_${p}_nor_gl.jpg"
  done
}

prep_grass() {
  local g=$1 src=$WORK/$1/textures out=$WORK/$1/prep
  mkdir -p "$out"
  magick "$src/${g}_diff_2k.jpg" "$src/${g}_alpha_2k.png" -alpha off -compose CopyOpacity -composite -resize 1024x1024 -depth 8 "$out/${g}_diff_alpha.png"
}

convert() {
  local blend=$1 asset=$2 dest=$3 max=$4 mask=$5
  shift 5
  mkdir -p "$(dirname "$dest")"
  blender -b "$blend" --python "$HERE/polyhaven_vegetation.py" -- "$WORK/$asset/prep" "$asset" "$dest" "$max" "$@" 2>&1 | grep -E "REPORT|rror" || true
  python3 "$HERE/glb_alpha_mask.py" "$dest" "$mask"
}

prep_grass grass_medium_01
prep_grass grass_medium_02
convert "$WORK/grass_medium_02/grass_medium_02_2k.blend" grass_medium_02 "$OUT/grass_medium_02/grass_medium_02.glb" 0 grass \
  grass_medium_02_a grass_medium_02_b grass_medium_02_c grass_medium_02_d grass_medium_02_e
convert "$WORK/grass_medium_01/grass_medium_01_2k.blend" grass_medium_01 "$OUT/grass_medium_01/grass_medium_01.glb" 0 grass \
  grass_medium_01_geonodes_large_a_LOD2 grass_medium_01_geonodes_large_b_LOD2 grass_medium_01_geonodes_large_c_LOD2 \
  grass_medium_01_geonodes_mid_a_LOD2 grass_medium_01_geonodes_mid_b_LOD2 grass_medium_01_geonodes_mid_c_LOD2 \
  grass_medium_01_geonodes_small_a_LOD2 grass_medium_01_geonodes_small_b_LOD2 \
  grass_medium_01_geonodes_tall_a_LOD2 grass_medium_01_geonodes_tall_b_LOD2

prep_tree fir_tree_01
convert "$WORK/fir_tree_01/fir_tree_01_1k.blend" fir_tree_01 "$OUT/fir_tree_01/fir_tree_01.glb" $TREE_MAX_TRIS twig \
  fir_tree_01_a_LOD2 fir_tree_01_b_LOD2 fir_tree_01_c_LOD2
prep_tree pine_tree_01
convert "$WORK/pine_tree_01/pine_tree_01_1k.blend" pine_tree_01 "$OUT/pine_tree_01/pine_tree_01.glb" $TREE_MAX_TRIS twig \
  pine_tree_01_a_LOD2 pine_tree_01_b_LOD2 pine_tree_01_c_LOD2
ls -la "$OUT"/*/
