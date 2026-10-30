#!/bin/zsh
# Warm a fresh worktree's cargo build dirs from the main checkout with APFS copy-on-write clones
# (cp -c: instant, no extra disk until files diverge). Each worktree keeps its own target dir;
# only dependency artifacts are reused, workspace crates rebuild (their paths differ).
# Usage: tools/warm_worktree.sh [<worktree path>]   (default: current directory)
set -e
MAIN=${CENT_ANS_MAIN:-/Users/jean_hubert/dev/game_project}
WT=${1:-$PWD}
WT=${WT:A}
[[ $WT == ${MAIN:A} ]] && { echo "warm_worktree: refusing to warm the main checkout"; exit 1; }
for crate in core client; do
  src=$MAIN/$crate/target/debug
  dst=$WT/$crate/target/debug
  [[ -d $src ]] || continue
  if [[ -e $dst/deps ]]; then echo "warm_worktree: $crate/target/debug already present, skipped"; continue; fi
  mkdir -p $dst
  # incremental/ only serves workspace crates, which rebuild anyway: not worth cloning.
  for entry in deps build .fingerprint; do
    [[ -e $src/$entry ]] && cp -c -R -p $src/$entry $dst/
  done
  echo "warm_worktree: $crate/target/debug cloned"
done
