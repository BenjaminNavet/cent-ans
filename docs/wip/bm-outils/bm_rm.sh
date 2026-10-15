#!/bin/zsh
L=$1; cd /Users/jean_hubert/dev/gp-bevy || exit 1
git merge-base --is-ancestor bm/$L bevy-migration && [ -z "$(git -C ../gp-bm-$L status --porcelain --untracked-files=no)" ] && git worktree remove --force ../gp-bm-$L && git branch -d bm/$L
