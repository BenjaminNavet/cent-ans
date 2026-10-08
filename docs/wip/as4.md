# AS4 — servants porteurs de munitions

Branche `feat/as4` (worktree `../gp-as4`). Doctrine : `docs/design/2026-10-08-sources-animation.md` § 3. Rendu seulement.

## État
- [x] Réglages `crew.haul` dans `data/fx/siege_engines.json` + schéma `siege_engines.schema.json`.
- [x] `SiegeCrewFx.add_haulers` / `haul_stage` / `haul_pose` ; tas et objets portés en MultiMesh de primitives.
- [x] Crochet d'une ligne dans `SiegeEnginesFx._crew_engine` (phase du rechargement déjà calculée).
- [x] Test `game/tests/as4_test.gd` ; drapeau A/B `--no-as4`.
- [x] Smoke OK (32 lignes), as4_test OK avec et sans --no-as4.
- [x] main fusionné.

## Prochaine étape
Jugement visuel (session principale) : postes et tas guessés d'après les `layouts` des servants existants.
