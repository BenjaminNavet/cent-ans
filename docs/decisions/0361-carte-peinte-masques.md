# 0361 — Carte peinte : retrait du calculé et peinture vectorielle régionale

Statut : accepté (14/10, lot CP0). Complète l'ADR 0358 (cuissons du sol) et l'éditeur de carte (lot CE).

## Contexte
Les masques d'auteur (`masks/{forest,rock,fields,marsh}.png`, 1 case = 16 px composés) ne permettaient que d'ajouter
à la couverture du sol calculée : impossible d'enlever une forêt ou un marais produits par le calcul. Corriger la
carte région par région demande aussi un format lisible, daté et motivé (raison historique), que des agents peuvent
écrire sans pinceau et que l'on peut contrôler sans lancer le jeu.

## Décision
1. **Masques de retrait** `masks/<nom>_clear.png` (même format). Valeur `v` : couverture calculée × `(1 - v)`
   (forêt : couvert et splat B ; roche : lande et splat A ; champs : splat G ; marais : canal R des zones humides).
   Les parts retirées du splat reviennent à l'herbe (somme des poids conservée). Le retrait passe avant l'ajout ;
   absent = aucun effet.
2. **Peinture vectorielle** `data/map/compose/paint/<region>.json` (schéma `compose_paint.schema.json`) : traits
   `{id, layer, mode add|clear, strength, feather_km, note, coords lonlat|px, polygon}`. `cent-ans compose paint`
   rastérise (lon/lat -> px composés par le cartogramme, rampe lisse de `feather_km` de part et d'autre du
   contour, max des traits par couche et mode) vers `masks/derived/<nom>.png` et `<nom>_clear.png`. Le pinceau
   de l'éditeur continue d'écrire `masks/*.png` : `masks.load_all` prend le max des deux sources, aucune ne
   écrase l'autre. `--preview out.png` écrit une image 443 x 382 (couleur par couche, damier pour les retraits,
   fond = masque de terre) pour un contrôle sans le jeu.
3. **Recompiler** (éditeur) enchaîne `geo compose`, `geo compose-relief`, `compose paint` puis
   `geo compose-layers --only landcover,colormap` ; `recompile_steps` reste surchargeable.
4. **Calque du calculé** : `GET /api/cover/<couche>.png` renvoie la couverture calculée actuelle (splat.png /
   wetlands.png) réduite à la grille des masques ; case à cocher dans l'éditeur.

## Conséquences
- Les masques dérivés sont des fichiers générés (PNG minuscules) : à versionner avec les traits qui les produisent.
- Le calque de couverture reflète la dernière cuisson, pas les traits non cuits.
- Le parcellaire `agri_regions` ne lit pas encore les masques de champs (seulement le splat G) ; à brancher si besoin.
- Les polygones lon/lat suivent le cartogramme : si les épingles changent, ils suivent la carte (les `px` non).

## Addendum (cause racine de la couverture composée)
Avant de peindre la forêt de l'Est, de l'Anatolie ou de la Grèce, noter que le défaut venait du calcul, non d'un manque de
peinture : `open_plains` ouvrait toute plaine plate (plaine russe, Carélie, Pologne) et le calibrage global `forest_share`
reportait le déficit sur les terres vallonnées du Sud. Corrigé : l'ouverture dépend désormais de la part défrichée KK10.
Les traits peints sont des **corrections locales** sur un calcul désormais plausible ; après la bake corrigée, re-mesurer
avant d'ajouter des traits de forêt « de masse » (`forest add` sur de grandes régions) ou de retrait « sans forêt ».

## Addendum CP3a : forêt peinte en massifs
Une valeur peinte `v` du masque `forest` relève la part boisée visée et non plus le couvert lisse : la forme vient d'un
bruit à massifs (rang uniforme, échelles 26/70/170 px comme `compute_forest`) seuillé à `1 - v`, bords compris. `rock add`
reste inchangé (mélangé à l'érosion continue). Aucun effet sur les autres couches.
