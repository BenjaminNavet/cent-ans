# 0215 — Eau des lacs façon mer ME1, routes maritimes discrètes

## Contexte
Vue Bretagne : faisceau de traits sombres épais en mer (routes maritimes des couloirs Manche/Gascogne tracées l'une sur l'autre, tirets en phases différentes). Lacs : aplat bleu à bord polygonal (contour simplifié de `lakes.json`).

## Décision
- `sea_lane_layer.gd` : tronçons communs dédupliqués au rendu (routes longues d'abord, distance de fusion 22 px carte), traits 1,2 à 2,2 px, encre à 50-65 % d'opacité. Les tracés complets restent utilisés pour infobulle et trajets.
- Lacs (`river_water.gdshader` mode `sheet`) : rive lue dans le raster `coast_dist` (fondu bruité sur la vraie côte au lieu du polygone, polygone élargi de 3 px), fonds clairs près de la rive, taches de teinte et reflets du ciel par plaques ; couleurs plus profondes.

## Conséquences
Rendu seulement, aucune règle touchée. Réglages : `MERGE_DISTANCE`, `INK*` (routes) ; `deep_color`, `sheet_sky_color` (lacs).
