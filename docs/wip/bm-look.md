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

## À faire
Splat/biomes, champs peints, saisons, parchemin, géomorphing.
