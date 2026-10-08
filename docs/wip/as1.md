# AS1 — bêtes animées (campagne + chevaux du camp)

Branche `feat/as1` (worktree `../gp-as1`). Doctrine : `docs/design/2026-10-08-sources-animation.md` § 2-3, ADR 0187 ; décision du lot : ADR 0188.

## État
- Fait : données `data/fx/animal_motion.json` + schéma + test Python ; `animal_motion.gdshaderinc` (pas 4 temps, tête, queue, souffle, roues, cahot) inclus par `folk_prop.gdshader` ; `camp_horse.gdshader` branché dans `battle_decor.gd` (`_horse_line`) ; `AnimalMotion` ; test `game/tests/as1_test.gd` OK.
- Planche `as1_shot.gd` (GPU) : écarts d'images non nuls avec le lot, nuls (idle, camp) avec `--no-as1`.
- A/B : `--no-as1` (ou `enabled:false`).

## Prochaine étape
1. Fusion de main, smoke, rapport. Le regard humain sur `game/tests/as1_shot.gd` reste à faire (session principale).

## Points ouverts
- (aucun)
