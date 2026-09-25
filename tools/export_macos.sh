#!/usr/bin/env bash
# Builds a distributable "Cent Ans.app" for Apple Silicon:
#   1. release build of the Rust GDExtension (core/build.sh --release),
#   2. Godot export with the "macOS" preset (game/export_presets.cfg),
#   3. copy of data/ into Cent Ans.app/Contents/Resources/data (read by MapPaths).
# Requires the Godot 4.7.2 export templates (Éditeur → Gérer les modèles d'export).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$ROOT/export/Cent Ans.app"

"$ROOT/core/build.sh" --release
mkdir -p "$ROOT/export"
rm -rf "$APP"
godot --headless --path "$ROOT/game" --import
# Not headless: the shader baker (export_presets.cfg, shader_baker/enabled) needs the Forward+
# renderer to precompile the shaders, otherwise the first launch compiles them (~10 s).
# The windowed editor rewrites project.godot on exit: keep the committed version.
cp "$ROOT/game/project.godot" "$ROOT/export/project.godot.bak"
trap 'mv "$ROOT/export/project.godot.bak" "$ROOT/game/project.godot"' EXIT
godot --path "$ROOT/game" --export-release "macOS" "$APP"
mkdir -p "$APP/Contents/Resources"
rsync -a --delete --exclude "schemas" "$ROOT/data/" "$APP/Contents/Resources/data/"
cp "$ROOT/CREDITS.md" "$APP/Contents/Resources/CREDITS.md"
# Ad-hoc signature after adding data (no Apple developer identity).
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
du -sh "$APP"
echo "Exporté : $APP"
# First launch of a fresh export compiles every GPU pipeline (macOS Metal cache, ~10 s before the
# menu, ~7 s at the first battle). One scripted run (menu, map, end of turn, save, battle) fills
# that cache for this machine, and checks the build. Skip with CENT_ANS_NO_WARMUP=1.
if [[ "${CENT_ANS_NO_WARMUP:-0}" != "1" ]]; then
    "$APP/Contents/MacOS/Cent Ans" -- --journey --frames=30 --turns=1 | grep "JOURNEY_JSON" | cut -c1-200 \
        || echo "Préchauffage : échec (voir le journal du jeu)"
fi
