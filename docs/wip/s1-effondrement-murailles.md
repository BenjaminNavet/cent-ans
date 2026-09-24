# WIP — S1 effondrement physique des murailles

Spéc : consigne du lot S1 (visuel seulement), contexte `docs/design/m8-sieges.md` ; ADR 0007.

## État
- [x] Données `data/fx/siege_fx.json` + schéma `data/schemas/siege_fx.schema.json` + pytest
  `tools/tests/test_siege_fx_schema.py`.
- [x] ADR `docs/decisions/0007-physique-rendu-seulement.md`, Jolt activé dans `game/project.godot`.
- [x] `game/scripts/battle/wall_collapse_fx.gd` : fracture procédurale (assises décalées, blocs
  ~cubiques, `blocks_per_meter` borné à [blocks_min, blocks_max], graine = index du pan), merlons
  en corps, planches de porte, pierres du parapet aux paliers 0,75 / 0,5 / 0,25, poussière
  (GPUParticles3D, bouffées rondes), secousse caméra (h/v offset), sol local HeightMapShape3D,
  couche de collision dédiée, plafonds actifs/conservés, modes free / freeze / sink.
- [x] Appels depuis `battle_siege.gd` (7 lignes + `add_child(door, true)` pour que le second
  vantail, auparavant nommé `@MeshInstance3D@…`, se cache aussi).
- [x] Test headless `game/tests/wall_collapse_fx_test.gd` (vert).
- [x] Smoke vert sur l'état final (exit 0).
- [x] Captures `docs/img/s1/s1-avant.png`, `s1-chute.png`, `s1-apres.png`
  (`game/tests/s1_collapse_shot.gd`, option `--gate` pour la porte).

## Prochaine étape
Lot terminé, en attente de fusion. Points ouverts :
- Porte de la Guyenne invisible dans le rendu existant : le pan « gate » (14 m) est noyé entre
  les deux tours qui le flanquent, donc les planches tombent dans les tours (pas de capture).
  Problème antérieur de géométrie des tours de porte (`BattleSiege._build_tower`), hors lot.
- Pas de son d'effondrement (AudioDirector) : hors périmètre.
- Les blocs d'un pan de 90 m restent gros (≈ 4,5 × 2,8 × 3 m à 60 blocs) ; monter `blocks_max`
  si le budget de corps le permet.
