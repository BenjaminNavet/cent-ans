#!/usr/bin/env bash
# Starts Cent Ans from the sources on macOS, Linux and Windows (Git Bash) (ADR 0117):
#   1. rebuilds the Rust GDExtension when core/ changed (cargo decides; core/build.sh only
#      replaces the library in game/bin/ when it changed),
#   2. runs the headless Godot import when game/ changed since the last import (files added,
#      removed or modified, other Godot version, first launch after a clone),
#   3. launches the game.
# Called by the double-click launchers at the root of the repository ("Lancer Cent Ans.*").
# Usage: tools/launch.sh [--no-build] [--import] [-- <extra Godot arguments>]
#   --no-build  skip the Rust build (the library already in game/bin/ is used)
#   --import    force the headless import
# Godot: $GODOT if set, else `godot`/`godot4` on the PATH, else the usual install places.
# Must stay compatible with the bash 3.2 shipped with macOS.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GAME="game"
STAMP="$GAME/.godot/cent_ans_import.stamp"
GODOT_SERIES="4.7"

BUILD=1
FORCE_IMPORT=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-build) BUILD=0 ;;
        --import) FORCE_IMPORT=1 ;;
        --)
            shift
            break
            ;;
        -h | --help)
            sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "Option inconnue : $1 (voir --help)" >&2
            exit 2
            ;;
    esac
    shift
done

cd "$ROOT"

say() { printf '\033[1m[Cent Ans]\033[0m %s\n' "$*"; }
fail() {
    printf '\033[1;31m[Cent Ans]\033[0m %s\n' "$*" >&2
    exit 1
}

case "$(uname -s)" in
    Darwin) PLATFORM="macos" ;;
    Linux) PLATFORM="linux" ;;
    MINGW* | MSYS* | CYGWIN*) PLATFORM="windows" ;;
    *) fail "Système non pris en charge : $(uname -s)" ;;
esac

# --- Godot -----------------------------------------------------------------------------------

# Prints the first existing executable among the arguments (globs already expanded).
first_executable() {
    local candidate
    for candidate in "$@"; do
        if [[ -f "$candidate" && -x "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done
    return 1
}

find_godot() {
    if [[ -n "${GODOT:-}" ]]; then
        if [[ "$PLATFORM" == "windows" ]] && command -v cygpath >/dev/null; then
            cygpath -u "$GODOT"
        else
            printf '%s\n' "$GODOT"
        fi
        return 0
    fi
    local name
    for name in godot godot4; do
        if command -v "$name" >/dev/null; then
            command -v "$name"
            return 0
        fi
    done
    shopt -s nullglob
    local found=1
    case "$PLATFORM" in
        macos)
            first_executable \
                /Applications/Godot.app/Contents/MacOS/Godot \
                "$HOME"/Applications/Godot.app/Contents/MacOS/Godot \
                /Applications/Godot_v"${GODOT_SERIES}"*.app/Contents/MacOS/Godot \
                "$HOME"/Downloads/Godot.app/Contents/MacOS/Godot \
                "$HOME"/Library/Application\ Support/Steam/steamapps/common/Godot\ Engine/Godot.app/Contents/MacOS/Godot &&
                found=0
            ;;
        linux)
            first_executable \
                "$HOME"/.local/bin/godot \
                "$HOME"/Applications/Godot_v"${GODOT_SERIES}"*_linux.* \
                "$HOME"/Downloads/Godot_v"${GODOT_SERIES}"*_linux.* \
                "$HOME"/Téléchargements/Godot_v"${GODOT_SERIES}"*_linux.* \
                "$ROOT"/../Godot_v"${GODOT_SERIES}"*_linux.* \
                /opt/godot/godot &&
                found=0
            ;;
        windows)
            # The console build (…_console.exe) is picked separately for the headless import.
            local exe
            for exe in \
                "$ROOT"/../Godot_v"${GODOT_SERIES}"*_win64.exe \
                "$HOME"/Desktop/Godot_v"${GODOT_SERIES}"*_win64.exe \
                "$HOME"/Downloads/Godot_v"${GODOT_SERIES}"*_win64.exe \
                "$HOME"/Downloads/Godot_v"${GODOT_SERIES}"*_win64/Godot_v"${GODOT_SERIES}"*_win64.exe \
                /c/Godot/Godot_v"${GODOT_SERIES}"*_win64.exe \
                /c/Program\ Files/Godot/Godot_v"${GODOT_SERIES}"*_win64.exe \
                "$HOME"/scoop/apps/godot/current/godot.exe; do
                if [[ -f "$exe" ]]; then
                    printf '%s\n' "$exe"
                    found=0
                    break
                fi
            done
            ;;
    esac
    shopt -u nullglob
    return $found
}

GODOT_BIN="$(find_godot)" || fail "Godot $GODOT_SERIES introuvable. Installez-le (https://godotengine.org/download)
ou indiquez son exécutable : GODOT=/chemin/vers/godot $0"

# On Windows the GUI build prints nothing in a terminal: import with the console build.
GODOT_CLI="$GODOT_BIN"
if [[ "$PLATFORM" == "windows" && "$GODOT_BIN" == *.exe && -f "${GODOT_BIN%.exe}_console.exe" ]]; then
    GODOT_CLI="${GODOT_BIN%.exe}_console.exe"
fi

GODOT_VERSION="$("$GODOT_CLI" --version 2>/dev/null | tail -n 1 | tr -d '\r')" ||
    fail "Impossible d'exécuter Godot : $GODOT_BIN"
say "Godot $GODOT_VERSION ($GODOT_BIN)"
if [[ "$GODOT_VERSION" != "$GODOT_SERIES".* ]]; then
    say "Attention : le projet attend Godot $GODOT_SERIES.x, le lancement peut échouer."
fi

# --- 1. Rust GDExtension ---------------------------------------------------------------------

case "$PLATFORM" in
    macos) LIB="$GAME/bin/libcent_ans.debug.dylib" ;;
    linux) LIB="$GAME/bin/libcent_ans.debug.so" ;;
    windows) LIB="$GAME/bin/cent_ans.debug.dll" ;;
esac

if [[ $BUILD -eq 1 ]]; then
    if command -v cargo >/dev/null || [[ -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        say "Compilation du cœur Rust (seulement si core/ a changé)…"
        core/build.sh || fail "La compilation du cœur Rust a échoué (voir ci-dessus)."
    elif [[ -f "$LIB" ]]; then
        say "Rust (cargo) absent : la bibliothèque déjà compilée $LIB est utilisée telle quelle."
    else
        fail "Rust (cargo) est nécessaire pour compiler le cœur du jeu : https://rustup.rs"
    fi
fi
[[ -f "$LIB" ]] || fail "Bibliothèque du cœur absente : $LIB (relancer sans --no-build)."

# --- 2. Headless import ----------------------------------------------------------------------

# Fingerprint of game/: Godot version + list of files (catches additions and removals). The
# compiled libraries are left out: a rebuild of the core needs no import.
game_fingerprint() {
    {
        printf '%s\n' "$GODOT_VERSION"
        find "$GAME" -path "$GAME/.godot" -prune -o -type f \
            ! -name '*.dylib' ! -name '*.so' ! -name '*.dll' ! -name '*.pdb' -print |
            LC_ALL=C sort
    } | cksum
}

# A file modified (or checked out by git) since the last import.
game_modified_since_stamp() {
    [[ -n "$(find "$GAME" -path "$GAME/.godot" -prune -o -type f \
        ! -name '*.dylib' ! -name '*.so' ! -name '*.dll' ! -name '*.pdb' \
        -newer "$STAMP" -print 2>/dev/null | head -n 1)" ]]
}

NEED_IMPORT=0
REASON=""
if [[ $FORCE_IMPORT -eq 1 ]]; then
    NEED_IMPORT=1 REASON="demandé (--import)"
elif [[ ! -f "$GAME/.godot/extension_list.cfg" || ! -f "$STAMP" ]]; then
    NEED_IMPORT=1 REASON="premier lancement, plusieurs minutes"
elif [[ "$(cat "$STAMP")" != "$(game_fingerprint)" ]]; then
    NEED_IMPORT=1 REASON="fichiers ajoutés ou supprimés, ou autre version de Godot"
elif game_modified_since_stamp; then
    NEED_IMPORT=1 REASON="fichiers du jeu modifiés"
fi

if [[ $NEED_IMPORT -eq 1 ]]; then
    say "Import des ressources Godot ($REASON)…"
    "$GODOT_CLI" --headless --path "$GAME" --import || fail "L'import Godot a échoué (voir ci-dessus)."
    # Fingerprint taken after the import: it writes the .import/.uid files next to the assets.
    game_fingerprint >"$STAMP"
else
    say "Ressources déjà importées."
fi

# --- 3. Game ---------------------------------------------------------------------------------

say "Lancement du jeu…"
exec "$GODOT_BIN" --path "$GAME" ${1+"$@"}
