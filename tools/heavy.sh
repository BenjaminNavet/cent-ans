#!/bin/zsh
# Semaphore: at most 3 heavy jobs (cargo build/bench/clippy/test, client runs) at once across all worktrees.
# Usage: heavy.sh <command...>   e.g. heavy.sh cargo check --features dev
# Each job appends one line to $HEAVY_LOG (default: main checkout out/heavy-log.tsv):
# start time, wait s (queue), run s, exit code, worktree, command.
zmodload zsh/datetime
SLOTS=3; DIR=/private/tmp/claude-501/heavy-slots; mkdir -p $DIR
LOG=${HEAVY_LOG:-/Users/jean_hubert/dev/game_project/out/heavy-log.tsv}
queued=$EPOCHSECONDS
wt=$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")
while true; do
  for i in $(seq 1 $SLOTS); do
    if mkdir $DIR/slot$i 2>/dev/null; then
      echo $$ > $DIR/slot$i/pid
      trap "rm -rf $DIR/slot$i" EXIT INT TERM
      started=$EPOCHSECONDS
      "$@"; code=$?
      mkdir -p ${LOG:h} 2>/dev/null
      printf '%s\t%d\t%d\t%d\t%s\t%s\n' "$(date -r $started '+%m-%d %H:%M:%S')" $((started - queued)) \
        $((EPOCHSECONDS - started)) $code "$wt" "${${*}[1,160]}" >> $LOG 2>/dev/null
      exit $code
    fi
    # reclaim slot if its owner died
    if [[ -f $DIR/slot$i/pid ]] && ! kill -0 $(cat $DIR/slot$i/pid) 2>/dev/null; then rm -rf $DIR/slot$i; fi
  done
  sleep 5
done
