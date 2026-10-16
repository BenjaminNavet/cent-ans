# bm-finegeo : fleuves fins, routes drapées, ancrages fins (lot `finegeo`)

## État (16/10)
- Port : `client/src/campaign/water/fine/` (`cafv.rs` = cafv_tile.gd, `store.rs` = fine_geo_store.gd,
  `job.rs` = fine_ribbon_job.gd, `mod.rs` = fine_geo_layer.gd, `material.rs` + `shaders/river_fine.wesl`,
  `shaders/road_fine.wesl` = river_fine/road_fine.gdshader) ; ponts en mode fin dans
  `countryside/bridges.rs` (`FineBridgeMode`, `apply_fine_bridges`, `fine_transform`).
- Lecture des tuiles sur `IoTaskPool`, maillage sur `AsyncComputeTaskPool` (comme les fils de Godot),
  2 installations par image, cache LRU 64 tuiles / 40 tuiles maillées, remaillage quand l'étage de page
  le plus fin sous la tuile change (jusqu'à E4), après 0,6 s de calme.
- Drapage : hauteurs en mètres, module `relief.wesl` via `drape::with_relief`, `sync_drape` (correctif
  showfix) pour les deux matériaux.
- Dump `campaign_fine_geo` (activé, raison, tuiles indexées/chargées/maillées, zone, poids, ponts fins).

## Constat important : données fines absentes de la carte composée
- `data/map/pyramid` → `~/dev/cent-ans-raw/compose/pyramid` : pas de `hydro_fine` ni `roads_fine`.
  Godot désactive donc `FineGeoLayer` sur la carte actuelle (`store.available` faux) ; Bevy aussi
  (dump : `enabled: false`, raison). Les tuiles existent dans `cent-ans-raw/pyramid-reelle/` mais dans
  le cadre de la carte RÉELLE : `rivers_fine.json` / `fine_anchors.json` datent du 02/10, avant la carte
  composée (Paris : 2213,3204 dans les ancrages contre 1651,2731 dans `settlements_px.json` ; écart
  médian des 400 colonies 637 px). Il faut regénérer `geo hydro-fine` (et les ancrages) dans le cadre
  composé (outil Python, hors de ce lot) pour que la couche serve en jeu.
- Crochet de preuve : `CENT_ANS_FINE_GEO_ROOT=<dossier contenant pyramid/hydro_fine>` (lien vers la
  pyramide réelle) ; les rubans sont alors posés dans l'ancien cadre (désalignés), preuve de chaîne
  seulement.

## Écarts à Godot
- Pas de texture de profondeur (lit non creusé en Bevy) : teinte profonde selon la largeur (`deep_width`).
- Zones personnalisées : toutes « fermées » (coupent l'eau) ; pas de villes 1:1 (zones ouvertes VH).
- Ponts-portes et ponts de zone non portés : `covers` est vide dans Godot depuis VT (ADR 0138) et toutes
  les zones ont `boundary_bridges: false` → aucun n'existe dans le jeu actuel.
- Ponts fins : bascule par id d'ancrage (Godot : même index dans `crossings_px.json` + id), 12 par image.

## Restes
- `fine_bed_carver.gd` (lit creusé dans les pages du relief) : lot terrain.
- Fondu des anciens rubans (rivières, routes, terrain) dans le disque fin (`old_materials`,
  `roads.set_fine_zone`, `fine_zone` du terrain) : à faire dans river.wesl/road.wesl quand les données
  fines du cadre composé existeront (sinon double tracé).
- Ancrages des colonies et hameaux (`SettlementLayer.apply_fine_anchors`) : lot marqueurs ; `FineGeo::store()`
  les expose (`settlements`, `hamlets`).
- Préréglage de qualité : niveau lu dans `RenderQuality.level` (Godot : budget `relief_items`).
