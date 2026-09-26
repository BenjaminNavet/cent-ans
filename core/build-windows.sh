#!/usr/bin/env bash
# Builds the GDExtension for Windows x86_64 and copies the DLL into game/bin/ (ADR 0087).
# On macOS/Linux it cross-compiles with cargo-xwin (MSVC CRT/SDK fetched on first run);
# on Windows (Git Bash, CI) it uses plain cargo.
# Usage: core/build-windows.sh [--release]
# One-time setup on the Mac:
#   rustup target add x86_64-pc-windows-msvc
#   brew install llvm lld && cargo install --locked cargo-xwin
set -euo pipefail

CORE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GAME_BIN="$CORE_DIR/../game/bin"
TARGET="x86_64-pc-windows-msvc"

PROFILE="debug"
CARGO_FLAGS=(-p godot-bridge --target "$TARGET")
if [[ "${1:-}" == "--release" ]]; then
    PROFILE="release"
    CARGO_FLAGS+=(--release)
fi

cd "$CORE_DIR"
case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*)
        cargo build "${CARGO_FLAGS[@]}"
        ;;
    *)
        # Homebrew's rustc has no Windows std: prefer rustup's proxies, and LLVM's
        # clang-cl/lld-link for the C bits and the link step.
        export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/lld/bin:/opt/homebrew/opt/llvm/bin:$PATH"
        cargo xwin build "${CARGO_FLAGS[@]}"
        ;;
esac

mkdir -p "$GAME_BIN"
TARGET_DIR="${CARGO_TARGET_DIR:-$CORE_DIR/target}"
cp "$TARGET_DIR/$TARGET/$PROFILE/cent_ans.dll" "$GAME_BIN/cent_ans.$PROFILE.dll"
echo "Copied cent_ans.$PROFILE.dll to game/bin/"
