# TB5 — mer et côtes (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB5 ». Branche `feat/tb5`, worktree
`/Users/jean_hubert/dev/gp-tb5`. Rendu Godot seulement (`core/` intact). Pas de fal.ai (ADR 0152).

## État
- [x] Squelette : cette note, `game/tests/tb5_coast_test.gd` (contrôles désactivés).
- [x] 1. Falaises (craie, granite, roche) et plages (sable, galets) : `data/map/coast_types.json` (régions en px carte, règle de pente, couleurs), schéma `coast_types.schema.json`, `tools/tests/test_coast_types.py` ; `CoastLook` (`game/scripts/map/coast_look.gd`) cuit la texture de géologie et pose les réglages ; `coast_band.gdshaderinc` + `coast_common.gdshaderinc`, un crochet `coast_band(...)` dans `terrain.gdshader`, une ligne dans `terrain_builder.gd`. Matières procédurales (aucune texture ajoutée, pas de ligne `CREDITS.md`).
- [x] 2. Mers par bassin : `data/map/sea_basins.json` (Atlantique, mer du Nord et Manche, Méditerranée, Baltique ; hors bassin : mer par défaut), schéma `sea_basins.schema.json`, `tools/tests/test_sea_basins.py` ; `SeaBasins` (`game/scripts/map/sea_basins.gd`) cuit la texture de poids (limites fondues) ; `sea_basins.gdshaderinc` inclus par `water.gdshader` (teinte, clarté, clapot, longue houle, écume, moutons) ; la saison TB1 s'applique par-dessus (`grey_scale` par bassin).
- [ ] 3. Ressac animé au trait de côte.
- [ ] Mesures `ss_shot.gd --stats` et `--bench` avant / après ; `game/tests/tb5_shot.gd`.

## Prochaine étape
Point 3 (ressac), puis mesures « après » et bench.

## Points ouverts
(aucun pour l'instant)
