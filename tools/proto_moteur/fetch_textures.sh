#!/usr/bin/env bash
# Download the CC0 Poly Haven ground texture (grass_path_2, 2k) used by the prototype.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p assets
for map in diff:Diffuse nor_gl:nor_gl rough:Rough; do
	curl -sfL -o "assets/grass_path_2_${map#*:}.jpg" \
		"https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/grass_path_2/grass_path_2_${map%%:*}_2k.jpg"
done
