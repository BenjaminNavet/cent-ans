# AN1a — Mouvement secondaire en shader (état)

Branche `feat/an1a-secondary-motion` (worktree agent). Orchestration : `docs/wip/an1-animation-vivante.md`.
ADR : `docs/decisions/0096-animation-vivante.md` (partie A ; partie B = AN1b).

## Principe
- Pièces souples reconnues sans recuisson (analyse des maillages fins) :
  - bas du surcot/jaque (codes livrée et gambison) : poids = part des os des jambes
    (0 à la taille, ~0,85 à l'ourlet) ;
  - caparaçon : code armoiries + os du cheval (`sever_bones[0]`), poids selon la hauteur de
    repos (ourlet 0,48 m → selle ~1,45 m) ;
  - queue : os `Tail1-7`, poids = rang dans la chaîne ;
  - crinière : robe du cheval à la teinte exacte des crins (0,33 → 84/255 en 8 bits ; ~7 sommets
    du corps partagent la teinte, déplacement de quelques mm).
- Moteurs : vitesse du régiment (`_speed` de BattleSoldiers → `move_speed`), vent de la bataille
  (`battle_finish.json` `wind`, via `BattleSecondaryMotion.set_wind`), amplitudes dans
  `data/fx/atmosphere.json` `secondary_motion` (schéma `fx_atmosphere.schema.json`).
- Code : `game/scripts/battle/battle_secondary_motion.gd` (uniformes), partie sommets de
  `battle_soldier_skinned.gdshader`, `battle_standard_flag.gdshader`.

## État
- [x] Données + schéma + script d'uniformes (squelette)
- [ ] Shader soldats (sommets)
- [ ] Branchement BattleSoldiers (vitesse), scène (vent), étendards (figurines + étoffes)
- [ ] Shader d'étendard : amplitudes des données, vent apparent de l'allure
- [ ] Tests : smoke, bv3_check, fg3_maps_test ; test dédié
- [ ] Banc FG5 (standard, ≤ +2 %)
- [ ] Captures (≤ 3) `docs/img/an1/`
- [ ] ADR partie A

## Prochaine étape
Shader soldats.
