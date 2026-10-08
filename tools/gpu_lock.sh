#!/bin/sh
# Run a command while holding the global local-generator lock (mflux, Qwen, SF3D).
# Blocks until free. Same lock file as tools/experiments/dn_batch.py.
#   tools/gpu_lock.sh <command> [args...]
LOCK="${CENT_ANS_GPU_LOCK:-$HOME/dev/cent-ans-raw/dn/gpu.lock}"
mkdir -p "$(dirname "$LOCK")"
exec python3 -I -c '
import fcntl, os, sys
fd = os.open(sys.argv[1], os.O_RDWR | os.O_CREAT, 0o644)
fcntl.flock(fd, fcntl.LOCK_EX)
os.set_inheritable(fd, True)
os.execvp(sys.argv[2], sys.argv[2:])
' "$LOCK" "$@"
