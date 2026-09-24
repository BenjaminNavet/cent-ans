# WIP — S1 effondrement des murailles + S2 incendies de siège

Demande du joueur (24/09) : physique de destruction des murailles et incendies.

## État
- **S1 (effondrement physique, rendu seulement)** : **fusionné dans main** (`41bb147`). Jolt activé, ADR 0007,
  `game/scripts/battle/wall_collapse_fx.gd`, réglages `data/fx/siege_fx.json`. Détails : `docs/wip/s1-effondrement-murailles.md`.
- **S2 (incendies, règle du cœur + rendu)** : **fusion prête mais PAS encore dans main**.
  - Branche `integration/s2` (commit `6934d01`) = main `f52fd92` + branche `worktree-agent-a9315344c0b1dad9c` (`0ada103`).
  - Conflit `battle_siege.gd` résolu (les deux côtés gardés : `_fx` S1 + `fire_fx` S2).
  - Vérifié sur `integration/s2` : `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test` OK.
  - Détails, règles et mesures : `docs/design/s2-incendies.md`, ADR 0008, `docs/wip/s2-incendies.md` (sur la branche).

## Prochaine étape (reprise)
1. `git worktree add ../gp-s2-merge integration/s2` (le worktree du scratchpad a pu disparaître ; la branche reste).
2. `core/build.sh` (ou copier la dylib dans `game/bin/`), `godot --headless --path game --import`, puis :
   `godot --headless --path game --script res://tests/smoke.gd`,
   `... res://tests/wall_collapse_fx_test.gd`, `... res://tests/s2_fire_fx_test.gd`,
   `uv run --project tools pytest tools/tests/test_siege_fx_schema.py tools/tests/test_siege_fire_schema.py`.
3. Si main a bougé : `git merge main` dans le worktree, re-tester ; puis dans le dépôt principal `git merge --ff-only integration/s2`.
4. Capture d'un siège en feu (pas faite : mode silencieux `~/.cent-ans-quiet` interdit les fenêtres Godot).

## Limites connues à traiter plus tard
- S1 : porte de Guyenne masquée par ses tours (géométrie antérieure) ; pas de son d'effondrement.
- S2 : règles embarquées par `include_str!` (recompiler pour changer un réglage) ; l'IA n'évite pas les rues en feu
  et n'utilise `burn` que pour les faubourgs ; pas de bouton « incendier » dans l'UI ; l'église ne s'effondre pas
  visuellement ; pas de lutte contre le feu.
