# Lanceur depuis les sources (LCH) — ADR 0117 — TERMINÉ sous Linux le 2026-09-30

Branche `feat/launcher`. Objectif : un lanceur à double-cliquer pour macOS, Linux et Windows,
qui recompile le cœur Rust et refait l'import headless seulement quand c'est nécessaire.

## Fait
- `tools/launch.sh` (logique commune) + `Lancer Cent Ans.{command,sh,bat}` à la racine.
- `core/build.sh` multiplateforme (dylib/so, délègue à `build-windows.sh` sous Git Bash),
  copie seulement si la bibliothèque a changé (idem `build-windows.sh`).
- `cent_ans.gdextension` : entrées Linux x86_64/arm64 ; `.gitignore` `*.so` ; `.gitattributes`.
- ADR 0117, README, CLAUDE.md.

## Vérification (Linux, conteneur, Godot 4.7.2 linux.x86_64)
- [x] Clone vierge → lanceur : compilation (4 min 48), import, `smoke.gd` vert (30 « smoke OK », exit 0).
- [x] 2e lancement ≈ 5 s : cargo sans travail, `.so` « up to date », pas d'import.
- [x] Fichier modifié, ajouté ou supprimé dans `game/` → import (≈ 10 s sans rien de neuf) ; puis plus rien.
- [x] Changement dans `core/` → recompilation de godot-bridge, `.so` recopiée, pas d'import.
- [x] Godot trouvé sans `GODOT` (`~/Downloads/Godot_v4.7.2-stable_linux.x86_64`) ; message clair sinon.
- [x] `shellcheck` propre sur les scripts.
- [ ] macOS (`Lancer Cent Ans.command`) et Windows (`Lancer Cent Ans.bat`) : non testables ici, à
  essayer sur la machine du joueur.
- L'import a créé 16 `.uid` de tests manquants dans le dépôt : commités à part.
