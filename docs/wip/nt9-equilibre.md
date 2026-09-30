# NT9 — équilibre après N6/N7

Branche `feat/nt9-balance` (worktree agent). Lot NT9 de `docs/wip/nt.md`. ADR 0128 (à compléter).

## État
- [x] Sonde : `BATTLE_TRACE=1` sur `century_probe` imprime chaque ligne « bataille FR/EN » comptée
  (script de classement hors dépôt : naval, assaut, sortie, terrain FR–EN / autres, notices, traditions)
- [x] Sortie de garnison comptée pour les missions (`siege::sortie` → `missions::note_battle_won`), test `a_won_sortie_counts_as_a_won_battle`
- [x] Bélier en résolution automatique : `auto_assault_bonus_percent` (20) dans `siege_engines.json`,
  `BattleContext.assault_bonus_percent`, `assault_odds` (IA) et prévision ; tests
  `a_ready_ram_helps_the_auto_resolved_assault`, `a_broken_gate_softens_the_walls`
- [x] Mesure avant (f36689196) / main, décomposition
- [x] Correction IA : une seule attaque par armée ennemie et par tour (`GridPlanner::attack_order_sparing`)
- [ ] Mesure branche (sonde `spare` en cours) ; `ram` (bélier seul) mesurée aussi
- [ ] `cv3_ai_stances` graine 1
- [ ] Garde-fous, fmt/clippy/test workspace, pytest, smoke ; ADR 0128

## Chiffres (`century_probe 464 1-6`, normale)
Le compteur « batailles FR/EN / déc. » compte toute ligne de journal de genre Bataille qui nomme
France ou Angleterre : batailles de terrain (y compris contre des tiers), assauts, avis « pendant le
tour de… », victoires héroïques/décisives, rangs de traditions, embuscades, navales. Les sorties n'y
entrent pas (leur ligne ne nomme pas de faction).

| type (/déc.) | avant f36689196 | main 35f40385d |
|---|---|---|
| total | 77,5 | 129,1 |
| terrain, autres adversaires | 28,1 | 46,1 |
| terrain FR–EN | 12,0 | 22,6 |
| victoires (flavour) | 16,2 | 24,8 |
| avis « pendant le tour » | 11,3 | 21,1 |
| rangs de traditions | 4,0 | 5,7 |
| assauts | 3,7 | 5,1 |
| embuscades | 1,3 | 2,6 |
| navales | 0,8 | 1,0 |

Batailles de terrain : 2791 → 4781 ; avec armées alliées (coalition) 405 → 1624 ; même province
et même saison qu'une bataille précédente 17 % → 31 %. Guerre FR–EN main : 68 % [61-83], 5/6.

Cause : sous le plafond (N6) un ost est plusieurs armées au même endroit ; chacune reçoit son
propre ordre `Attack` sur l'ennemi le plus proche : la 1re bat l'ennemi, les suivantes attaquent
tour à tour la même armée ou ses restes (« 0 contre 8 », « 0 contre 4 »). Avant N6 elles étaient
fusionnées en une armée et ne livraient qu'une bataille.

## Prochaine étape
Lire `spare-probe.txt` (scratchpad) avec `classify.py`/`sizes.py` ; décider ; ADR 0128.
