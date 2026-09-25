# EQ3 — équilibre de campagne (guerre FR-EN, survie de l'Écosse, difficulté)

Branche : `worktree-agent-a65913a9ab64f3915` (main 9fe0550a : EQ1, EQ2, DF1, DP1).

## État : réglage retenu (v3), checks finaux

- [ ] Référence main : century_probe 5 × 464 (normal), balance_probe 8 × 200.
- [ ] Guerre FR-EN : 3/5 graines dans 55-75 %, au moins 2 trêves par siècle.
- [ ] Écosse : survie sur 5 graines.
- [ ] DF1 : niveau difficile dans la sonde.

## Référence main (9fe0550a, niveau normal)

`century_probe 464 1-5` : guerre FR-EN 49/49/61/65/57 % (moy. 56 %, 3/5 dans 55-75 %),
11-12 phases de guerre par siècle ; 4 majeures en vie en 1400 : 5/5 ; **Écosse jamais détruite**
(0/5 graines, aucune chute de majeure sur le siècle) ; banqueroutes 0,30 / fac. / déc. ;
couleuvriniers 5/5.

Diagnostic (`dp1_probe 300 1`) : chaque guerre FR-EN s'arrête par une paix que l'Angleterre
achète (or, tribut) ; après la paix, l'Angleterre reste en guerre avec les alliés de la France
entrés sur appel (Bretagne, Bourgogne, Naples, Écosse…) : sa fatigue continue de monter en
« paix » (jusqu'à 100 en 1407) et son trésor ne remonte pas, d'où des paix de 5 à 8 ans.

## Itérations
- v1 (trêve liant tous les alliés entrés après le début) : guerre FR-EN **32 %** [20-39], 15-23
  paix par siècle : la paix d'un petit allié (Écosse) entraînait la France. Rejeté.
- v2 : seul le partenaire le plus faible suit la paix du plus fort. century : guerre **65 %**
  [53-73], 4/5 dans la bande, 10-14 paix par siècle, plus longue guerre 8-12 ans, 5/5 majeures,
  Écosse jamais détruite, banqueroutes 0,11. Difficile : 60 % [51-66], 3/5. balance 8 × 200 :
  milice 30 %, trouble 17,4 / 20,4, Haut 24 %, banqueroutes 0,11, **révoltes 3,9** (cible 4-10).
- v3 (retenue) : v2 + `weariness_unrest_divisor` 5 → 4 (la fatigue de guerre pèse plus sur
  l'ordre public) pour remonter les révoltes.

## Avant / après (niveau normal ; ADR 0054)

Avant = main 9fe0550a ; après = v3. Même machine, build release.

| Mesure | Avant | Après | Cible |
|---|---|---|---|
| Guerre FR-EN, siècle 5 × 464 | 56 % [49-65] (49/49/61/65/57) | **67 % [53-75]** (70/68/68/53/75) | 55-75 % |
| Graines dans la bande | 3/5 (affiché) | 4/5 (75,4 % affiché hors bande : 3/5) | ≥ 3/5 |
| Paix FR-EN par siècle (plus longue guerre) | 10-11 phases | 9-15 (9 à 17 ans) | ≥ 2 trêves |
| 4 majeures en vie en 1400 | 5/5 | 5/5 | 5/5 |
| Écosse détruite (siècle) | 0/5 | 0/5 | 0 |
| Banqueroutes / fac. / déc. (siècle) | 0,30 | 0,12 | < 0,5 |
| Couleuvriniers recrutés | 5/5 | 5/5 | ≥ 1 |
| balance 8 × 200 : guerre FR-EN | 58 % | 66 % | — |
| balance : trouble final / moyen | 20,9 / 22,5 | 19,8 / 20,8 | 15-35 |
| balance : révoltes / partie | 7,9 | 4,2 | 4-10 |
| balance : impôt Haut | 30 % | 25 % | < 40 % |
| balance : banqueroutes | 0,29 | 0,17 | < 0,5 |
| balance : milice | 32,0 % | 30,3 % | < 40 % |

Niveau difficile (`DIFFICULTY=hard century_probe`, nouveau réglage de la sonde) : avant 56 %
[47-63], 3/5 ; v2 60 % [51-66], 3/5 ; Écosse jamais détruite, 5/5 majeures en 1400.

## Prochaine étape
Attendre la référence, puis analyser les phases de guerre FR-EN (VERBOSE=1).
