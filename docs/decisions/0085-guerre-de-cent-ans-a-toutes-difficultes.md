# ADR 0085 — La guerre de Cent Ans à tous les niveaux de difficulté (lot EQ6)

Date : 2026-09-26. Statut : accepté.

## Contexte

Décision du joueur (2026-09-26) : la cible « France et Angleterre en guerre 55-75 % du siècle »
(EQ3, ADR 0054) vaut à tous les niveaux de difficulté (ADR 0037), pas seulement en Normale : la
difficulté doit rendre la guerre plus ou moins dure pour le joueur, pas décider si elle a lieu.

Mesure sur main (`century_probe`, 464 tours, mêmes graines qu'EQ4/EQ5 ; la France, jouée par
l'IA, porte les handicaps du joueur) : facile 48 % (1/5 graines dans la bande), normale 62 %
(8/10), difficile 56 % (5/10), très difficile 50 % (1/5). Le nouveau tableau « EQ6 » de la sonde
compte, à chaque tour de paix, ce qui empêche l'Angleterre de déclarer la guerre :

- **Facile** : la raison d'attitude « Niveau de difficulté » (+10 envers le joueur) porte
  l'attitude anglaise au-dessus du seuil de la guerre de prétention (20) dans 60-70 % des tours
  de paix ; une paix de 1420 à 1449 sur la graine 4.
- **Difficile, très difficile** : l'Angleterre, plus riche, déclare sans cesse d'autres guerres
  (croisade contre Grenade, prétention héritée par mariage sur Vérone tous les trois ans) ; le
  repos de 12 tours qui suit toute déclaration bloque la guerre de France 50-80 % des tours de
  paix. `war_target` classait les prétentions par rapport de forces : la plus petite couronne
  revendiquée passait avant la France.
- **Guerres sans fin** (facile, graine 1 : 1346-1406) : l'Angleterre, battue (fatigue 100,
  score -82), ne peut rien payer ; la France gagne sans rien pouvoir prendre et refuse toute paix
  blanche (article « Paix » à -88 pour le vainqueur) ; le vainqueur ne se lasse jamais.

## Décision

Trois règles, chacune derrière un réglage de `data/ai/diplomacy.json` (défaut sans le fichier :
comportement antérieur).

1. `war.main_claim_first` : la **prétention principale** d'une couronne est le trône revendiqué
   du plus grand royaume vivant (`diplomacy::main_claim` ; l'Angleterre : la France). Sa guerre
   passe avant toute autre prétention (priorité doublée dans `war_target`) et le repos qui suit
   une autre déclaration ne la retarde pas. Les autres garde-fous restent : trêve, fatigue,
   trésor, régence, souverain captif, front déjà trop lourd, rapport de forces, attitude.
2. `war.claim_war_ignores_difficulty` : pour cette seule guerre, ni l'attitude (« Niveau de
   difficulté ») ni le rapport de forces exigé (`ai_war_ratio_percent`) du niveau ne comptent.
   La difficulté continue de peser sur toutes les autres guerres, sur l'acceptation des offres
   du joueur, sur l'économie et sur le moral.
3. `negotiation.long_war_years` / `long_war_points_per_year` / `long_war_max_points` : au-delà
   de `long_war_years` ans d'une même guerre, chaque camp — vainqueur compris — ajoute la raison
   « Guerre interminable » (points par année, plafonnés) à l'évaluation d'un traité qui y met fin
   (`negotiation::context_reasons`). Une guerre que personne ne peut gagner finit en trêve
   (Brétigny 1360, Leulinghem 1389). Valeurs : 8 ans, 8 points par an, plafond 100.

Données de difficulté (`data/rules/difficulty.json`, défaut intégré identique) : voir
« Réglage du très difficile » ci-dessous.

## Conséquences

Mesures détaillées : `docs/wip/eq6-guerre-toutes-difficultes.md`.

(à compléter)

## Réglage du très difficile

(à compléter)
