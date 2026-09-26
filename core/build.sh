#!/usr/bin/env bash
# Builds the GDExtension and copies the dylib into game/bin/ so that Godot can load it.
# Usage: core/build.sh [--release]
set -euo pipefail

CORE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GAME_BIN="$CORE_DIR/../game/bin"

PROFILE="debug"
CARGO_FLAGS=()
if [[ "${1:-}" == "--release" ]]; then
    PROFILE="release"
    CARGO_FLAGS+=(--release)
fi

cd "$CORE_DIR"
cargo build -p godot-bridge ${CARGO_FLAGS[@]+"${CARGO_FLAGS[@]}"}

mkdir -p "$GAME_BIN"
# PB3a: honour a shared CARGO_TARGET_DIR (agent worktrees build into the main checkout's
# target/ to avoid duplicating gigabytes of build artefacts and losing incremental cache).
TARGET_DIR="${CARGO_TARGET_DIR:-$CORE_DIR/target}"
# Remove first: overwriting a loaded/signed dylib in place invalidates its code signature on
# macOS and the next Godot launch is SIGKILLed (exit 137). A fresh inode avoids it.
rm -f "$GAME_BIN/libcent_ans.$PROFILE.dylib"
cp "$TARGET_DIR/$PROFILE/libcent_ans.dylib" "$GAME_BIN/libcent_ans.$PROFILE.dylib"
echo "Copied libcent_ans.$PROFILE.dylib to game/bin/"
