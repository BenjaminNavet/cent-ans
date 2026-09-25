# ADR 0054 — Une trêve lie les alliés entrés dans la guerre (lot EQ3)

Date : 2026-09-25. Statut : accepté.

## Contexte

Sur main (EQ1, EQ2, DF1, DP1), la guerre France-Angleterre couvre 56 % du siècle
(`century_probe` 5 × 464 : 49/49/61/65/57 %). La sonde `dp1_probe` montre pourquoi les paix
duraient 5 à 8 ans : quand la France et l'Angleterre signaient, les alliés de la France entrés
sur appel (Bretagne, Bourgogne, Naples, Écosse…) restaient en guerre contre l'Angleterre. Sa
fatigue de guerre montait donc en « paix » (jusqu'à 100) et son trésor ne se refaisait pas ; le
prétendant ne pouvait plus redéclarer la guerre (fatigue > 70, une saison d'entretien en caisse).
Monter `pretender_reluctance` (EQ2) ne jouait que sur la fin des guerres, pas sur ces paix
longues.

Historiquement, les grandes trêves liaient les alliés des deux couronnes : Leulinghem (1389)
comprenait l'Écosse, la Castille et les alliés de l'Angleterre.

## Décision

1. `make_peace` (`sim-campaign/src/diplomacy.rs`) : quand `negotiation.truce_binds_allies` est
   vrai (`data/ai/diplomacy.json`), une paix entre A et B signe aussi la même trêve pour chaque
   allié ou vassal de l'un des deux, **plus faible que lui**, en guerre contre l'autre depuis le
   début de cette guerre ou après (il a répondu à l'appel). Une guerre plus ancienne de l'allié
   (propre querelle) continue.
2. Le partenaire le plus fort n'est pas lié : une paix anglo-écossaise ne met pas fin à la guerre
   franco-anglaise (première itération rejetée : la guerre FR-EN tombait à 32 %).
3. `weariness_unrest_divisor` 5 → 4 : la fatigue de guerre, qui retombe désormais en paix, pèse
   un peu plus sur l'ordre public (sinon les révoltes tombaient à 3,9 par partie, sous 4-10).
4. Défaut sans le fichier : `false` (comportement antérieur).

## Conséquences

- Guerre FR-EN 56 % → ~65 % au niveau normal, 4/5 graines dans 55-75 %, 10 à 14 paix par
  siècle, plus longue guerre 8 à 12 ans (mesures détaillées : `docs/wip/eq3-equilibre.md`).
- Moins de guerres résiduelles : banqueroutes en baisse (0,30 → ~0,1 / faction / décennie).
- L'Écosse n'est détruite dans aucune graine (déjà le cas sur main).
- La trêve de l'allié est une paix ordinaire : un événement « Paix entre … » par allié lié.

## Addendum (lot EQ4, 2026-09-26) — mesure combinée, aucune règle changée

Mesure de EQ1, EQ2, EQ3, DP2, DF1, C4/C5 et SG4 ensemble (`century_probe`, nouveau tableau
« EQ4 », détail : `docs/wip/eq4-equilibre-combine.md`). Niveau normal, 10 graines × 464 tours :
guerre FR-EN **60 % [52-67]**, 8/10 graines dans 55-75 % ; **12,5 trêves** par siècle [10-16] ;
banqueroutes **0,17** / faction / décennie ; révoltes 5,1 par partie de 200 tours
(`balance_probe`, 16 graines) ; un siège sur trois se termine par une prise ; la 1re faction
tient 25 % des provinces en 1437 comme en 1453 (34 % au pire) : pas de boule de neige ; 0,3
faction mineure éliminée par partie, aucune majeure.

- SG4 ne touche que la bataille de siège 3D ; la campagne IA contre IA n'en est pas changée.
- DP2 : l'IA accorde ~120 accès militaires par siècle ; désactiver le passage laisse la guerre
  FR-EN inchangée au niveau normal (59 %) et la remonte de 56 à 60 % en difficile. Le passage ne
  rend pas les guerres interminables.
- Difficile / très difficile : 56 % / 51 % de guerre FR-EN et une Angleterre plus grosse
  (jusqu'à 38 % des provinces), effet attendu de DF1 sur une France jouée par l'IA.

Deux correctifs de données essayés puis annulés faute de dérive réelle (Paix de Dieu adoucie ;
entretien IA en facile).
