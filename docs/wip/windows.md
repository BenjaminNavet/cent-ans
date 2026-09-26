# Portage Windows (WIN) — repris le 2026-09-26

Objectif : le jeu se lance et s'exporte sous Windows x86_64.

## Fait (dans main)
- `game/bin/cent_ans.gdextension` : entrées `windows.{debug,release}.x86_64` →
  `res://bin/cent_ans.{debug,release}.dll` (`.dll`/`.pdb` ignorés dans `game/bin/.gitignore`).
- `core/build-windows.sh [--release]` : compilation croisée depuis le Mac avec `cargo xwin`
  (cible `x86_64-pc-windows-msvc`), `cargo build` simple sous Windows (Git Bash / CI).
  **Vérifié** : la DLL debug compile (≈ 1 min 15), exporte `gdext_rust_init`, n'importe que
  des DLL système.
- `core/.cargo/config.toml` : CRT statique (`+crt-static`, pas besoin du redistribuable
  Visual C++) et `/ignore:4099` (PDB de libcmt absents, inoffensif).
- `MapPaths.default_data_dir` : jeu exporté Windows → `data/` à côté de `Cent Ans.exe` ;
  relief externe : dossier `Cent Ans relief` à côté de l'exe (déjà prévu).
- `cent-ans export-data --dir <dossier>` (en plus de `--app`) + test pytest (7/7 verts).
- `game/export_presets.cfg` : préréglage `[preset.1]` « Windows Desktop » (x86_64, pck
  séparé, S3TC/BPTC, `modify_resources=false` pour éviter rcedit/wine, wrapper console).
- `tools/export_windows.sh` : dll release → export Godot → data/ → CREDITS → zip.
- `.github/workflows/windows.yml` : sur `windows-latest`, build dll debug, import Godot,
  `smoke.gd` headless. Déclenché par `workflow_dispatch` ou push sur `windows/**`.

## Pièges rencontrés
- Le `cargo`/`rustc` du PATH est celui de Homebrew (pas de std Windows) : le script met
  `~/.cargo/bin` (rustup, mis à jour en 1.98.1 + cible msvc) en tête du PATH.
- Installés sur le Mac : `brew install lld`, `cargo install cargo-xwin`, modèles d'export
  Windows 4.7.2 (`windows_*.exe`) dans `~/Library/Application Support/Godot/export_templates/4.7.2.stable/`.

## Reste à faire
- [x] 1. CI : run 36228061477 vert sous Windows (21 min, vraie sim Rust).
- [ ] 2. Export réel `CENT_ANS_EXPORT_RELIEF=none CENT_ANS_NO_ZIP=1 tools/export_windows.sh` — en cours.
- [ ] 3. Partie d'essai sur un vrai PC (joueur).
- [x] 4. ADR 0087 (b0b4807c).
- [x] 5. README (99bb1f53, seulement la section ; Remerciements d'une autre session non touché).
- [x] 6. docs/tools.md « Export Windows ».
- [ ] 7. Supprimer la branche `windows/port` (locale + distante) à la fin.
