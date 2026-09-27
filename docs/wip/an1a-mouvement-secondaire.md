# AN1a — Mouvement secondaire en shader (état)

Branche `feat/an1a-secondary-motion` (worktree agent). Orchestration : `docs/wip/an1-animation-vivante.md`.
ADR : `docs/decisions/0096-animation-vivante.md` (partie A ; partie B = AN1b).

## État : TERMINÉ 27/09 (branche prête, main fusionné, non fusionnée dans main)

## Principe
- Pièces souples reconnues sans recuisson (analyse des maillages fins) :
  - bas du surcot, de la jaque ou de la cotte (codes livrée, étoffe, gambison) : poids = part
    des os des jambes (0 à la taille, ~0,85 à l'ourlet) ; chausses (étoffe ≥ 0,99 sur les
    jambes) rigides ; au LOD2 l'ourlet du surcot est entièrement sur les jambes (gardé) ;
  - caparaçon : code armoiries + os du cheval (`sever_bones[0]`), poids selon la hauteur de
    repos (ourlet 0,48 m → selle 1,45 m) ;
  - queue : os `Tail1-7`, poids = rang dans la chaîne ;
  - crinière : robe du cheval à la teinte exacte des crins (0,33 → 84/255 en 8 bits ; ~7 sommets
    du corps partagent la teinte, déplacement de quelques mm).
- Moteurs : vitesse du régiment (`_speed` de BattleSoldiers → `move_speed`), vent de la bataille
  (`battle_finish.json` `wind`, via `BattleSecondaryMotion.set_wind`), amplitudes dans
  `data/fx/atmosphere.json` `secondary_motion` (schéma `fx_atmosphere.schema.json`).
- Code : `game/scripts/battle/battle_secondary_motion.gd` (uniformes), partie sommets de
  `battle_soldier_skinned.gdshader`, `battle_standard_flag.gdshader` (onde des données,
  ondulation le long de la hampe, vent apparent de l'allure du porteur).
- `--no-an1a` : tout coupé (banc A/B ; étendards = ancien comportement).

## Tests
`an1a_motion_test.gd` (défaut et `--coarse-figures`), smoke, `bv3_check`, `fg3_maps_test`
(défaut et `--coarse-figures`), `test_atmosphere_schema.py` : OK.

## Banc (Ultra, 1600×900, `--units=50 --bench-at=90`, passes alternées, durée moyenne d'image)
Machine partagée avec d'autres agents : bruit de ±10 % sur une passe isolée.
| | avec AN1a | `--no-an1a` | écart |
|---|---|---|---|
| standard (5 passes, médiane) | 19,83 ms | 20,04 ms | -1 % (non mesurable) |
| rapproché `--closeup` (5 passes, médiane / min) | 19,22 / 18,81 ms | 19,02 / 18,32 ms | +1,0 % / +2,7 % |
Première version (boucles `pick`, 4 sinus, 160 m) : rapproché +9,8 % → poids vectorisés
(`greaterThanEqual` · poids), un sinus de rafale, portée 100 m (1 px ≈ 8 cm à 110 m).

## Captures
`docs/img/an1/an1a_closeup.png` (mêlée 7 m), `docs/img/an1/an1a_mounted_standard.png`
(porte-étendard monté) : aucun artefact (déchirure, caparaçon décollé). Une image fixe ne montre
pas le mouvement : à juger en jeu.

## Points ouverts
- Jugement du mouvement en jeu par le joueur (amplitudes dans les données).
- Vitesse par régiment, pas par soldat ; `TIME` : le flottement continue en pause.
- Les porte-étendards/musiciens n'ont que le vent (pas de vitesse) ; les étoffes des drapeaux
  ont le vent apparent de l'allure.
- Un vrai masque cuit (recuisson `battle_fine.py`) supprimerait l'heuristique de teinte de la
  crinière.
