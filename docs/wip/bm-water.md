# bm-water : eau et routes de la carte Bevy (lot `client/src/campaign/water/`)

Etat (15/10) : porte et verifie par `--dump-state` (section `campaign_water`), aucun avertissement de shader.
- ribbon.rs (polyline_mesh.gd + rivers/roads ribbons), polygon.rs (triangulation, offset), data.rs (chargeurs),
  basins.rs (sea_basins.gd texture), labels.rs (river_labels.gd placement), materials.rs + shaders/*.wesl,
  mod.rs (plugin, sea, coast, lakes, rivers, roads, ponts-placeholders, dump).
- Crochets : `GroundHeight` (HeightSampler), `WaterView` (poids de paliers camera), message `RebuildWater`.
- Aide dev : `CENT_ANS_WATER_DEBUG_CAMERA=x,z,hauteur`.

Restes : voir le rapport du lot (freshwater_layer, sea_lane, lit creuse/gues, textes des noms, swash).

Lot drape : rubans (côte, rivières, routes, lacs) draperés sur GPU (`water/drape.rs`, altitude en mètres dans
`position.y`, lift dans les uniformes) ; noms de rivières rendus (`water/label_ui.rs`, police `UiTheme`, rotation
selon le cours). Les ponts restent posés au CPU (pas de suivi du zoom).

## Lot water2
Fait : mer vivante (`water/sea_life.rs`, `shaders/sea.wesl`, data/fx/sea_life.json : fonds clairs + caustiques,
crêtes du large, saturation/luminosité, liseré de ressac, estrans avec texture de zones) ; noms de rivières sans
collision (zone HUD, `RegionLabelObstacles`, noms de région, autres rivières ; dump `labels.suppressed`) ;
lisibilité des rubans à 260 u (`RiverParams::legibility`, dump `river_legibility`) : plancher écran 2,4-7 px
(majeures) / 0,9-2,6 px (mineures), mineures opaques à 260.
Restes : `water_detail.gd` (détails d'eau), `trade_route_layer.gd`, freshwater_layer, sea_lane, lit creusé/gués,
swash, profondeur de mer réelle (heightmap) au lieu de l'estimation par distance à la côte. Ponts : toujours posés
au CPU (lot countryside) ; leur hauteur doit suivre `ReliefGpu`/`GroundHeight` (relief dynamique au zoom).
