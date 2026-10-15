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

## look3 (branche bm/look3)
Fait (Godot -> Rust) :
- Sol régional TX : `ground.rs` (campaign_textures.gd, biome_parents.gd) : tableau 45 couches `tx_campaign_bg_*` (feuille JPEG découpée 5x9,
  couches ramenées à 512 px, mips CPU, normales en Rg8), table biome x rôle (parents), teintes relatives, fondu de biomes
  (`biomes_blend_ab/dist` fusionnés en une texture RGBA8 avec le masque des paysages agricoles et la distance au lit des rivières),
  splat de matières (`splat.png` demi-résolution), grain `tx_micro_ground_*`. `terrain.wesl::ground_proc` (sample_layer / sample_role,
  niveaux d'échelle `tile_screen_px`, normales de détail) puis fondu avec la carte de couleur façon `sg_apply` (0,45 de près, 1 de loin).
- Champs peints : `fields.rs` (hb_ground.gd, ADR 0290 / 0352) : table cultures par biome et paysage agricole (rang = ordre du fichier,
  `landscape_order`), blocs `view` et `geo` de `ground_biome_mix.json` ; `terrain.wesl::fields_apply` : parcelles de Voronoi par région
  (orientation, taille par biome), culture tirée par poids (70 % dominante de la région), relief (altitude, pente), vignes de coteau,
  prés au bord des fleuves, haies, patchwork lointain par soles puis terroirs.
- Parchemin complet : `parchment.rs` + `terrain.wesl::parchment_ground` : lavis de faction (couleur du propriétaire par province, pigment
  accumulé au bord), montagnes dessinées (altitudes en blocs de 16 px, R32Float), arbres dessinés d'après le splat.
- Crochets de développement : `CENT_ANS_NO_FOG=1` (lève le voile), `CENT_ANS_NO_GROUND=1` (carte de couleur seule, comparaison A/B).
Preuve : dump `campaign_terrain.material.regional_ground` / `parchment`, tests `ground`, `fields`, `parchment` ; capture de la vue parchemin.
Restes : (1) PRÉOCCUPATION : en capture hors écran, de près (distance <= 400) le terrain n'apparaît presque pas (sonde rouge en tête de
fragment : 0 % de pixels) alors qu'il se voit de loin ; un plan sombre le recouvre (eau / abysse ? relief dynamique ?), identique avec et sans
`CENT_ANS_NO_GROUND` : hors dossier terrain, à élucider avant de juger le rendu proche. (2) pas de neige / sable texturés (rôles 5, 6), pas de
parcelles `tx_campaign_parcels` (champs peints en aplats de culture + grain du sol), pas de défrichements de terroir (`terroir_mask`), pas
de saison par culture (`season_field`), pas de variation macro GA4. (3) frontières du parchemin = tracés drapés du module provinces (pas
repris dans le shader). (4) fine_geo (fine_geo_layer/store, fine_bed_carver, fine_ribbon_job) et éclairage de campagne
(campaign_lighting.gd) non faits. (5) couches 512 px au lieu de 1024 (mémoire) ; splat en demi-résolution.
