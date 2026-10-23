#!/bin/zsh
# Semaphore: at most 2 heavy jobs (cargo build/bench/clippy/test, client runs) at once across all worktrees.
# Usage: heavy.sh <command...>   e.g. heavy.sh cargo check --features dev
SLOTS=2; DIR=/private/tmp/claude-501/heavy-slots; mkdir -p $DIR
while true; do
  for i in $(seq 1 $SLOTS); do
    if mkdir $DIR/slot$i 2>/dev/null; then
      echo $$ > $DIR/slot$i/pid
      trap "rm -rf $DIR/slot$i" EXIT INT TERM
      "$@"; exit $?
    fi
    # reclaim slot if its owner died
    if [[ -f $DIR/slot$i/pid ]] && ! kill -0 $(cat $DIR/slot$i/pid) 2>/dev/null; then rm -rf $DIR/slot$i; fi
  done
  sleep 5
done
