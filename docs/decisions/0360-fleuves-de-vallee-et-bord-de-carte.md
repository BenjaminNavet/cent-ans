# 0360 — Fleuves de vallée (Nil, Euphrate) et bord de carte sans bord

Statut : accepté (14/10, lot CC CS2). Complète l'ADR 0355 (carte composée) et l'ADR 0358.

## Contexte
Captures jeu4 de la session principale :
- Le Nil n'était qu'un trait droit pâle (polyligne de 3-4 points de `valley_courses.json` + bande « oasis » du
  colormap) : `compile.build_rivers` ne garde que 45 fleuves (les plus longs / `must_keep`), ni le Nil ni les fleuves
  de Mésopotamie n'y figurent, donc ni eau, ni lit creusé, ni delta.
- À la caméra 1100 près d'un bord, la carte apparaît comme une dalle sur un fond gris : l'anneau de brume de CS
  (`sea.gd`, plans gris à 0,6 au-dessus de l'eau, 0,5 carte de large) est lui-même un rectangle plat visible, plus
  sombre que l'horizon du ciel, et la terre coupée au bord montre un flanc ; une bande sombre venait de l'abîme
  visible au-delà du bord (l'eau est transparente sur la terre du bord, clampée).
- Tavelures noires en désert : affleurements rocheux `faithful` (modèles tels que générés, albédo moyen 0,06-0,08)
  de `hamada`, `gypsum_badland`, `sandstone_dry` (200-500 m, biomes secs) vus de haut = ovales noirs sur le sable.
- Cumulus du ciel : cartes de nuées apparaissant à 640-960, visibles à 1100.

## Décision
1. **Fleuves de vallée dans `rivers.geojson`** : `cent-ans compose valley-rivers` (`compose/valley_rivers.py`)
   projette les tracés de `data/map/valley_courses.json` (Natural Earth 10 m pour le tracé dense du Nil et de
   l'Euphrate, deltas et amonts d'auteur) par le cartogramme, simplifie (efface le crénelage de l'ancienne
   grille), lisse (Chaikin), cale les bras du delta sur la mer, et les ajoute (`source: "valley"`, remplacés à chaque
   rejeu). Eau, lit creusé (`geo rivers-render`) et creusement du relief (`compose-relief-real`) les traitent comme
   les 45 autres. Largeurs d'affichage dans `river_styles.json`.
2. **Bande irriguée** : le colormap lit `valley_courses.json > irrigated_areas` (delta, Fayoum : distance nulle
   dedans) en plus de la distance aux fleuves ; palette `oasis` vert foncé de cultures, bande pleine ~ 4 km, fondue
   jusqu'à 11 km, peu de bruit.
3. **Bord de carte** : la terre descend au niveau de la mer sur `edge_haze_px` (vertex de `terrain.gdshader`) et se
   fond dans une brume de la couleur de l'horizon du ciel ; la mer (`water.gdshader`) fait de même et s'étend à
   `margin_factor = 4` cartes de chaque côté (jusqu'à l'horizon, plus d'anneau de plans gris) ; au-delà du bord la
   brume est pleine, opaque (cache l'abîme). La largeur `edge_haze_px = 12 + 0,14 x distance caméra` est posée par
   `CampaignWeatherView`. Aucune brume sur le parchemin.
4. **Rochers de désert** : `faithful: false` (gain de couleur du catalogue vers le sable) ; `min_pixels` 4 -> 7.
5. **Nuées** : `cumulus_cards.enabled: false` ; cirrus effacés avant 680 ; labours éclaircis dans `season_field`.

## Conséquences
- `geo compose` complet réécrit `rivers.geojson` sans les fleuves de vallée : rejouer `compose valley-rivers`
  juste après (puis `compose-relief-real`, `geo rivers-render`, `geo colormap`).
- Le Nil au sud de 28° N, l'Euphrate au-delà de 44° E sont hors du cartogramme (points retirés).
- `compose-relief-real` : si le cache d'auteur (`derived/author_height.npy`) est perdu, il est reconstruit avec les
  entrées complètes (pic ~ 7,8 Go une fois ; correction d'un cas 1 x 1).
- Le Tigre n'est pas tracé : le cartogramme écrase l'extrême est (tracé aberrant, lacs du masque) ; à reprendre si la carte s'étend à la Mésopotamie.
