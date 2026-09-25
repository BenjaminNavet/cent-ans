#!/usr/bin/env bash
# Builds a distributable "Cent Ans.app" for Apple Silicon:
#   1. release build of the Rust GDExtension (core/build.sh --release),
#   2. Godot export with the "macOS" preset (game/export_presets.cfg),
#   3. copy of data/ into Cent Ans.app/Contents/Resources/data (read by MapPaths), never into the
#      .pck; the fine relief cache data/map/pyramid (~2.9 GB, ADR 0036 lot ZG7b) goes where
#      CENT_ANS_EXPORT_RELIEF says: "bundle" (default, inside the app), "external" (folder
#      "Cent Ans relief" next to the app, split download) or "none" (light build, close zoom
#      limited). Run `uv run --project tools cent-ans geo relief-all --check` first.
# Requires the Godot 4.7.2 export templates (Éditeur → Gérer les modèles d'export).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$ROOT/export/Cent Ans.app"
RELIEF_MODE="${CENT_ANS_EXPORT_RELIEF:-bundle}"

"$ROOT/core/build.sh" --release
mkdir -p "$ROOT/export"
rm -rf "$APP"
godot --headless --path "$ROOT/game" --import
# Shader baker left off (export_presets.cfg): tested with the Metal toolchain, it saved only
# ~0.5 s of the ~11 s first launch (driver pipeline compilation dominates) and made the exported
# game print a ParticlesShaderRD leak at every exit (PF1: that leak came from the campaign
# weather's ParticleProcessMaterial, now a plain particles shader). The warm-up run below is
# what helps. The exported game keeps its own user dir (ADR 0031).
godot --headless --path "$ROOT/game" --export-release "macOS" "$APP"
mkdir -p "$APP/Contents/Resources"
rm -rf "$ROOT/export/Cent Ans relief"
# data/ without schemas, symlinks followed (agent worktrees link the relief cache), APFS clones.
uv run --project "$ROOT/tools" cent-ans export-data --app "$APP" --relief "$RELIEF_MODE"
cp "$ROOT/CREDITS.md" "$APP/Contents/Resources/CREDITS.md"
# Ad-hoc signature after adding data (no Apple developer identity).
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true
du -sh "$APP"
if [[ -d "$ROOT/export/Cent Ans relief" ]]; then du -sh "$ROOT/export/Cent Ans relief"; fi
echo "Exporté : $APP"
# First launch of a fresh export compiles every GPU pipeline (macOS Metal cache, ~10 s before the
# menu, ~7 s at the first battle). One scripted run (menu, map, end of turn, save, battle) fills
# that cache for this machine, and checks the build. Skip with CENT_ANS_NO_WARMUP=1.
if [[ "${CENT_ANS_NO_WARMUP:-0}" != "1" ]]; then
    "$APP/Contents/MacOS/Cent Ans" -- --journey --frames=30 --turns=1 | grep "JOURNEY_JSON" | cut -c1-200 \
        || echo "Préchauffage : échec (voir le journal du jeu)"
fi
