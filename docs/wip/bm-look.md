# bm-look — aspect de la carte de campagne (Bevy)

Branche `bm/look`. Dossier : `client/src/campaign/terrain/`.

## Fait
- `relief.wesl` : module WESL partagé (port de `campaign_relief.gdshaderinc`), validé par un test naga.
  Patron de liaison documenté dans l'en-tête du fichier et dans `relief_gpu.rs`
  (`ReliefParams` + texture `ReliefGpu::floor`).
- `cpu_sampler.rs` : alimente `water::height::GroundHeight` et `provinces::SurfaceHeight`
  depuis `Surface` + `ReliefState` (instantané rafraîchi en PreUpdate, `RebuildWater` à la 1re
  disponibilité).
- Colormap uni olive : course entre `poll_colormap` et `ensure_lod` (le matériau n'était relié à
  la colormap que si `TerrainLod` existait déjà) ; liaison par `TerrainMaterialHandle`.

- `season.rs` + `terrain.wesl` : saisons (poids lissés lus dans `Sim`, crochet `CENT_ANS_SEASON`,
  étalonnage/or d'été/sol d'hiver, lignes de neige par altitude, neige de plaine au nord).

## À faire (Godot : terrain.gdshader, campaign_textures.gd, biome_parents.gd, cafv_tile.gd, parchment_overlay.gd)
Splat/biomes (tableaux de textures), champs peints, vue parchemin, géomorphing, brouillard de guerre,
neige selon biome/pente fine, essences saisonnières.

## look2 (branche bm/look2)
Fait : `look.rs` (voile de brouillard de guerre depuis `FogOfWar::mask` en texture R8, réglages `campaign_map.json`
`fog_of_war`), fondu parchemin simplifié selon `ZoomView::strategic` (papier, hachures, forêts, mer, massifs),
sol mouillé / neige fraîche selon la météo sous le foyer (lissés), `DistanceFog` de la caméra honoré par le shader
(`camera_distance_fog`), géomorphing des LOD (attribut COLOR = delta parent, distance de bascule dérivée de
`max_vertex_px`), neige de plaine `snow.plain_share` 0,55 + `snow.plain_color` plus clair dans `data/ui/campaign_seasons.json`.
Preuve : dump `campaign_terrain.material` (fog_of_war.bound, fresh_snow, parchment_weight, geomorph).
Reste : splat/biomes 14 TX (tableaux `tx_campaign_bg_*`, `biomes_base.png`, `bg_ab/bg_dist`), grain `tx_micro_ground`,
champs peints (parcels, `field_layer`), `fine_geo`, parchemin complet (lavis de faction, montagnes/arbres dessinés, frontières),
météo par province (aujourd'hui globale au foyer), vérification visuelle.
