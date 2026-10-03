# ADR 0168 — Eaux de 1340 dans le masque terre : lacs historiques, retenues modernes

Date : 2026-10-03 (lot LR-10). Complète ADR 0036 (retenues de la pyramide), 0142 §4 (lacs SS3)
et 0161 (HC2).

## Contexte
`land_mask.png` est la terre de Natural Earth moins ses lacs (`geo build`). Il porte des règles
(grille de navigation, provinces) et l'habillage (arbres, rives, nappes de `lakes.json`). Deux
écarts à 1340 restaient : cinq retenues de barrage du XXe siècle (Sainte-Croix, Der-Chantecoq,
Orient, Ebro, Riaño) en eau, et des lacs de 1340 absents (Fucin, Copaïs, Amouq, Haarlemmermeer,
Whittlesey Mere asséchés depuis ; Windermere, Paladru, Joux… trop petits pour Natural Earth) ou
sans nappe (Grand-Lieu, Loch Ness, étang de Berre).

## Décision
- `data/map/historical_lakes.json` (schéma `historical_lakes.schema.json`) : une enveloppe
  lon/lat grossière tracée à la main par lac (polygone, ou axe + largeur), un seuil de relief
  `max_height_m`, un niveau de nappe `level_m`, des sources. Le contour vient du relief
  (`heightmap.png` ≤ seuil dans l'enveloppe, plus grande composante) : un fond de lac asséché
  est une plaine plus basse que l'ancien niveau. Pas de donnée OSM.
- `cent-ans geo land-mask` refait seulement le masque (identique à l'octet à `geo build` sans
  correction), sans les polygones Natural Earth qui portent un point de `modern_reservoirs.json`
  (et au plus 6 fois sa surface), avec les lacs historiques creusés. `geo build` applique les
  mêmes corrections.
- Province des pixels redevenus terre : celle du pixel étiqueté le plus proche. Les pixels devenus
  lac gardent leur province (le sélecteur reste juste, la grille de navigation tient les armées
  hors de l'eau). `coast_dist.png` et `province_border_dist.png` sont recalculés (même code que
  `geo splat`, reproduit à l'identique).
- `geo lakes` ajoute une nappe par lac historique à niveau (enveloppe ∩ eau du masque), sans test
  de planéité, niveau relevé au 3e quartile du relief rendu du bassin pour rester visible ; les
  lacs extraits centrés dans ce bassin sont remplacés. Champ `historical` dans `lakes.json`.
- Navigation régénérée (`geo navgrid`) : 286 cases deviennent infranchissables, 73 franchissables.

## Conséquences
- Règle de jeu : armées contournées par le Fucin, le Copaïs, l'Amouq, le Haarlemmermeer… et
  libres de traverser les vallées des cinq retenues.
- Petits lacs élargis à la maille de 719 m (`all_touched`) : 2 à 3 fois leur surface réelle,
  choix de lisibilité comme les rivières élargies (ADR 0158).
- Non régénérés : `colormap_bc1_*` (l'eau peinte sous les nappes manque pour les lacs
  historiques ; les retenues y sont déjà en terre), `splat.png`, biomes, occupation du sol,
  `provinces.geojson` (géométries inchangées, à 295 px près).
- Les grandes retenues soviétiques et turques marquées `Reservoir` par Natural Earth (Rybinsk,
  Kouïbychev, Keban…) restent en eau dans le masque : hors de cette décision.
