# bm-campwater3 : gués, fine_zone, cohérence du creusement

## Fait (16/10)
- Gués/bacs dans `water/shaders/river.wesl` (`river_water.gdshader` : `fords[32]`, `ford_count`) : eau claire peu profonde
  (`deep` plafonné), remous et cailloux (`riffle` ajouté à l'écume), alpha x (1 - 0,35 ford) ; seulement de près
  (`fwidth(flow.x) < 0.5`), pas pour les lacs. Données : `materials::ford_uniforms` (= `river_crossings.gd`
  `ford_uniforms`, structure ford = 1, ferry = 0,25, rayon = largeur x 0,8 + 0,35), liés par `RiverParams::set_fords`
  à tous les rubans de rivière (19 gués/bacs au dump).
- `fine_zone` : uniforme de `RiverParams`, `fine_zone_mask` dans `river.wesl` (alpha x (1 - masque), comme
  `river_water.gdshader`), alimenté par `sync_fine_zone` (water/mod.rs) depuis `FineGeo.zone` (disque centré sur la
  visée, rayon = tuiles prêtes x poids du palier). Dump `campaign_water.river_fords` : zone [1651, 2731, 78, 1] avec
  la couche fine (`CENT_ANS_FINE_GEO_ROOT=~/dev/cent-ans-raw/compose`, caméra Paris d=60).
- Creusement : test `carve_tests::water_sink_matches_terrain_carve` (lit les constantes de `river.wesl` et
  `terrain.wesl`) : mêmes rayons 220/380, eau enfoncée de 0,022 < profondeur du lit 0,075 (eau sous la berge, au-dessus
  du fond : ni flottante ni enterrée). Les deux utilisent la distance caméra au point monde.

## Restes
- Le terrain fin (`river_banks` `fine_zone_mask`, `roads.set_fine_zone`) n'est pas concerné ici (lot terrain/roads).
- Pas de contrôle à l'œil (consigne : pas de capture).
