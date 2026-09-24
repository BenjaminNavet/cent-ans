# WIP — S1 effondrement physique des murailles

Spéc : consigne du lot S1 (visuel seulement), contexte `docs/design/m8-sieges.md` ; ADR 0007.

## État
- [x] Données `data/fx/siege_fx.json` + schéma `data/schemas/siege_fx.schema.json` + pytest
  `tools/tests/test_siege_fx_schema.py`.
- [x] ADR `docs/decisions/0007-physique-rendu-seulement.md`, Jolt activé dans `game/project.godot`.
- [ ] `game/scripts/battle/wall_collapse_fx.gd` (squelette commité, implémentation en cours).
- [ ] Appels depuis `battle_siege.gd` (diff minimal, lot B4 en parallèle).
- [ ] Test headless `game/tests/wall_collapse_fx_test.gd`.
- [ ] Smoke vert, capture avant/après.

## Prochaine étape
Implémenter la fracture (grille décalée, graine = index du pan), les corps, la poussière, la
secousse, les paliers du parapet, le plafond de corps actifs.
