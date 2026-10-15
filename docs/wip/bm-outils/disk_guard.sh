#!/bin/zsh
# Every 3 min: if < 40 GB free, drop incremental caches of lot worktrees with no running rustc/cargo. Alert under 15 GB.
while true; do
  f=$(df -g /Users | awk 'NR==2{print $4}')
  if [ "$f" -lt 40 ]; then
    busy=$(ps -axo command | grep -E 'rustc|cargo ' | grep -v grep | grep -oE 'gp-[a-z0-9-]+' | sort -u)
    for d in /Users/jean_hubert/dev/gp-bm-*(N); do
      n=${d:t}
      echo "$busy" | grep -qx "$n" && continue
      D="$d/client/target/debug/incremental"
      [ -d "$D" ] && rm -rf "${D:?}" && echo "$(date +%H:%M) pruned $n (free ${f} GB)"
    done
  fi
  f=$(df -g /Users | awk 'NR==2{print $4}')
  if [ "$f" -lt 8 ]; then echo "LOW DISK: ${f} GB free"; exit 0; fi
  sleep 180
done
