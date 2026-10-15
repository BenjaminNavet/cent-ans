# bm-water : eau et routes de la carte Bevy (lot `client/src/campaign/water/`)

Etat (15/10) : porte et verifie par `--dump-state` (section `campaign_water`), aucun avertissement de shader.
- ribbon.rs (polyline_mesh.gd + rivers/roads ribbons), polygon.rs (triangulation, offset), data.rs (chargeurs),
  basins.rs (sea_basins.gd texture), labels.rs (river_labels.gd placement), materials.rs + shaders/*.wesl,
  mod.rs (plugin, sea, coast, lakes, rivers, roads, ponts-placeholders, dump).
- Crochets : `GroundHeight` (HeightSampler), `WaterView` (poids de paliers camera), message `RebuildWater`.
- Aide dev : `CENT_ANS_WATER_DEBUG_CAMERA=x,z,hauteur`.

Restes : voir le rapport du lot (freshwater_layer, sea_lane, lit creuse/gues, textes des noms, swash).
