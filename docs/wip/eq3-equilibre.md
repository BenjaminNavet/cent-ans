# EQ3 — équilibre de campagne (guerre FR-EN, survie de l'Écosse, difficulté)

Branche : `worktree-agent-a65913a9ab64f3915` (main 9fe0550a : EQ1, EQ2, DF1, DP1).

## État : piste 1 en mesure (la trêve lie les alliés entrés dans la guerre)

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
- v2 : seul le partenaire le plus faible suit la paix du plus fort (mesure en cours).

## Prochaine étape
Attendre la référence, puis analyser les phases de guerre FR-EN (VERBOSE=1).
