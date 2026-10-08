# AS1 — bêtes animées (campagne + chevaux du camp)

Branche `feat/as1` (worktree `../gp-as1`). Doctrine : `docs/design/2026-10-08-sources-animation.md` § 2-3, ADR 0187 ; décision du lot : ADR 0188.

## État
- Squelette : API `AnimalMotion` (game/scripts/visual/animal_motion.gd), données `data/fx/animal_motion.json` + schéma, shaders vides.

## Prochaine étape
1. Shader de sommets `animal_motion.gdshaderinc` (pas, tête, queue, souffle) inclus par `folk_prop.gdshader`.
2. Shader `camp_horse.gdshader` + branchement dans `battle_decor.gd` (`_horse_line`).
3. Test `game/tests/as1_test.gd`, planche `as1_shot.gd`, smoke.

## Points ouverts
- (aucun)
