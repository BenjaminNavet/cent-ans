# Lanceur depuis les sources (LCH) — ADR 0117

Branche `feat/launcher`. Objectif : un lanceur à double-cliquer pour macOS, Linux et Windows,
qui recompile le cœur Rust et refait l'import headless seulement quand c'est nécessaire.

## Fait
- `tools/launch.sh` (logique commune) + `Lancer Cent Ans.{command,sh,bat}` à la racine.
- `core/build.sh` multiplateforme (dylib/so, délègue à `build-windows.sh` sous Git Bash),
  copie seulement si la bibliothèque a changé (idem `build-windows.sh`).
- `cent_ans.gdextension` : entrées Linux x86_64/arm64 ; `.gitignore` `*.so` ; `.gitattributes`.
- ADR 0117, README, CLAUDE.md.

## Vérification
- [ ] Linux (conteneur, Godot 4.7.2 linux.x86_64) : compilation + import + smoke via le lanceur.
- [ ] 2e lancement : pas de recompilation copiée, pas d'import.
- [ ] Modification d'un fichier de `game/` → import ; ajout/suppression → import.
- [ ] macOS et Windows : non testables ici (à essayer sur la machine du joueur).
