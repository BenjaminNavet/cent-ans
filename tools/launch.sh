#!/usr/bin/env bash
# Starts Cent Ans from the sources on macOS, Linux and Windows (Git Bash) (ADR 0117, ADR 0362).
# The Bevy client (client/); the Godot client was removed (ADR 0366).
#   0. fast-forwards the clone when it follows the `stable` branch (ADR 0159; never fatal),
#   1. checks cargo and, on Linux, the system libraries (alsa, udev, vulkan...), then
#      `cargo build --release` in client/ (cargo decides what to recompile),
#   1b. downloads the generated 3D models package when absent or older than the published one
#      (ADR 0212; never fatal: maquettes procédurales de repli),
#   2. checks the composed relief pyramid (data/map/pyramid/relief_pyramid.json; links the usual
#      store ~/dev/cent-ans-raw/compose/pyramid when present, else prints the generation command;
#      never fatal: without it the close zoom is only limited),
#   3. launches the game.
# Called by the double-click launchers at the root of the repository ("Lancer Cent Ans.*").
# Usage: tools/launch.sh [--dry-run] [--no-update] [--no-build] [--no-relief]
#                        [--no-models] [-- <extra arguments of the client>]
#   --dry-run    print what would be done (tools found, missing libraries, commands), change nothing
#   --no-update  skip the update check (also: CENT_ANS_NO_UPDATE=1)
#   --no-build   skip the Rust build (the binary already built is used)
#   --no-relief  skip the composed relief check
#   --no-models  skip the generated models check (no download)
# Arguments after `--` go to the client (--state, --faction, --seed...).
# Must stay compatible with the bash 3.2 shipped with macOS.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Branch followed by the players' clones; the CI moves it (.github/workflows/windows.yml).
UPDATE_BRANCH="stable"
CLIENT="client"
WINDOWS_LAUNCHER="Lancer Cent Ans.exe"

# Kept for the restart after an update (bash 3.2: an empty array is unset under `set -u`).
ORIGINAL_ARGS=(${1+"$@"})

# No update in CI: the workflow tests the commit it checked out.
UPDATE=1
[[ -n "${CENT_ANS_NO_UPDATE:-}" || -n "${CI:-}" ]] && UPDATE=0
BUILD=1
DRY_RUN=0
RELIEF=1
MODELS=1
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run) DRY_RUN=1 ;;
        --no-update) UPDATE=0 ;;
        --no-build) BUILD=0 ;;
        --no-relief) RELIEF=0 ;;
        --no-models) MODELS=0 ;;
        --)
            shift
            break
            ;;
        -h | --help)
            sed -n '2,22p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
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

# --- 0. Update -------------------------------------------------------------------------------

# Windows refuses to overwrite a running executable but lets it be renamed: the running
# launcher becomes *.old (removed at the next launch) and an identical, unlocked copy takes its
# place for git to replace.
release_running_launcher() {
    cp -p "$WINDOWS_LAUNCHER" "$WINDOWS_LAUNCHER.new" &&
        mv -f "$WINDOWS_LAUNCHER" "$WINDOWS_LAUNCHER.old" &&
        mv "$WINDOWS_LAUNCHER.new" "$WINDOWS_LAUNCHER"
    # Never leave the clone without its launcher.
    if [[ ! -f "$WINDOWS_LAUNCHER" && -f "$WINDOWS_LAUNCHER.old" ]]; then
        mv "$WINDOWS_LAUNCHER.old" "$WINDOWS_LAUNCHER"
    fi
    rm -f "$WINDOWS_LAUNCHER.new"
}

# Players follow `stable`, which the CI fast-forwards to every commit of main that passes the
# smoke test (ADR 0159). Only a clone sitting on that branch updates itself: a development
# checkout (main, feature branches, worktrees) is never touched. Fast-forward only, and git
# keeps local changes or refuses: nothing is ever discarded. Never fatal: offline, diverged or
# refused, the installed version starts.
# The whole function is parsed before it runs and it ends with a restart, so bash never reads
# on in a launch.sh that git has just replaced.
self_update() {
    command -v git >/dev/null || return 0
    local branch remote before target count
    branch="$(git symbolic-ref --quiet --short HEAD 2>/dev/null)" || return 0
    if [[ "$branch" != "$UPDATE_BRANCH" ]]; then
        say "Mises à jour automatiques inactives sur la branche $branch (pour les activer : git fetch && git switch $UPDATE_BRANCH)."
        return 0
    fi
    remote="$(git config "branch.$UPDATE_BRANCH.remote" || true)"
    [[ -n "$remote" && "$remote" != "." ]] || return 0

    say "Recherche d'une mise à jour…"
    if ! GIT_TERMINAL_PROMPT=0 git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=15 \
        fetch --quiet "$remote" "$UPDATE_BRANCH" 2>/dev/null; then
        say "Mise à jour non vérifiée (hors ligne ?) : la version installée est lancée."
        return 0
    fi
    before="$(git rev-parse HEAD)"
    target="$(git rev-parse FETCH_HEAD)"
    if [[ "$before" == "$target" ]]; then
        say "Le jeu est à jour."
        return 0
    fi
    if ! git merge-base --is-ancestor "$before" "$target" 2>/dev/null; then
        say "Mise à jour impossible : ce clone a des commits absents de $remote/$UPDATE_BRANCH. La version installée est lancée."
        return 0
    fi

    count="$(git rev-list --count "$before..$target")"
    say "Mise à jour : $count commit(s) à installer…"
    if [[ "$PLATFORM" == "windows" && -f "$WINDOWS_LAUNCHER" ]] &&
        ! git diff --quiet "$before" "$target" -- "$WINDOWS_LAUNCHER"; then
        release_running_launcher || true
    fi
    if ! git merge --ff-only --quiet "$target"; then
        say "Mise à jour non installée (fichiers modifiés sur place, voir ci-dessus) : la version installée est lancée."
        return 0
    fi
    say "Mise à jour installée : version $(git log -1 --format='%h du %cd' --date=format:'%d/%m/%Y')."
    exec bash "$ROOT/tools/launch.sh" --no-update ${ORIGINAL_ARGS[@]+"${ORIGINAL_ARGS[@]}"}
}

rm -f "$WINDOWS_LAUNCHER.old" 2>/dev/null || true
# A dry run changes nothing: no fetch, no merge.
if [[ $UPDATE -eq 1 && $DRY_RUN -eq 0 ]]; then
    self_update
fi

# First top-level "version" of a JSON file written one key per line (empty if absent).
json_version() {
    [[ -f "$1" ]] || return 0
    sed -n 's/^ *"version": *\([0-9][0-9]*\).*/\1/p' "$1" | head -n 1
}

# Same scheme as the relief (ADR 0212): dn/package.json says which package is installed. The game
# runs without it (procedural maquettes), so a failure never blocks the launch.
fetch_models() {
if [[ $MODELS -eq 1 ]]; then
    MODELS_WANTED="$(json_version data/art/dn_models_hosting.json)"
    MODELS_HAVE="$(json_version "${CENT_ANS_MODELS_DIR:-game/assets/models}/dn/package.json")"
    if [[ -n "$MODELS_WANTED" && "$MODELS_HAVE" != "$MODELS_WANTED" ]]; then
        if command -v uv >/dev/null; then
            say "Modèles générés : vérification du paquet v$MODELS_WANTED (téléchargement ≈ 0,9 Go s'il manque ; --no-models pour passer)…"
            uv run --project tools cent-ans art models-fetch --if-needed ||
                say "Modèles générés non mis à jour (maquettes de repli) ; nouvel essai au prochain lancement."
        else
            say "Modèles générés absents ou anciens, et uv introuvable (https://docs.astral.sh/uv/) : maquettes de repli."
        fi
    fi
fi
}

# The Bevy client reads the COMPOSED relief pyramid (ADR 0355): <relief dir>/pyramid/relief_pyramid.json,
# produced locally by `cent-ans geo compose-relief-real` and kept outside git (the real relief
# package v3 of the Godot client is NOT used any more). If the pyramid is missing and the usual
# store (~/dev/cent-ans-raw/compose/pyramid) exists, it is linked in place; otherwise the command
# to generate it is printed. Never fatal: without it the close zoom falls back to procedural detail.
COMPOSED_PYRAMID_STORE="${CENT_ANS_COMPOSED_PYRAMID:-$HOME/dev/cent-ans-raw/compose/pyramid}"

relief_dir() { printf '%s\n' "${CENT_ANS_RELIEF_DIR:-data/map}"; }

check_relief() {
    [[ $RELIEF -eq 1 ]] || return 0
    local dir manifest
    dir="$(relief_dir)/pyramid"
    manifest="$dir/relief_pyramid.json"
    if [[ -f "$manifest" ]]; then
        [[ $DRY_RUN -eq 1 ]] && say "(essai à blanc) relief composé : $manifest présent"
        return 0
    fi
    if [[ -f "$COMPOSED_PYRAMID_STORE/relief_pyramid.json" && ! -e "$dir" ]]; then
        if [[ $DRY_RUN -eq 1 ]]; then
            say "(essai à blanc) relief composé : $dir serait lié à $COMPOSED_PYRAMID_STORE"
            return 0
        fi
        rm -f "$dir" 2>/dev/null || true
        if ln -s "$COMPOSED_PYRAMID_STORE" "$dir" 2>/dev/null; then
            say "Relief composé : $dir lié à $COMPOSED_PYRAMID_STORE."
            return 0
        fi
    fi
    say "Relief composé absent ($manifest) : la carte rapprochée sera limitée."
    if [[ -e "$dir" || -L "$dir" ]]; then
        say "  $dir existe mais sans relief_pyramid.json (ancien paquet réel v3 ?) : renommez-le (mv $dir ${dir}_v3_reel) puis relancez."
    fi
    say "  Génération (hors git, plusieurs Go) : uv run --project tools cent-ans geo compose-relief-real"
    say "  puis liez la sortie : ln -s <dossier produit> $dir (voir docs/wip/cc-carte-composee.md)."
}

# --- Bevy client (ADR 0362) ---------------------------------------------------------

# Linux only: the system libraries Bevy links or loads (alsa, udev, a Vulkan driver, windowing).
# Prints what is missing with the install command; returns 1 when something is missing.
check_linux_libraries() {
    local missing=() name
    if command -v pkg-config >/dev/null; then
        for name in alsa libudev; do
            pkg-config --exists "$name" || missing+=("$name")
        done
    else
        missing+=("pkg-config")
    fi
    # A Vulkan loader and at least one driver (mesa, nvidia, amd...).
    if ! { command -v vulkaninfo >/dev/null || ldconfig -p 2>/dev/null | grep -q 'libvulkan\.so'; }; then
        missing+=("vulkan (libvulkan1 + pilote)")
    fi
    if [[ ${#missing[@]} -eq 0 ]]; then
        say "Bibliothèques système : alsa, udev, vulkan trouvés."
        return 0
    fi
    say "Bibliothèques système manquantes pour Bevy : ${missing[*]}"
    say "  Debian/Ubuntu : sudo apt install build-essential pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev libvulkan1 mesa-vulkan-drivers"
    say "  Fedora        : sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel vulkan-loader mesa-vulkan-drivers"
    say "  Arch          : sudo pacman -S base-devel alsa-lib systemd-libs wayland libxkbcommon vulkan-icd-loader"
    return 1
}

CLIENT_TARGET="${CARGO_TARGET_DIR:-$CLIENT/target}"
BINARY_NAME="cent-ans"
[[ "$PLATFORM" == "windows" ]] && BINARY_NAME="cent-ans.exe"
CLIENT_BINARY="$CLIENT_TARGET/release/$BINARY_NAME"

find_cargo() {
    export PATH="$HOME/.cargo/bin:${USERPROFILE:+$(cygpath -u "$USERPROFILE" 2>/dev/null)/.cargo/bin:}$PATH"
    command -v cargo >/dev/null
}

launch_bevy() {
    local have_cargo=0
    find_cargo && have_cargo=1
    if [[ $DRY_RUN -eq 1 ]]; then
        say "(essai à blanc) client Bevy, plateforme $PLATFORM"
        if [[ $have_cargo -eq 1 ]]; then say "(essai à blanc) cargo : $(command -v cargo)"; else say "(essai à blanc) cargo introuvable : rustup serait nécessaire (https://rustup.rs)"; fi
        [[ "$PLATFORM" == "linux" ]] && { check_linux_libraries || true; }
        [[ $BUILD -eq 1 ]] && say "(essai à blanc) compilation : cargo build --release --manifest-path $CLIENT/Cargo.toml"
        [[ $MODELS -eq 1 ]] && say "(essai à blanc) modèles générés : paquet v$(json_version data/art/dn_models_hosting.json) voulu, v$(json_version "${CENT_ANS_MODELS_DIR:-game/assets/models}/dn/package.json") installé"
        check_relief
        say "(essai à blanc) lancement : CENT_ANS_ROOT=$ROOT $CLIENT_BINARY ${1+$*}"
        exit 0
    fi

    if [[ $BUILD -eq 1 ]]; then
        if [[ $have_cargo -eq 1 ]]; then
            [[ "$PLATFORM" == "linux" ]] && { check_linux_libraries || fail "Installez ces bibliothèques puis relancez."; }
            if [[ -x "$CLIENT_BINARY" ]]; then
                say "Compilation du client Bevy (seulement ce qui a changé)…"
            else
                say "Compilation du client Bevy : première fois, 10 à 30 minutes selon la machine (Bevy est gros)…"
            fi
            cargo build --release --manifest-path "$CLIENT/Cargo.toml" ||
                fail "La compilation du client Bevy a échoué (voir ci-dessus)."
        elif [[ -x "$CLIENT_BINARY" ]]; then
            say "Rust (cargo) absent : le client déjà compilé $CLIENT_BINARY est utilisé tel quel."
        else
            fail "Rust (cargo) est nécessaire pour compiler le client : https://rustup.rs
(Windows : installez aussi les Build Tools de Visual Studio, charge « Développement Desktop en C++ ».)"
        fi
    fi
    [[ -x "$CLIENT_BINARY" ]] || fail "Client Bevy absent : $CLIENT_BINARY (relancer sans --no-build)."

    fetch_models
    check_relief

    say "Lancement du jeu (Bevy)…"
    export CENT_ANS_ROOT="$ROOT"
    exec "$CLIENT_BINARY" ${1+"$@"}
}

launch_bevy ${1+"$@"}
