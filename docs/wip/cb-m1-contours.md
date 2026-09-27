# CB-M1 — Contours de formation en décales (état)

Branche : `feat/cb-m1-outline`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB-M1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`
(« Contour de formation »).

## État

- [x] Squelette `game/scripts/battle/battle_formation_outline.gd` (API publique, vide).
- [x] Implémentation (textures par rapport d'aspect, décales, états, pulsé) — non testée.
- [x] Branchement dans `battle_scene.gd` (`outlines`, `_update_outlines`), `_rings` et `BattleMeshes.outline` supprimés.
- [x] Test `game/tests/cb_m1_outline_test.gd` écrit, pas encore lancé (dylib en construction).
- [ ] Capture `game/tests/cbm_outline_shot.gd`.

## Prochaine étape

Lancer le test (build `CARGO_TARGET_DIR=<worktree>/core/target-cbm1 ./core/build.sh`, import), corriger, puis capture.
