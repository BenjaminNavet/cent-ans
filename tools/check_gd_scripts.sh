#!/usr/bin/env bash
# GT9: Godot exits 0 on a GDScript Parse Error in a --script file, so a broken test goes unnoticed.
# This runs `godot --check-only` on each game/tests/*.gd and game/tests/lib/*.gd (or on the paths
# given as arguments, relative to game/ or the repo root) and exits 1 if any script reports an error.
# Prerequisite: `godot --headless --path game --import` once (class_name cache).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT="${GODOT:-godot}"
# macOS has no `timeout`: use it (or gtimeout) when present, otherwise run unbounded (watchdog still applies).
TO=(); for t in timeout gtimeout; do command -v "$t" >/dev/null && { TO=("$t" 60); break; }; done
cd "$ROOT"

files=()
if [ "$#" -gt 0 ]; then
  for p in "$@"; do
    p="${p#"$ROOT"/}"; p="${p#game/}"; files+=("$p")
  done
else
  while IFS= read -r f; do files+=("${f#game/}"); done < <(ls game/tests/*.gd game/tests/lib/*.gd 2>/dev/null)
fi

bad=()
for rel in "${files[@]}"; do
  out="$(CENT_ANS_MAX_ERRORS=200 ${TO[@]+"${TO[@]}"} "$GODOT" --headless --path game --check-only --script "res://$rel" 2>&1 || true)"
  if printf '%s\n' "$out" | grep -qE 'SCRIPT ERROR|Parse Error|Compile Error'; then
    bad+=("$rel")
    printf '%s\n' "$out" | grep -E 'SCRIPT ERROR|Parse Error|Compile Error' | head -5 | sed "s|^|[$rel] |"
  fi
done

echo "check_gd_scripts: ${#files[@]} script(s) checked, ${#bad[@]} with errors"
if [ "${#bad[@]}" -gt 0 ]; then
  printf 'FAILED: %s\n' "${bad[@]}"
  exit 1
fi
