# CB-M1 — Contours de formation en décales (état)

Branche : `feat/cb-m1-outline`. Plan : `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`
(section CB-M1). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`
(« Contour de formation »).

## État

- [x] Squelette `game/scripts/battle/battle_formation_outline.gd` (API publique, vide).
- [ ] Implémentation (textures, décales, états, pulsé).
- [ ] Branchement dans `battle_scene.gd`, suppression de `_rings` et de `BattleMeshes.outline`.
- [ ] Test `game/tests/cb_m1_outline_test.gd`.
- [ ] Capture `game/tests/cbm_outline_shot.gd`.

## Prochaine étape

Implémenter `outline_state` et la génération des textures.
