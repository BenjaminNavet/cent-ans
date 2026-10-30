# 0811 — Rivières lisibles à toute distance (lot RIVIERES)

Statut : accepté (30/10/2026). Numéro à confirmer à la fusion.

## Contexte

Les fleuves (Seine, Marne, Loire...) disparaissaient de la carte de campagne à d 600 et 900, avant comme après VIS-R5 (ADR 0806).
Hypothèses de VIS-R5 (disque de géo fine, `generalised.width_scale`) écartées : à d 600 le disque fin est nul (`fine_zone` = -1)
et le dump `river_bands` donne 6 à 8 px dessinés. Un fragment magenta opaque a montré la géométrie à l'écran : c'était l'alpha.

## Décision

1. `river.wesl` : `edge` (0 sur l'axe, 1 au bord de l'eau) est normalisé par la demi-largeur d'eau dessinée `sw`, et `half_px`
   devient les pixels de demi-largeur d'eau (`sw / fwidth(side)`). Avant, `side` (unités de demi-largeur vraie) était comparé à 1
   alors que le plancher en pixels élargit l'eau (`sw` > 1 dès d 200 pour la Seine) : `px_edge` tombait à 0 et l'adoucissement
   de bord annulait l'alpha de l'eau, sauf sur un filet central. Plus le plancher monte (VIS-R5 : 4 -> 6 px), plus c'était grave.
2. `data/map/river_display.json` `look.far_color` assombri (0,15 0,27 0,38) : le bleu-gris lointain avait la luminance du sol vert.

## Conséquences

- Les largeurs en pixels de `major_min_px` et de l'échelle par importance sont enfin celles que l'on voit à toute distance.
- Les berges (`look.riparian`) vivent dans la même bande de `wet_band_px` (12 px) ; elles ne sont lisibles qu'au palier moyen proche.
- Hors périmètre : `terrain.wesl` (VIS-R3b), palier fin.
