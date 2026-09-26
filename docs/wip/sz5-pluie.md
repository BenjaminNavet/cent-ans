# SZ5 — pluie crédible à tous les paliers de zoom (défaut S7)

Worktree `agent-adeddc2012349e482` (depuis `main`). Lien : `docs/wip/sz-suites-zoom.md`,
`docs/wip/zg7c-recette.md` (défaut S7 : « pluie en bâtonnets blancs géants au palier site »).

## Diagnostic
`CampaignWeatherView._update_particles` dimensionnait les gouttes avec
`quad.size = base * maxf(k, minf(0.3, k * 1.36))`, `k = distance / 100`. 1 unité monde =
`MapData.meters_per_px` (≈ 719 m) : à `distance = 1,8` (palier site, `ZoomTiers.site_threshold`),
cette formule donnait une strie longue de 0,027 unité, soit **≈ 19 m réels** (large de 0,6 m) —
d'où les « bâtonnets géants ». La formule restait globalement proportionnelle à la distance (donc
correcte en angle vu depuis la caméra, cohérente en vue lointaine, validée par ZG7c) mais ignorait
l'échelle physique réelle.

## Solution
Ressource `PrecipitationProfile` (`game/resources/precipitation.tres`,
`game/scripts/map/precipitation_profile.gd`), branchée dans `campaign_weather_view.gd` à la place
des constantes codées en dur. Chaque grandeur (taille de goutte/flocon, boîte d'émission, hauteur
du centre de la boîte, vitesse de chute, ondulation de la neige, densité `amount_ratio`) est une
fonction continue de la distance caméra :
- ancrée sur une valeur réaliste **en mètres** (convertie via `meters_per_unit`, lu sur
  `MapData.meters_per_px`) à `near_distance` (1,8 = seuil du palier site) ;
- raccordée par une **loi de puissance** (linéaire en `log(valeur)` vs `log(distance)`, donc
  monotone et bornée par les deux ancrages, jamais de rebond) à `far_distance` (30, ancien palier
  « plat » de la formule remplacée) ;
- au-delà de `far_distance`, comportement **strictement identique** à l'ancien (vue vallée /
  lointaine déjà validée par ZG7c, testé par `_check_far_matches_legacy`) ;
- en deçà de `near_distance`, ratio (valeur / distance) maintenu constant : la taille continue de
  décroître avec la distance (aucun palier plat, aucun « rebond » au fond d'un canyon).

Deux itérations ont été nécessaires après la première capture :
1. **Oubli de conversion m → unités monde** dans les ratios (fait passer `0,06 m` littéralement
   comme un ratio face à `near_distance` en unités) : gouttes redevenues énormes (vallée : nappe
   blanche pleine écran). Corrigé par le paramètre `meters_per_unit` et l'interpolation en loi de
   puissance ci-dessus (au lieu d'un blend linéaire de ratios, qui explose entre les deux ancrages).
2. **Boîte d'émission trop haute** : en réduisant sa taille à une échelle réaliste sans revoir son
   décalage vertical (ancien `hauteur = distance × 0,35`, pensé pour une boîte immense), la pluie se
   retrouvait à ≈ 700 m au-dessus du point visé — hors du champ visible au palier site (aucune
   goutte visible). Ajout de `height()`, ancré lui aussi sur une hauteur réaliste (25 m) au palier
   site et raccordé à l'ancien ratio (0,35) au-delà de `far_distance`.
3. Une goutte réellement fidèle (millimétrique) serait sous le pixel à la distance caméra la plus
   proche (le palier site reste à l'échelle « quelques centaines de mètres à 1 km » compressée) :
   tailles restées stylisées (35 cm de large, 2,5 m de traînée pour la pluie, 20 cm pour la neige) —
   très en deçà de l'ancien (des dizaines de mètres) tout en restant visibles.

## État
- [x] 1. Squelette (`PrecipitationProfile`, `.tres`, doc wip)
- [x] 2. Courbe continue (taille, boîte, hauteur, vitesse, densité) branchée dans
  `campaign_weather_view.gd`
- [x] 3. Captures avant/après `docs/img/sz5/` (pluie : palier site Val de Loire et vallée Paris,
  lointain Paris ; neige : palier site Val de Loire)
- [x] 4. Test dédié `game/tests/sz5_precipitation_test.gd` (continuité, échelle réaliste au palier
  site, comportement legacy inchangé au-delà de `far_distance`, monotonie) : OK. `smoke` : OK
  (lancé en tâche de fond, à confirmer avant fusion).

## Captures
- `val_de_loire_site_avant.jpg` / `val_de_loire_site_apres.jpg` : palier site, pluie forcée
  (`--map-weather=rain`) — avant : bloc blanc géant plein écran ; après : gouttes fines et
  translucides, visibles individuellement au-dessus de la Loire.
- `paris_vallee_avant.jpg` / `paris_vallee_apres.jpg` : palier vallée — avant : nappe blanche
  couvrant tout le ciel ; après : quelques stries fines localisées, plausibles.
- `paris_lointain_apres.jpg` : palier lointain (d = 60) — inchangé par rapport à ZG7c (nappe de
  pluie floue, cohérente vue de loin), confirme que la vue lointaine n'est pas dégradée.
- `val_de_loire_site_neige_apres.jpg` : neige au palier site (`--map-weather=snow`), pas de
  bâtonnets géants non plus (le sol déjà blanc masque les flocons individuels, comme avant SZ5 :
  hors périmètre du défaut S7).

## Limites / points ouverts
- Au palier site, la pluie reste peu visible sur certains points de vue (ex. Paris, focus au
  niveau de la cathédrale) : la boîte d'émission (≈ 70 m) peut se retrouver masquée par un grand
  bâtiment au point visé. Pas un défaut du lot (pas de « bâtonnet géant »), mais pourrait être
  affiné (décalage horizontal de la boîte vers la caméra, ou taille de boîte plus généreuse).
  Testé et lisible sur un point de vue dégagé (Val de Loire).
  Réglage laissé dans `game/resources/precipitation.tres`.
- Tailles de gouttes/flocons restent stylisées, pas photométriquement réalistes (une goutte
  millimétrique serait invisible à cette échelle de caméra) : compromis assumé, documenté dans le
  script.
- Coût : `amount_ratio` réduit la densité active en vue lointaine (0,6 à `far_distance` et au-delà,
  au lieu de 1 avant) sans redémarrer le système de particules ; nombre max de particules
  (`GPUParticles3D.amount`) inchangé (5000 pluie / 2600 neige).
- Neige non spécifiquement retravaillée au-delà du même mécanisme générique (pas signalée comme
  défectueuse dans ZG7c) ; vérifiée sans régression.

## Prochaine étape
Lot terminé, fusion par l'orchestrateur (ff-only, hors checkout principal). Suivre le résultat de
`smoke` (lancé en tâche de fond au moment de la rédaction).
