# bm-weather (lot météo Bevy)

État : nuages + ombres (sprites), pluie/neige (pool d'entités), brume `DistanceFog`, ressource `FogOfWar`, bandeau de saison : portés dans `client/src/campaign/weather/`.
Reste : voir le rapport (masque météo / sol mouillé / brume de vallée = shader de terrain, lot look ; `map_atmosphere.gd` cirrus / rideaux de pluie / aurore ; éclairs d'orage ; fondu du bandeau).

## weather2 (vague 4)
Porté : `sky.rs` (cirrus, rideaux de pluie, brouillard de fleuve, éclairs d'orage ; réglages `data/fx/map_atmosphere.json`), ombres de cumulus par beau temps (`clouds.rs`, `fair_alpha`), fondu des nuées en parchemin (`ZoomView::strategic`), fondu du bandeau (alpha par couleur de nœud).
Reste : aurore boréale (`map_atmosphere.gd`), champ de cumulus du ciel, masque météo / sol mouillé (shader terrain, lot look), vérification visuelle.
