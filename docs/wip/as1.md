# AS1 — bêtes animées (campagne + chevaux du camp)

Branche `feat/as1` (worktree `../gp-as1`). Doctrine : `docs/design/2026-10-08-sources-animation.md` § 2-3, ADR 0187 ; décision du lot : ADR 0188.

## État
- Fait : données `data/fx/animal_motion.json` + schéma + test Python ; `animal_motion.gdshaderinc` (pas 4 temps, tête, queue, souffle, roues, cahot) inclus par `folk_prop.gdshader` ; `camp_horse.gdshader` branché dans `battle_decor.gd` (`_horse_line`) ; `AnimalMotion` ; test `game/tests/as1_test.gd` OK.
- A/B : `--no-as1` (ou `enabled:false`).

## Prochaine étape
1. Planche `game/tests/as1_shot.gd` (GPU) et comparaison d'images numérique.
2. Smoke, fusion de main, rapport.

## Points ouverts
- (aucun)
