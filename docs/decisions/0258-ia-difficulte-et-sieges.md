# 0258 — L'IA de campagne lit la difficulté et tient ses sièges

Statut : accepté

## Contexte
Revue RX (`docs/wip/rx/ia.md`) : la difficulté ne changeait que des chiffres (revenu, entretien, moral) ; l'IA
jouait pareil à tous les niveaux. 59 % des sièges de murailles se terminaient sans prise. Pas de mémoire de cible
d'un tour à l'autre. Mesures avec `campaign_probe` (ADR 0246), 4 graines x 200 tours (`--full`), avant le lot :
363 sièges terminés (hors villages), 150 pris (41 %), 56 par paix, 157 « autres » dont l'armée vivante repartait
après ~1,5 tour dans plus de 85 % des cas.

## Décision
1. Quatre leviers de difficulté lus par `ai::campaign` (fichier `data/rules/difficulty.json`, champs optionnels,
   neutres à 100 / 0, sans effet sur le joueur) : `ai_siege_superiority_percent` (supériorité exigée pour assiéger,
   facile 130, difficile 90, très difficile 80), `ai_assault_odds_delta` (+10 / -5 / -10), `ai_decision_noise_percent`
   (erreur de jugement déterministe, constante 16 tours, sur la valeur des cibles de siège : facile 35, sinon 0) et
   `ai_aggression_delta` (-15 / +10 / +20, raids et embuscades). Aucun bonus de ressources supplémentaire :
   « difficile » joue mieux, pas plus riche.
2. Mémoire de cible : `siege_target_persistence` (1,6) et `defence_target_persistence` (1,25) multiplient la
   valeur de la place vers laquelle l'armée marche déjà (`Army.destination`) ; c'est aussi l'hystérésis des
   objectifs. L'armée ne change de cible que pour une place valant nettement plus.
3. Cause dominante des sièges abandonnés : une armée assiégeante repartait dès qu'une force hostile (une arête
   autour) dépassait 1,2 x sa puissance. `siege_hold_threat_factor` passe à 1,8 : elle tient jusqu'à une
   supériorité nette. La règle de levée de siège du cœur n'est pas touchée.
4. L'IA ne propose plus d'ordre d'embuscade avec déplacement que le cœur refuserait : l'ordre est rejoué sur
   une copie avant d'être proposé (`stances::ambush_orders`).
5. `campaign_probe` mesure les sièges : commencés, pris, paix, autres (secourus / abandonnés, armée vivante,
   durée moyenne).

## Mesures (4 graines x 200 tours)
| | sièges terminés | pris | part prise |
|---|---|---|---|
| avant | 408 | 170 | 41,7 % |
| persistance seule | 453 | 176 | 38,9 % (sans effet) |
| + `siege_hold_threat_factor` 1,8 | 411 | 199 | 48,4 % |

La mémoire de cible seule n'aide pas : la cause était le départ sur menace. Part FR-EN en guerre 68-80 % (avant
72-86 %), guerres actives 48-59 (avant 43-64) : bande EQ6 conservée. Tour 1 : voir ci-dessous.

## Tour 1 (Hongrie 301 ms, Holstein 259 ms)
Non reproduit : au temps CPU de thread (`turn_perf --sequential`, 40 tours) médiane 3,6 ms, p99 18 ms, max 39 ms ;
`turn_hotspot 1 1 fac_hungary` : planification 13 ms (premier appel) puis 4-7 ms, Holstein 2,7 ms. Les 300 ms
mesurés étaient du temps mural sur machine saturée (charge 30-110). Aucun changement de code.

## Conséquences
- Un nouveau niveau ou un autre réglage de l'IA se règle dans les données, pas dans le code.
- La part de sièges pris reste < 50 % : le reste se joue au cœur (levée de siège, secours) et à la diplomatie
  (hors périmètre).
- Les valeurs des leviers de difficulté sont des premiers réglages : à ajuster avec `campaign_probe` en jouant
  le même scénario aux quatre niveaux.
