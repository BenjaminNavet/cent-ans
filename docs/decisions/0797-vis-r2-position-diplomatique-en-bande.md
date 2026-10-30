# 0797 — Position diplomatique en bande intérieure en vue 3D (VIS-R2)

Numéro attribué à la fusion. Amende l'ADR 0175 (lisibilité du territoire) ; reprend le constat G1-4 de
`docs/audits/2026-10-29-visuel-campagne.md`.

## Contexte

Le lavis de position (ADR 0175, `alpha.enemy 0.26`, `self 0.34`) couvre tout le territoire dès la distance 260. Posé sur un
sol déjà sombre, il remplace le terrain au lieu de le teinter : la Guyenne et le Kent disparaissent sous un aplat rouge.

## Décision

- En vue 3D (mode `political`, lavis de position actif), sous `zoom.flat_from_distance` (900), la position ne se lit que par
  une **bande intérieure dégradée** de 4 à 8 px écran le long du bord de chaque région de même couleur. Cœur de la province :
  aucun lavis (ni teinte héraldique, ni position).
- L'aplat d'ADR 0175 revient en fondu sur `flat_fade_distance` (300) après `flat_from_distance`, et pleinement dès que le
  parchemin est en force (`strategic`). À d 2 600 le calcul est identique à l'ancien (poids d'aplat 1).
- Distance au bord : 8 lectures (4 directions, rayons 1 et 2 texels) de la texture de remplissage déjà liée ; aucun
  échantillonneur ajouté. Le bord est donc celui de la texture au quart du raster (précision de 4 unités carte) : la bande peut
  s'écarter du ruban de frontière, tracé au raster plein, de moins de 2 unités.
- Les couleurs de bande (alpha au bord par position) vivent dans une seconde texture de même taille (binding 6), peinte avec
  le remplissage ; elle est vide hors du mode `political`.
- Réglages dans `data/map/stance_fill.json` (`band.width_px`, `band.alpha`, `band.falloff`, `zoom.flat_from_distance`,
  `zoom.flat_fade_distance`), schéma `data/schemas/stance_fill.schema.json`.
- Les lentilles (religion, loyauté, ravitaillement, etc.) et le mode diplomatie sont inchangés : le joueur les choisit.

## Conséquences

- Le terrain des voisins reste lisible ; la lecture « à qui est cette province » passe par les bords, les plaques et le
  survol.
- Une région de même couleur (deux provinces d'un même camp et d'un même suzerain) n'a pas de bande entre elles.
- Une texture RGBA8 de plus (un quart de raster) et 9 lectures par fragment du lavis, 0 hors du mode `political`.
