# 0137 — Décor de campagne : arbres en cartes et imposteurs, herbe proche, ombres par préréglage

Date : 2026-09-30. Statut : acceptée (autonomie donnée par le joueur).
Spec : `docs/superpowers/specs/2026-09-30-fc-fps-et-decor-campagne-design.md`. Suite de l'ADR 0123.

## Contexte

Saccades au zoom et au déplacement sur la carte (rendu, 8,6 M primitives à d = 30) ; arbres
proches lus comme des boules low-poly ; aucune herbe en volume ; ombres des maquettes jusqu'à
500 quel que soit le préréglage.

## Décision

1. **Ombres par préréglage** : `model_shadow_distance` (Basse 0, Moyenne 150, Haute 250,
   Ultra 500) ; moulins ombrés seulement sous `veg_shadow_distance`.
2. **Arbres en trois paliers** : cartes de feuilles (~150-250 triangles) sous `near_distance`
   45, imposteurs cuits dans Blender (8 azimuts, albédo + normale + AO, 2 triangles) au-delà,
   fondu tramé de 8 unités entre les deux ; `ForestDetail` suit la même règle. `--no-fc2`,
   `--no-fc5` pour les A/B.
3. **Herbe et broussailles** en cartes croisées par cellule de 32 unités sous d = 40
   (`clutter_density` par préréglage), semis sur fil de fond, installation ≤ 0,2 ms par cellule.

## Conséquences

- Primitives mesurées (1280×720) : d = 150 5,20 → 4,39 M ; d = 25 7,73 → 8,68 M (herbe et
  cartes, sous le plafond fixé de +2 M) ; appels de dessin quasi inchangés (842 → 845).
- Temps d'image : banc Metal A/B à refaire sur machine calme (`docs/wip/fc.md`).
- Ouvert : herbe encore discrète ; clé de préréglage pour étendre la portée des arbres (700).
