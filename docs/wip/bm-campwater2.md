# bm-campwater2 : eau douce, couloirs maritimes, reflets, lit creusé

## État (16/10)
- `water/fresh.rs` + `shaders/fresh.wesl` : salins, torrents (écume), gués, cascades (glb du manifeste DN), étangs/lagunes
  par blocs (niveaux patch/près autour du foyer de la caméra), fondu `fade_start/end`. Roselières comptées, non dessinées.
- `water/lanes.rs` + `shaders/lane.wesl` : couloirs maritimes à l'encre (3 encres x haute mer/cabotage, tronçons
  partagés dédupliqués, masqués au palier vallée), état du tour via `sim_campaign::sea_lanes::sea_lanes`.
- `shaders/sea.wesl` : paillettes du soleil (`water_glint`), constantes WGSL.
- `shaders/river.wesl` : lit creusé (eau à -0,022, ruban x1,35, bord non adouci) selon le même poids de distance
  que `river_carve` du terrain ; `minor_max_distance` (0,9 x étendue) masque les rivières mineures.

## Vérification (16/10)
- fmt, `clippy --features dev --all-targets -D warnings`, `cargo test --features dev` : 1151 + annexes verts (9 tests neufs).
- Dumps `--state Campaign` (debug, hors écran, muet), caméra par `out/camera.txt` : aucune erreur de shader ni panique.
  Manche (xz 1100,2150) d=60 et d=600, Paris (1651,2731) d=60 et d=600, Dombes (2186,3666) d=40.
  `campaign_sea_lanes` : 25 couloirs, 29 tronçons après déduplication, 3 425 échantillons, 10 cabotage + 15 haute mer
  (encre libre, 0 hostile au départ). `campaign_freshwater` : 1 712 cases de salins, 88 torrents (898 sommets),
  16 cascades et 3 gués en glb ; étangs : Dombes d=40 = 82 blocs, 263 mares, 941 touffes comptées ; d=600 : patch_alpha 0
  (cf. `patch_max_distance` 120), statiques visibles (< 1 200).
- Non vérifié à l'œil (pas de capture) : eau enfoncée de 0,022 dans le lit creusé (si le terrain ne creusait pas,
  les rivières proches disparaîtraient : `WATER_SINK` / `CARVE_NEAR` en tête de `river.wesl`), paillettes, encre.

## Écarts à Godot
- Roselières (`freshwater_reeds.gdshader`, cartes) comptées mais non dessinées (lot végétation) ; pas de cercles
  d'exclusion autour des villes 1:1 ; pas de colonne d'eau de remplacement des cascades (les glb existent).
- Tirage pseudo-aléatoire propre (SplitMix64) : les mares ne tombent pas aux mêmes endroits que dans Godot.
- Pas de texture de profondeur pour l'eau du lit creusé (teinte profonde selon la largeur) ; creusement par distance
  caméra (comme le terrain Bevy) au lieu des tuiles `carved_rects`.
- Paillettes : constantes WGSL, sans ombre de cumulus (`v_cloud_sun`) ni effacement par le parchemin.

## Restes
- `freshwater_ground.gdshaderinc` / névés et glaciers (`_apply_glacier`) : lot coastterr.
- Gués dans le shader de rivière (`fords[32]`), `fine_zone` (ancien ruban fondu dans le disque fin).
- Infobulle des couloirs (`nearest_lane`) à brancher dans le HUD / picking.
