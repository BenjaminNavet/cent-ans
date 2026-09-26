#!/bin/bash
# Builds the one-line cleanup command from the current READY list.
S=$(dirname "$0")
MAIN=/Users/jean_hubert/dev/game_project
parts=("cd $MAIN")
while read -r kind path _; do
  case $kind in
    worktree)
      branch=$(git -C "$path" symbolic-ref --short -q HEAD)
      parts+=("git worktree unlock $path 2>/dev/null; git worktree remove --force $path")
      [ -n "$branch" ] && parts+=("git branch -d $branch") ;;
    target)
      parts+=("rm -rf $path") ;;
  esac
done < <("$S/cleanup_status.sh" | awk '{print $2, $3}')
parts+=("git worktree prune" "df -h /System/Volumes/Data | tail -1")
( IFS=';'; echo "! ${parts[*]}" ) | sed 's/;/; /g'
