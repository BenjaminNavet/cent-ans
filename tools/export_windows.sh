#!/usr/bin/env bash
# Builds a distributable Windows x86_64 folder "export/windows/" from the Mac (ADR 0087):
#   1. release build of the Rust GDExtension cross-compiled to cent_ans.dll
#      (core/build-windows.sh --release),
#   2. Godot export with the "Windows Desktop" preset (game/export_presets.cfg):
#      Cent Ans.exe + Cent Ans.pck + cent_ans.release.dll (+ Cent Ans.console.exe, which keeps a
#      console open with the game log, for bug reports),
#   3. copy of data/ next to the exe (read by MapPaths), relief cache placed according to
#      CENT_ANS_EXPORT_RELIEF as in tools/export_macos.sh ("bundle", "external", "none"),
#   4. zip "export/Cent Ans Windows.zip" (skip with CENT_ANS_NO_ZIP=1).
# Requires the Godot 4.7.2 Windows export templates (windows_*_x86_64.exe in
# ~/Library/Application Support/Godot/export_templates/4.7.2.stable/). No warm-up run: the
# game cannot be launched from the Mac; the GitHub workflow "windows" runs the smoke test.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/export/windows"
RELIEF_MODE="${CENT_ANS_EXPORT_RELIEF:-bundle}"

"$ROOT/core/build-windows.sh" --release
rm -rf "$OUT"
mkdir -p "$OUT"
godot --headless --path "$ROOT/game" --import
godot --headless --path "$ROOT/game" --export-release "Windows Desktop" "$OUT/Cent Ans.exe"
uv run --project "$ROOT/tools" cent-ans export-data --dir "$OUT" --relief "$RELIEF_MODE"
cp "$ROOT/CREDITS.md" "$OUT/CREDITS.md"
du -sh "$OUT"
if [[ "${CENT_ANS_NO_ZIP:-0}" != "1" ]]; then
    rm -f "$ROOT/export/Cent Ans Windows.zip"
    # The archive unpacks to "Cent Ans/" (zip follows the symlink and stores its files).
    (cd "$ROOT/export" && rm -f "Cent Ans" && ln -s windows "Cent Ans" \
        && zip -qr "Cent Ans Windows.zip" "Cent Ans" && rm "Cent Ans")
    du -sh "$ROOT/export/Cent Ans Windows.zip"
fi
echo "Exporté : $OUT"
