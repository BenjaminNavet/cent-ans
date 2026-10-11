#!/bin/zsh
# T2 with "3D render" source images (ADR 0342): for each chunk of the dn_batch catalog,
# local Z-Image images (2 seeds, heuristic choice) then TRELLIS.2 on the chosen seed.
# Resumable: dn_batch and t2_batch both skip what already exists.
#   tools/experiments/t2_r3d_chain.sh [CHUNK]   (catalog: python3 tools/experiments/t2_batch.py sources)
set -u
cd "$(dirname "$0")/../.."
CATALOG=$HOME/dev/cent-ans-raw/t2/r3d/catalog.json
LOG=$HOME/dev/cent-ans-raw/t2/r3d
CHUNK=${1:-25}
ids=("${(@f)$(python3 -c "import json; [print(e['id']) for e in json.load(open('$CATALOG'))]")}")
for ((start = 1; start <= ${#ids}; start += CHUNK)); do
  chunk=("${(@)ids[start,start+CHUNK-1]}")
  only=${(j:,:)chunk}
  echo "== chunk $start/${#ids} $(date +%H:%M)"
  uv run --with rembg --with onnxruntime --with fal-client --with pillow --with numpy python \
    tools/experiments/dn_batch.py "$CATALOG" --only "$only" --until select --select-best \
    --charter warn --image-backend local --image-workers 1 >> "$LOG/images.log" 2>&1
  ~/dev/cent-ans-raw/trellis-mac/.venv/bin/python tools/experiments/t2_batch.py generate \
    --tag r3d --ids "${only//__r3d/}" 2>&1 | grep -v Sampling >> "$LOG/t2.log"
done
echo "chain over $(date +%H:%M)"
