#!/bin/zsh
# Banc du lot DN-FORET : panoramique de la carte (fenêtre en arrière-plan) à une distance donnée.
# Usage : tools/dn_forest_bench.sh <distance> [options Godot du jeu, ex. --no-dn-trees]
# Imprime p50 / p99 de l'image, appels de dessin et primitives.
set -u
cd "${0:A:h}/.." || exit 1
distance=$1
shift
tools/godot_bg.sh --path game res://scenes/campaign_map.tscn -- --stage=map --hide-armies --bench-map \
	--bench-pan-only --bench-distance=$distance --bench-seconds=12 "$@" 2>&1 | grep "bench_map" | head -1 | python3 -c "
import json, re, sys
line = sys.stdin.read()
match = re.search(r'bench_map (\{.*\})', line)
if not match:
    print('no bench line')
    sys.exit(1)
data = json.loads(match.group(1))
keys = ('fps_avg', 'frame_ms_p50', 'frame_ms_p99', 'draw_calls_p50', 'primitives_p50')
print({k: data[k] for k in keys if k in data})
"
