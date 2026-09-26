#!/bin/bash
# Prints one line per cleanup candidate that is READY (idle, merged, no process).
# Format: READY <kind> <path> <size>   kind = worktree | target
MAIN=/Users/jean_hubert/dev/game_project
cd "$MAIN" || exit 1
now=$(date +%s)

# cwd of every running process (once)
cwds=$(lsof -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | sort -u)

idle_minutes() { # newest file mtime under $1, ignoring build caches
  local newest
  newest=$(find "$1" \( -name target -o -name .godot -o -name .git \) -prune -o -type f -newermt '-1 day' -print0 2>/dev/null \
    | xargs -0 stat -f %m 2>/dev/null | sort -n | tail -1)
  echo $(( (now - ${newest:-0}) / 60 ))
}

git worktree list --porcelain | awk '/^worktree /{p=$2} /^locked/{l[p]=1} /^$/{print p"\t"(p in l?1:0)}' | while IFS=$'\t' read -r p locked; do
  [ "$p" = "$MAIN" ] && continue
  [ -d "$p" ] || continue
  grep -q "^$p" <<<"$cwds" && continue
  head=$(git -C "$p" rev-parse HEAD 2>/dev/null) || continue
  git merge-base --is-ancestor "$head" main || continue
  [ -z "$(git -C "$p" status --porcelain --untracked-files=no)" ] || continue
  need=30; [ "$locked" = 1 ] && need=60
  [ "$(idle_minutes "$p")" -ge $need ] || continue
  echo "READY worktree $p $(du -sh "$p" 2>/dev/null | cut -f1)"
done

# main's cargo target: no cargo/rustc anywhere and untouched for 30 min
if ! pgrep -q -x cargo && ! pgrep -q -x rustc && [ -d "$MAIN/core/target" ]; then
  newest=$(find "$MAIN/core/target" -maxdepth 3 -newermt '-30 minutes' -print -quit 2>/dev/null)
  [ -z "$newest" ] && echo "READY target $MAIN/core/target $(du -sh "$MAIN/core/target" | cut -f1)"
fi
