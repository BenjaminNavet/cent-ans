# bm-gaps10 : falaises de Douvres, eau du fond d'Avignon

## Douvres (campagne, `terrain.wesl` `coast_band`)
- Chaîne vérifiée : `coast_types.json` (région `chalk_south_england`, `cliff_probe_px` 5) -> sonde cuite
  (`coast::bake_probe` utilise bien le `cliff_probe_px` de la région) -> uniforms `CoastParams`
  (`coast_bound` true, `cliff_m` 36/52) -> géologie (R = craie) : rien ne manque, la craie est bien peinte.
  Test `dover_cliff_foot_reads_the_chalk_wall` : sur la bande 0-3 px de la côte autour de 1,31 E 51,12 N,
  plus de la moitié des texels lisent un mur >= 36 m (sonde 100-190 m au pied).
- Cause du « tout plage » : la bande de falaise de Godot ne fait que `cliff_px` x 0,75-1,15 x 2,6 (2-3 px
  carte) à partir de la côte raster, or à Douvres le mur est 3-4 px à l'intérieur de la côte raster et la
  pente (8 -> 160 m sur 1,3 -> 3,9 px) n'est quasi pas couverte : sur capture (analyse de pixels, sans
  lecture d'image) la bande côté terre était gris sombre, étroite, et le reste du versant restait herbe.
- Correction (écart à Godot, assumé) : `coast_band` reçoit la pente (`slope_g`) ; sur une falaise
  (`cliff_f`), la roche (craie) suit la pente raide (0,45-1,1 m/m) jusqu'à 1,2-2,6 x la largeur de bande
  de la côte ; portée de la bande 1,2 -> 2,2 x `cliff_px x widen`. Mesure : 5,9 % des pixels de la vue
  Douvres d=25 changent, la bande claire s'élargit vers l'intérieur.
- Reste : la craie est rendue en gris-clair sous la lumière basse (aube) ; à juger à l'oeil.

## Avignon (siège, `siege/backdrop.rs` + `backdrop.wesl`)
- Le « Rhône » est le maillage `Water` de `avignon_siege.glb` (matériau 0,045/0,09/0,10, rugosité 0,12),
  `water_meshes` 1 ; aucune rivière de champ de bataille (`battle_water.river` false). Il était rendu
  comme un plan lisse sans variation (réflexion d'environnement uniforme = rectangle gris).
- Correction : branche eau du shader du fond : houle procédurale (mêmes deux octaves que la rivière de
  bataille) sur la normale, fresnel de Schlick vers la couleur du ciel (`BackdropParams.sky` <-
  `water::sky_tint`, météo de la bataille), teinte profonde modulée par un bruit large. Les bords droits
  du maillage restent (Godot a le même rectangle) ; pas de fondu de rive : les sommets du `Water`
  n'ont pas de profondeur.
- Blocs clairs « dans le pré » : non retrouvés en 3D. L'analyse de pixels de la démo (avec et sans
  déploiement) ne montre dans le corps de la vue que 6 cadres crème réguliers (32 x 26 px, grille 3 x 2,
  x 411-745, y 195-305) : ce sont des éléments d'interface (cadres de cartes/boutons), pas des rochers.
  À confirmer par le joueur sur sa capture ; sinon piste : cadres de la HUD de bataille sans icône.
- Constat annexe : la démo d'Avignon prend l'horizon de `prov_suzdal` (`battle_decor.horizon.province`),
  panorama `picardy_plains` : la province de la carte de bataille de siège est mal résolue (hors lot).
