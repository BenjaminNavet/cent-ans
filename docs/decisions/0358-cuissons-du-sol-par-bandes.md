# 0358 — Cuissons du sol par bandes (splat, occupation du sol, carte de couleur)

Statut : accepté (14/10, lot CC CS). Complète l'ADR 0355 (addendum CD) et l'ADR 0348 (outils).

## Contexte
Sur la carte composée (7080 x 6104 px), `geo splat` pointait à 14 Go et `geo colormap` à 12 Go de
mémoire résidente : une machine de 51 Go a planté quand une cuisson lourde a croisé d'autres
commandes. Les transformées de distance (EDT) de scipy coûtent à elles seules ≈ 2 Go de
temporaires sur la carte entière, les flous gaussiens et gradients autant, et les anciens codes
gardaient tous les champs de la carte en même temps.

## Décision
Toute étape qui évalue la carte entière travaille **par bandes de lignes avec halo**, et le résultat
ne dépend pas de la hauteur des bandes (mêmes octets) :
- `splat.gaussian_banded`, `splat.slope_banded` : halo = rayon du noyau, résultat identique ;
- `splat.edt_capped` : EDT par bandes **plafonnée** à `cap_px` ; chaque valeur ≤ cap est exacte
  (halo `cap + 2`), les plus grandes sont écrêtées ; les appelants donnent le plafond au-delà
  duquel leur consommateur sature (seuils de `smoothstep`, queues de `exp(-(d/k)²)`) ;
- `splat.coast_dist_banded`, `splat.border_dist_banded` : champs de distance à la côte et aux
  frontières (encodés 8 bits, plage ± 64 px et ± 21 px, donc halos 80 et 32) ;
- `compute_splat` : le bruit fractal (le seul champ global, tiré dans l'ordre des canaux) est tiré
  d'abord, tout le reste est calculé par bandes (halo 56 px) ; composition et encodage du splat par
  bandes ; `lon` / `lat` jamais pour la carte entière (`landcover.pixel_lonlat_rows`) ;
- `colormap.bake` : champs de la carte entière calculés une fois (`global_fields` : composantes
  d'eau, distances aux lacs et aux lieux, poids de biomes à grille grossière), puis une tuile par
  bande de texture (`tile_inputs`, halo 16 lignes de carte) ; le bruit de hachage reste évalué aux
  coordonnées de la carte entière (`ColormapInputs.row0`).
Tests : égalité avec la version entière sur grilles synthétiques (`test_splat`, `test_landcover`,
`test_colormap::test_tiled_bake_equals_single_band`) et sur recadrages de données réelles.

## Conséquences
- Pics mesurés (`/usr/bin/time -l`) : voir `docs/wip/cc-cs-sol.md` § Mémoire.
- Un nouveau champ à grande portée (flou > 56 px par exemple) doit élargir `SPLAT_HALO`.
- Les distances plafonnées (`DISTANCE_CAP_PX`, `LAKE_CAP_PX`, `SETTLEMENT_CAP_PX`, `RIVER_CAP_PX`)
  ne sont pas des distances vraies au-delà de leur plafond : ne pas les réutiliser telles quelles
  pour autre chose.

## Sol de près (même lot)
- Carte de couleur : les cultures sont rapprochées de la moyenne de leur liste (`crop_contrast`),
  suivent des soles de 14 km (`sole_km`, `sole_share`) et portent un grain de luminosité
  (`grain`) : fin du confetti or / brun / vert de blocs tirés au hasard.
- Prairie sèche : `landcover_params.json` `grass_to_heath` convertit une part de la prairie du splat
  en lande / garrigue selon le biome (méditerranéen, semi-aride, steppe) : fin des taches de gazon
  vert vif sur un sud dont la carte de couleur est paille.
- Vallées sèches : `valley_green` (palette `oasis`) peint une bande verte le long des rivières
  permanentes des biomes secs (Nil).
