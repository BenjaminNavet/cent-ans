#!/bin/zsh
# Free disk regularly while many agent worktrees build in parallel.
# Every pass: when free space is below the threshold, delete the cargo target/ dirs of agent worktrees
# whose build has been idle for IDLE_MIN minutes (no cargo/rustc process in them, no recent write),
# then the main checkout's client incremental cache. Branches and sources are never touched.
# Usage: tools/disk_guard.sh [--once]   env: DISK_GUARD_MIN_GB (default 150), DISK_GUARD_IDLE_MIN (45),
#        DISK_GUARD_EVERY_S (1200). Log: out/disk-guard.log
set -u
MAIN=${CENT_ANS_MAIN:-/Users/jean_hubert/dev/game_project}
MIN_GB=${DISK_GUARD_MIN_GB:-150}
IDLE_MIN=${DISK_GUARD_IDLE_MIN:-45}
EVERY=${DISK_GUARD_EVERY_S:-1200}
LOG=$MAIN/out/disk-guard.log
mkdir -p $MAIN/out

free_gb() { df -g / | awk 'NR==2 {print $4}' }

busy() {  # any process (cargo, a bench binary, a test script...) runs from or inside $1
  ps -axo command= 2>/dev/null | grep -v disk_guard | grep -qF -- "$1" && return 0
  lsof -a -d cwd 2>/dev/null | grep -qF -- "$1"
}

pass() {
  local before=$(free_gb)
  (( before >= MIN_GB )) && { echo "$(date +%F' '%H:%M) free ${before} G >= ${MIN_GB} G, nothing to do" >> $LOG; return }
  for wt in $MAIN/.claude/worktrees/*(N/); do
    for t in $wt/core/target $wt/client/target; do
      [[ -d $t ]] || continue
      # recent write anywhere in the build dir → still in use
      if [[ -n $(find $t -maxdepth 3 -mmin -${IDLE_MIN} -print -quit 2>/dev/null) ]]; then continue; fi
      # agent still editing sources in this worktree → keep its builds
      if [[ -n $(find $wt -maxdepth 5 -path '*/target' -prune -o -type f -mmin -${IDLE_MIN} -print -quit 2>/dev/null) ]]; then continue; fi
      busy $wt && continue
      [[ -n ${DRY:-} ]] && { echo "would remove $t"; continue; }
      rm -rf -- $t && echo "$(date +%F' '%H:%M) removed $t" >> $LOG
    done
  done
  local inc=$MAIN/client/target/debug/incremental
  if [[ -d $inc && -z ${DRY:-} ]] && ! busy $MAIN/client; then rm -rf -- $inc && echo "$(date +%F' '%H:%M) removed $inc" >> $LOG; fi
  echo "$(date +%F' '%H:%M) free ${before} G -> $(free_gb) G" >> $LOG
}

if [[ ${1:-} == --once ]]; then pass; exit 0; fi
while true; do pass; sleep $EVERY; done
