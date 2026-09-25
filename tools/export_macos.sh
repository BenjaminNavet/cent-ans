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
# Shader baker left off (export_presets.cfg): tested with the Metal toolchain, it saved only
# ~0.5 s of the ~11 s first launch (driver pipeline compilation dominates) and made the exported
# game print a ParticlesShaderRD leak at every exit. The warm-up run below is what helps.
godot --headless --path "$ROOT/game" --export-release "macOS" "$APP"
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
