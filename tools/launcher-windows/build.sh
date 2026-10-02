#!/usr/bin/env bash
# Builds the Windows double-click launcher and writes it at the root of the repository as
# "Lancer Cent Ans.exe" (ADR 0150). The executable is committed: a player who has just cloned
# the repository has no compiler yet. Rebuild and commit it whenever src/main.rs changes.
# Cross-compiles from macOS/Linux with cargo-xwin (same one-time setup as
# core/build-windows.sh); plain cargo on Windows (Git Bash).
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$CRATE_DIR/../.."
TARGET="x86_64-pc-windows-msvc"

cd "$CRATE_DIR"
case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*)
        cargo build --release --target "$TARGET"
        ;;
    *)
        export PATH="$HOME/.cargo/bin:/opt/homebrew/opt/lld/bin:/opt/homebrew/opt/llvm/bin:$PATH"
        cargo xwin build --release --target "$TARGET"
        ;;
esac

TARGET_DIR="${CARGO_TARGET_DIR:-$CRATE_DIR/target}"
cp "$TARGET_DIR/$TARGET/release/lancer-cent-ans.exe" "$ROOT/Lancer Cent Ans.exe"
ls -l "$ROOT/Lancer Cent Ans.exe"
