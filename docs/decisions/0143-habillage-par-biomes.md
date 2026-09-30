# 0143 — Habillage de la carte de campagne par biomes

Date : 2026-09-30. Chantier HB (suite de SS, ADR 0142). Statut : appliquée.

## Contexte

Après SS, capture du joueur (Évreux, vue moyenne) : carte encore « vide et IGN ». Constats : relief en papier froissé (exagération verticale ×4,31 au-delà de 45 unités), arbres isolés semés partout sans massifs, champs non dessinés en vue moyenne (carte de couleur à 360 m/texel trop grossière), ni rochers ni rivières lisibles. Demandes : champs, forêts, roches, rivières ; assets générés par fal.ai ; végétation différente selon le climat (océanique, continentale, méditerranéenne, steppes…) ; forêts plus variées (sapins, hêtres, érables, pins, pommiers…).

## Décision

1. **Carte des biomes** `data/map/biomes.png` (indice 8 bits, taille de la carte 7168×6144) + légende `data/map/biomes.yaml`, cuite par `cent-ans geo biomes`. Indices figés :
   0 mer/hors carte, 1 océanique, 2 continental, 3 méditerranéen, 4 steppe, 5 boréal, 6 montagnard/alpin, 7 semi-aride.
   Elle pilote la palette de la carte de couleur (re-cuisson), les matières de sol et les essences d'arbres.
2. **Matières de sol générées (fal.ai)** : textures tuilables vues du ciel (cultures, prés, vigne, oliveraie, verger, canopées par biome, roche, éboulis, lande, steppe, tourbière), rendues raccordables par l'outil, empaquetées en tableau de textures. Le shader dessine un **parcellaire de vue moyenne** (cellules ~300–600 m) texturé par ces matières selon biome et culture, plus canopée sur les forêts et roche sur les pentes.
3. **Essences** : ~16 imposteurs 8 azimuts par la chaîne GA3 L2 (flux-2 + nano-banana-2/edit + bria), catalogue `data/art/tree_species.yaml` (essence → biomes, altitude, humidité, densité, part en lisière/bosquet/verger). Forêts denses en massifs, arbres isolés rares dans les champs, vergers près des villages, ripisylves (peupliers, saules) le long des rivières.
4. **Rochers** : affleurements générés (fal.ai TRELLIS, chaîne GA3) posés sur pentes fortes, montagne, garrigue et lande, en imposteurs/instances comme `GroundClutter`.
5. **Relief** : exagération lointaine ramenée de 4,31 à ~2,2 (réglée sur captures).
6. **Rivières** : chantier RC (ADR 0141) ; HB vérifie seulement leur lisibilité en vue moyenne et signale.

Budget fal.ai du chantier : plafond 8 $ (`docs/budget.md`, section HB).

## Conséquences

- Nouvelles données versionnées : biomes (~quelques Mo), tableau de matières (~20–40 Mo), atlas d'imposteurs.
- Le nombre d'appels de dessin des arbres ne doit pas croître de plus de 30 % (atlas commun des essences).
- Toute règle de répartition vit dans les données (`biomes.yaml`, `tree_species.yaml`), pas dans le code.

## Constat d'application (2026-09-30)

- Relief : baisser `exaggeration_far` casse l'écrasement des montagnes (les échelles cuites dépendent de ×4,31) ; retenu : gain local négatif (−0,4 loin, −0,1 près) et écrasement des montagnes dès la vue stratégique (`mountain_squash_far` 1).
- Une part importante du « gris IGN » venait des couches au-dessus du sol : brouillard de guerre (voile clair), teinte de faction de près, brume météo ; toutes adoucies.
- Coût fal.ai du chantier : 3,82 $ (matières 1,10 $, essences 2,42 $, rochers 0,30 $).
- Les affleurements 3D se lisent mal (camouflés) ; la roche du sol sur pentes (HB3) porte l'essentiel.
