#!/bin/zsh
l=$1; G=/Users/jean_hubert/dev/gp-bevy; w=/Users/jean_hubert/dev/gp-bm-$l
git -C $G merge-base --is-ancestor bm/$l bm/merge8 || { echo "NOT ANCESTOR $l"; exit 1; }
[ -z "$(git -C $w status --porcelain --untracked-files=no)" ] || { echo "DIRTY $l"; git -C $w status --porcelain --untracked-files=no | head; exit 1; }
git -C $G worktree remove --force $w && git -C $G branch -D bm/$l
