#!/bin/zsh
# Usage: bm_wt.sh <lot> — worktree ../gp-bm-<lot> on branch bm/<lot> from bevy-migration,
# with the ignored heavy assets linked and client/target APFS-cloned from gp-bevy.
set -eu
lot=$1; D=/Users/jean_hubert/dev; M=$D/game_project; W=$D/gp-bm-$lot
git -C $D/gp-bevy worktree add -q --no-checkout $W -b bm/$lot bevy-migration
for d in game/assets data; do mkdir -p $W/$(dirname $d); cp -c -R $D/gp-bevy/$d $W/$d; done
git -C $W reset -q && git -C $W checkout -q -- . ":!game/assets" ":!data"
# No target clone (15/10): cloned targets keep absolute dylib paths into gp-bevy (dyld errors) and stale hashes; sccache makes a fresh build ~5 min.
echo $W
