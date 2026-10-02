# Portage Windows (WIN) — TERMINÉ le 2026-09-26 (ADR 0087)

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
- [x] 2. Export réel OK (`export/windows/`, 845 Mo sans relief : exe 104 Mo, pck 576 Mo — tous les
  assets importés, pas propre à Windows —, dll 18 Mo, data 147 Mo, console.exe).
- [ ] 3. Partie d'essai sur un vrai PC (joueur) : seul point non vérifié (rendu Vulkan/D3D12, perfs).
  Non testé non plus : l'exe exporté lui-même (la CI teste la DLL debug dans l'éditeur).
- [x] 4. ADR 0087 (b0b4807c).
- [x] 5. README (99bb1f53, seulement la section ; Remerciements d'une autre session non touché).
- [x] 6. docs/tools.md « Export Windows ».
- [x] 7. Branche `windows/port` supprimée (locale + distante). Pour relancer la CI : pousser une
  branche `windows/…` ou, une fois main poussée, `gh workflow run windows.yml`.

## Suite 2026-09-26 soir
- `.gitattributes` (`*.sh` en LF) et pas à pas README pour lancer depuis les sources sous Windows.
- main poussée sur origin jusqu'à 6e68ed1b (export construit depuis bc89fe91, même code).
- `export/Cent Ans Windows.zip` (805 Mo, sans relief, se décompresse en `Cent Ans/`) prêt.
- **Release GitHub non créée** (permission refusée à l'agent) : commande à lancer par le joueur,
  notes dans `docs/wip/windows-release-notes.md` :
  `gh release create preview-windows-2026-09-26 "export/Cent Ans Windows.zip" --target 6e68ed1b --prerelease --title "Cent Ans — préversion Windows (26/09/2026)" --notes-file docs/wip/windows-release-notes.md`

## Vérification 2026-09-30
- CI Windows run 36664599846 vert sur `main` d95f45b9 (≈ 575 commits après le portage) : DLL
  11 min, import 3 min, smoke 14 min. Lancé en poussant `windows/check-2026-09-30` (le
  `workflow_dispatch` est refusé à l'agent, 403).

## Lanceur `.exe` 2026-10-02 (ADR 0153)
- Demande du joueur : un `.exe` pour démarrer sous Windows après un clone (pas un export).
- `Lancer Cent Ans.exe` commité à la racine (309 Ko, n'importe que des DLL système), source
  `tools/launcher-windows/` (2 tests unitaires), build `tools/launcher-windows/build.sh`.
  Même travail que le `.bat`, qui reste en repli.
- CI `windows.yml` : tests unitaires du lanceur, puis smoke via l'exécutable commité.
- CI run 37030962872 vert (exe commité : compilation, import, smoke sous Windows). Fusionné dans
  main et poussé (661ed46a9) ; branche et worktree supprimés. ADR renuméroté 0153 (0150 pris).
