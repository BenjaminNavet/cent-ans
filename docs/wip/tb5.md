# TB5 — mer et côtes (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB5 ». Branche `feat/tb5`, worktree
`/Users/jean_hubert/dev/gp-tb5`. Rendu Godot seulement (`core/` intact). Pas de fal.ai (ADR 0152).

## État
- [x] Squelette : cette note, `game/tests/tb5_coast_test.gd` (contrôles désactivés).
- [ ] 1. Falaises (craie, granite) et plages (sable, galets) : région → type de côte dans `data/map/`.
- [ ] 2. Mers différenciées par bassin (teinte, houle), valeurs dans `data/`.
- [ ] 3. Ressac animé au trait de côte.
- [ ] Mesures `ss_shot.gd --stats` et `--bench` avant / après ; `game/tests/tb5_shot.gd`.

## Prochaine étape
Mesures « avant » (Douvres, Atlantique, Méditerranée), puis point 1.

## Points ouverts
(aucun pour l'instant)
