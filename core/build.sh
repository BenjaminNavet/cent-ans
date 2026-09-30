#!/usr/bin/env bash
# Builds the GDExtension for the host platform and copies the library into game/bin/ so that
# Godot can load it: libcent_ans.<profile>.dylib (macOS), libcent_ans.<profile>.so (Linux),
# cent_ans.<profile>.dll (Windows, via core/build-windows.sh).
# The library is only replaced when its content changed (lanceur, ADR 0117): an unchanged build
# leaves game/bin/ untouched.
# Usage: core/build.sh [--release]
set -euo pipefail

CORE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GAME_BIN="$CORE_DIR/../game/bin"

case "$(uname -s)" in
    Darwin) LIB_SRC="libcent_ans.dylib" LIB_EXT="dylib" ;;
    Linux) LIB_SRC="libcent_ans.so" LIB_EXT="so" ;;
    MINGW* | MSYS* | CYGWIN*) exec "$CORE_DIR/build-windows.sh" "$@" ;;
    *)
        echo "build.sh: unsupported platform $(uname -s)" >&2
        exit 1
        ;;
esac

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
SRC="$TARGET_DIR/$PROFILE/$LIB_SRC"
DEST="$GAME_BIN/libcent_ans.$PROFILE.$LIB_EXT"
if cmp -s "$SRC" "$DEST"; then
    echo "libcent_ans.$PROFILE.$LIB_EXT is up to date in game/bin/"
    exit 0
fi
# Remove first: overwriting a loaded/signed dylib in place invalidates its code signature on
# macOS and the next Godot launch is SIGKILLed (exit 137). A fresh inode avoids it.
rm -f "$DEST"
cp "$SRC" "$DEST"
echo "Copied libcent_ans.$PROFILE.$LIB_EXT to game/bin/"
