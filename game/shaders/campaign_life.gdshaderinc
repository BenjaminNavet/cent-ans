// Lot CV1 : campagne vivante, fonctions incluses par terrain.gdshader (crochets localisés).
// - Saisons : poids globaux `campaign_season` (printemps, été, automne, hiver ; somme 1),
//   publiés par season_visuals.gd, glissant d'un tour à l'autre.
// - Terroirs : masque terroir_mask.gd (R cultures, G vigne, B brûlis, A pâtures).
// - Ombres de nuages qui passent.

global uniform vec4 campaign_season;
uniform sampler2D terroir_mask : filter_linear, repeat_disable, hint_default_black;
uniform bool has_terroir = false;
uniform bool life_enabled = true;
