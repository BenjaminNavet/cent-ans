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
godot --headless --path "$ROOT/game" --export-release "macOS" "$APP"
mkdir -p "$APP/Contents/Resources"
rsync -a --delete --exclude "schemas" "$ROOT/data/" "$APP/Contents/Resources/data/"
# Ad-hoc signature after adding data (no Apple developer identity).
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
du -sh "$APP"
echo "Exporté : $APP"
