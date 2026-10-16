# bm-campwater2 : eau douce, couloirs maritimes, reflets, lit creusé

## État (16/10)
- `water/fresh.rs` + `shaders/fresh.wesl` : salins, torrents (écume), gués, cascades (glb du manifeste DN), étangs/lagunes
  par blocs (niveaux patch/près autour du foyer de la caméra), fondu `fade_start/end`. Roselières comptées, non dessinées.
- `water/lanes.rs` + `shaders/lane.wesl` : couloirs maritimes à l'encre (3 encres x haute mer/cabotage, tronçons
  partagés dédupliqués, masqués au palier vallée), état du tour via `sim_campaign::sea_lanes::sea_lanes`.
- `shaders/sea.wesl` : paillettes du soleil (`water_glint`), constantes WGSL.
- `shaders/river.wesl` : lit creusé (eau à -0,022, ruban x1,35, bord non adouci) selon le même poids de distance
  que `river_carve` du terrain ; `minor_max_distance` (0,9 x étendue) masque les rivières mineures.
