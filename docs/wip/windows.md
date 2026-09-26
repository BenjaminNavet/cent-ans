# Portage Windows (WIN) — EN PAUSE (2026-09-26)

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
1. **CI** : la branche `windows/port` (= main au commit 2509864b) était en cours de push vers
   origin au moment de la pause. Vérifier `git ls-remote origin refs/heads/windows/port` ;
   sinon `git push origin windows/port`. Puis `gh run list --workflow windows` / `gh run watch`.
   Corriger ce qui casse (chemins, `\` vs `/`, fins de ligne, smoke). Dépôt privé : minutes
   Windows ×2 sur le quota gratuit (0 $ attendu, noter dans docs/budget.md sinon).
2. **Export réel** : `CENT_ANS_EXPORT_RELIEF=none tools/export_windows.sh` (release ≈ 10 min
   de LTO), vérifier que `export/windows/` contient exe, pck, `cent_ans.dll`, `data/`.
   Si l'export Godot refuse une option du préréglage, l'ouvrir une fois dans l'éditeur
   (Projet → Exporter) pour qu'il réécrive les clés de 4.7.2.
3. Idéalement : une partie d'essai sur un vrai PC (GPU Vulkan/D3D12, perfs non mesurées).
4. ADR `docs/decisions/0087-portage-windows.md` (numéro à vérifier : dernier = 0086).
5. **README** (demandé par le joueur) : section « Windows » — prérequis, `core/build-windows.sh`,
   `tools/export_windows.sh`, lancement. Attention : README.md avait des modifications non
   commitées d'une autre session ; n'ajouter que la section.
6. `docs/tools.md` : section « Export Windows » à côté de « Export macOS ».
7. Fusion finale : tout est déjà dans main (commits `wip:` b9175b41 → 2509864b) ; supprimer
   la branche `windows/port` (locale + distante) une fois la CI verte.
