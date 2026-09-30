# NT9 — équilibre après N6/N7

Branche `feat/nt9-balance` (worktree agent). Lot NT9 de `docs/wip/nt.md`. ADR 0128 (révision NT9),
ADR 0127 (sorties). État : **terminé**, à fusionner.

## Fait
- Sonde : `BATTLE_TRACE=1` (`century_probe`) imprime chaque ligne « bataille FR/EN » comptée ;
  `ARMY_COUNT=1` le nombre d'armées de campagne FR+EN par tour. Classement hors dépôt (scripts
  du scratchpad : naval, assaut, terrain FR–EN / autres, avis, traditions, répétitions).
- Missions : une sortie de garnison compte comme bataille gagnée pour son vainqueur
  (`siege::sortie` → `missions::note_battle_won`) ; test `a_won_sortie_counts_as_a_won_battle`.
- Bélier en résolution automatique : `siege_engines.json` `auto_assault_bonus_percent` (20),
  `BattleContext.assault_bonus_percent`, `assault_odds` (IA/UI), prévision (« Porte enfoncée par
  le bélier ») ; tests `a_ready_ram_helps_the_auto_resolved_assault`, `a_broken_gate_softens_the_walls`.
- IA : une seule attaque par armée ennemie et par tour (`GridPlanner::attack_order_sparing`) ;
  test `one_attack_per_enemy_army_and_turn`.
- `cv3_ai_stances` : la graine 1 retrouve son embuscade → test rétabli sur la graine 1 seule.
- fmt, clippy -D warnings, `cargo test --workspace` (176 résultats ok, 0 échec), pytest 1283,
  smoke OK. Garde-fous bataille (`ep7_historical`, `ep9b_duel`, `ai`) : sim-battle non touché,
  verts dans le test workspace.

## Chiffres (`century_probe 464`, normale ; graines 1-6 | 7-12)
Le compteur « batailles FR/EN / déc. » compte toute ligne de journal de genre Bataille qui nomme
France ou Angleterre (terrain, y compris contre des tiers, assauts, avis « pendant le tour de… »,
victoires héroïques, rangs de traditions, embuscades, navales ; pas les sorties).

| | avant f36689196 | main 35f40385d | bélier seul | NT9 (final) |
|---|---|---|---|---|
| batailles FR/EN / déc. | 77,5 \| 60,1 | 129,1 \| 138,5 | 129,6 \| – | 106,4 \| 124,7 |
| batailles de terrain | 2791 \| 2181 | 4781 \| 5073 | 4823 | 3697 \| 4676 |
| même province et saison | 17 \| 18 % | 31 \| 30 % | 32 % | 15 \| 17 % |
| coalitions (armées alliées) | 405 \| 377 | 1624 \| 1427 | 1482 | 248 \| 310 |
| guerre FR–EN moy. | 67,6 \| 69 % | 68 \| 69 % | 67 % | 64 \| 64 % |
| graines dans 55-75 % | 6/6 \| 6/6 | 5/6 \| 6/6 | 6/6 | 5/6 \| 5/6 |
| sièges réussis | 44 \| 42 % | 39 \| 40 % | 39 % | 43 \| 36 % |

Décomposition graines 1-6 (/déc., avant → main) : terrain autres adversaires 28,1 → 46,1 ; terrain
FR–EN 12,0 → 22,6 ; victoires 16,2 → 24,8 ; avis « pendant le tour » 11,3 → 21,1 ; traditions
4,0 → 5,7 ; assauts 3,7 → 5,1 ; embuscades 1,3 → 2,6 ; navales 0,8 → 1,0.

Variante écartée : épargne limitée aux armées placées avec le premier assaillant : 132,5/déc.
(graines 1-6), sans effet.

Par époque (batailles de terrain, graines 1-6, 1337-76 / 1377-1416 / 1417+) : avant 616 / 982 / 1193 ;
main 680 / 1614 / 2487 ; NT9 692 / 1288 / 1717. Sur 1337-1377 (graines 1-3, 160 tours) : armées
FR+EN de campagne 6,7 → 9,1 par tour, batailles 354 → 327.

## Cause et décision
1. Artefact (corrigé) : sous le plafond, chaque armée d'un ost avait son ordre `Attack` ; plusieurs
   attaquaient tour à tour la même armée ennemie ou ses restes.
2. Reste (admis, ADR 0128) : ×1,68 sur 12 graines, tardif : le plafond coupe en plusieurs osts ce
   qui était une armée unique de 58 000 hommes ; batailles plus disputées (« 0 contre n » 50 % →
   36-40 %). Au-delà de la cible ~1,3× : justification écrite dans l'ADR.
3. Guerre FR–EN 64 % en moyenne (dans la cible) mais 10/12 graines dans la bande (51 % graine 6 :
   l'Angleterre vise une autre revendication pendant 130 tours ; 76 % graine 12) ; main 11/12.

## Points ouverts
- Guerre FR–EN −4 points avec la correction IA : à surveiller en partie pilote.
- Si le nombre de batailles gêne en partie pilote : levier suivant = relever `attack_ratio`
  (`data/ai/grid.json`) ou faire marcher ensemble les armées d'un ost (règle de renfort à écrire).
