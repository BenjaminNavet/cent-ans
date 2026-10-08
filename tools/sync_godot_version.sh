#!/usr/bin/env bash
# Aligns the Godot version pinned in the repository on the Godot installed on this machine
# (ADR 0186): the Windows CI (.github/workflows/windows.yml, GODOT_VERSION) and, when the minor
# series changes, the series expected by tools/launch.sh and game/project.godot.
# Homebrew upgrades Godot every night; tools/launch.sh calls this script on the development
# branch `main` so that the CI tests the version the game is developed on.
# Usage: tools/sync_godot_version.sh "<output of godot --version>"
#   e.g. "4.7.2.stable.official.ed1daf0bf" -> GODOT_VERSION "4.7.2-stable", series "4.7".
# Only stable releases are pinned. Commits the changed files (explicit paths) when they had no
# other local change; otherwise leaves them modified and says so. Never fatal.
# Must stay compatible with the bash 3.2 shipped with macOS.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW=".github/workflows/windows.yml"
LAUNCHER="tools/launch.sh"
PROJECT="game/project.godot"
README="README.md"

say() { printf '\033[1m[Cent Ans]\033[0m %s\n' "$*"; }

# "4.7.2.stable.official.ed1daf0bf" -> "4.7.2" and "stable"; "4.8.stable.official.x" -> "4.8".
if [[ ! "${1:-}" =~ ^([0-9]+\.[0-9]+)(\.[0-9]+)?\.([a-z0-9]+)(\.|$) ]]; then
    say "Version de Godot illisible : « ${1:-} » ; CI non alignée."
    exit 0
fi
local_series="${BASH_REMATCH[1]}"
local_number="${BASH_REMATCH[1]}${BASH_REMATCH[2]}"
local_status="${BASH_REMATCH[3]}"
if [[ "$local_status" != "stable" ]]; then
    say "Godot $local_number-$local_status n'est pas une version stable : CI non alignée."
    exit 0
fi
local_tag="$local_number-stable"

cd "$ROOT"
pinned_tag="$(sed -n 's/^ *GODOT_VERSION: "\(.*\)"$/\1/p' "$WORKFLOW")"
[[ -n "$pinned_tag" ]] || { say "GODOT_VERSION introuvable dans $WORKFLOW."; exit 0; }
[[ "$pinned_tag" == "$local_tag" ]] && exit 0
pinned_number="${pinned_tag%-stable}"
pinned_series="$(sed -n 's/^GODOT_SERIES="\(.*\)"$/\1/p' "$LAUNCHER")"

files=("$WORKFLOW" "$README")
[[ "$pinned_series" != "$local_series" ]] && files+=("$LAUNCHER" "$PROJECT")
if ! git diff --quiet HEAD -- "${files[@]}"; then
    say "Godot $local_tag installé, CI en $pinned_tag : ${files[*]} ont des modifications locales, alignement à faire à la main."
    exit 0
fi

# Rewrites a file through a temporary copy: a new inode, so the bash running launch.sh keeps
# reading the old one.
rewrite() {
    local file="$1" expression="$2"
    sed "$expression" "$file" >"$file.tmp" && mv "$file.tmp" "$file"
}
escape() { printf '%s' "$1" | sed 's/\./\\./g'; }

rewrite "$WORKFLOW" "s/GODOT_VERSION: \"$(escape "$pinned_tag")\"/GODOT_VERSION: \"$local_tag\"/"
rewrite "$README" "s/$(escape "$pinned_number")-stable/$local_tag/g; s/Godot $(escape "$pinned_number")\([^.0-9]\)/Godot $local_number\1/g"
if [[ "$pinned_series" != "$local_series" ]]; then
    rewrite "$LAUNCHER" "s/^GODOT_SERIES=\"$(escape "$pinned_series")\"$/GODOT_SERIES=\"$local_series\"/"
    chmod +x "$LAUNCHER"
    rewrite "$PROJECT" "s/config\/features=PackedStringArray(\"$(escape "$pinned_series")\"/config\/features=PackedStringArray(\"$local_series\"/"
    rewrite "$README" "s/Godot $(escape "$pinned_series")\([^.0-9]\)/Godot $local_series\1/g"
fi

if git commit --quiet -m "chore(ci): follow the installed Godot $local_tag (was $pinned_tag)" -- "${files[@]}"; then
    say "CI alignée sur Godot $local_tag (commit $(git rev-parse --short HEAD), à pousser)."
else
    say "CI alignée sur Godot $local_tag, commit impossible : ${files[*]} modifiés, à commiter."
fi
