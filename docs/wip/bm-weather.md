# bm-weather (lot météo Bevy)

État : nuages + ombres (sprites), pluie/neige (pool d'entités), brume `DistanceFog`, ressource `FogOfWar`, bandeau de saison : portés dans `client/src/campaign/weather/`.
Reste : voir le rapport (masque météo / sol mouillé / brume de vallée = shader de terrain, lot look ; `map_atmosphere.gd` cirrus / rideaux de pluie / aurore ; éclairs d'orage ; fondu du bandeau).

## weather2 (vague 4)
Porté : `sky.rs` (cirrus, rideaux de pluie, brouillard de fleuve, éclairs d'orage ; réglages `data/fx/map_atmosphere.json`), ombres de cumulus par beau temps (`clouds.rs`, `fair_alpha`), fondu des nuées en parchemin (`ZoomView::strategic`), fondu du bandeau (alpha par couleur de nœud).
Reste : aurore boréale (`map_atmosphere.gd`), champ de cumulus du ciel, masque météo / sol mouillé (shader terrain, lot look), vérification visuelle.

## weather3 (vague 5)
Porté : `aurora.rs` (aurore boréale, hiver/soir, `SkyDusk` à alimenter par le lot look), `cumulus.rs` (champ de cumulus du ciel, 3 plans, textures cuites), `focus_clear` (rideaux de pluie et brouillards de fleuve, 8 niveaux de matériaux partagés), texture de pluie fidèle au shader (stries + voile, enveloppe verticale/latérale en couleurs de sommets), rivières lues directement dans `rivers_render.json` (plus de double `WaterData::load`).
Les familles désactivées dans `data/fx/map_atmosphere.json` (aurore, cumulus) s'allument avec `CENT_ANS_ATMOSPHERE_ALL=1` pour les preuves.
Reste : masque météo / sol mouillé (shader terrain), turn_light (dusk), pénalité mer et `wet` des cumulus, focus_clear sur cirrus/cumulus/aurore (plan unique), vérification visuelle.

## weather4 (vague 5) : brume de campagne fidèle à Godot
Cause du voile : `atmosphere.rs` posait un `DistanceFog` exponentiel (densité 0,5/distance, alpha jusqu'à 0,8 par météo) EN PLUS du brouillard de profondeur propre au shader de terrain (`params.fog`, début 800), soit deux couches ; or Godot n'a aucune brume de distance météo (seulement le voile de sol plafonné `weather_mist_max` 0,07, ADR 0142, dans le shader terrain).
Fait : une seule couche = brouillard de profondeur de l'Environment Godot (`campaign_map.tscn`, `campaign_atmosphere.gd::apply_distance`, `campaign_lighting.gd::aerial`, `data/fx/campaign_lighting.json` bloc `aerial`, `fog_color` de saison de `atmosphere.json` mêlé au bleu aérien à 50 %). Formule Godot : `pow(smoothstep(début, fin, d), 1,6) * densité`, début/fin = distance caméra × facteurs (paires selon l'inclinaison 32°-60°), ramenés vers 18/240 en vue rapprochée. Bevy n'ayant que des rampes linéaires, une droite est ajustée sur la courbe (`DepthFog::fit_linear`, points g = 0,2 et 0,8). Le fog propre du terrain est coupé (`params.fog.w = 0`) ; le shader terrain applique le `DistanceFog` caméra (`camera_distance_fog`). La météo (pluie, neige, orage, brume du matin) ne touche plus la brume de distance.
Écarts : pas de perspective aérienne ni de diffusion solaire (`fog_sun_scatter`) ; courbe linéaire ajustée (écart < 0,02 de quantité). Le voile de sol plafonné à 0,07 reste à vérifier côté `terrain.wesl` (lot look).

Valeurs effectives (dump `weather.atmosphere.table`, G = formule Godot, B = rampe Bevy ; u = position entre début et fin) :

| D caméra | inclin. | début | fin | densité | Bevy début/fin | foyer G/B | u50 G/B | u75 G/B | fin G/B |
|---|---|---|---|---|---|---|---|---|---|
| 2600 | 70,0 | 4940 | 20800 | 0,50 | 9508/19142 | 0,000/0,000 | 0,165/0,174 | 0,381/0,380 | 0,500/0,500 |
| 1200 | 48,3 | 1984 | 8463 | 0,63 | 3850/7786 | 0,000/0,000 | 0,209/0,221 | 0,482/0,481 | 0,633/0,633 |
| 600 | 39,0 | 810 | 3532 | 0,80 | 1594/3248 | 0,000/0,000 | 0,263/0,278 | 0,606/0,605 | 0,796/0,796 |
| 200 | 32,8 | 250 | 1101 | 0,85 | 495/1012 | 0,000/0,000 | 0,280/0,296 | 0,647/0,646 | 0,849/0,849 |
| 60 | 30,6 | 75 | 330 | 0,85 | 148/303 | 0,000/0,000 | 0,280/0,297 | 0,648/0,646 | 0,850/0,850 |

## weather5 (vague 8) : perspective aérienne, diffusion solaire, météo par province
Fait (`atmosphere.rs`) : `aerial_perspective` (données `campaign_lighting.json`, paire [vue basse, dessus], lissée par l'inclinaison) = part de la teinte du ciel (`ambient.color`) dans la couleur de brume, pondérée par la direction du regard (`aerial_weight` : pleine vers l'horizon, nulle vers le bas ; rayon central pour la caméra Bevy) ; `fog_sun_scatter` de saison (`atmosphere.json`) = `DistanceFog.directional_light_color` = couleur du soleil × valeur, lobe `scatter_exponent` 8. Les meshes PBR (soleil `CampaignSun`) l'appliquent déjà ; dump `weather.atmosphere.{aerial_weight,sun_scatter}`.
Fait (`ground.rs` + `ground.wesl`) : météo PAR PROVINCE comme Godot. `WeatherGround.field` (RGBA8, carte/8 : R pluie, G neige, B brouillard, A orage, frange bruitée de 3 lectures = `wx_sample`) rebâti à chaque changement de météo ; `WeatherGround.clear` (R8, carte/4 : intérieur de la province sélectionnée, bord fondu `clear_feather_px`) rebâti à chaque changement de sélection. Dump section `weather_ground`. Module WESL `cent_ans_client::campaign::weather::ground` : `wx_field`, `wx_selection_clear`, `wx_scatter`, `wx_aerial_weight`, `wx_aerial_fog_color`.
À BRANCHER (terrain.wesl / material.rs, hors de mon dossier, lot showfix/look) :
1. `TerrainMaterial` : deux textures (field, clear) + sampler linéaire, copiées chaque image depuis `Res<WeatherGround>` (`.field.clone()`, `.clear.clone()`, `.any`) ; `map_size` existe déjà (`params.map.xy`).
2. `terrain.wesl` : `import cent_ans_client::campaign::weather::ground::{wx_field, wx_selection_clear, wx_scatter, wx_aerial_fog_color};` ; dans `weather_mist`/`weather_ground` remplacer `look.mist.x` (poids global au point visé), `look.weather.x/y` (humide/neige) par `let w = wx_field(...)`, humide = max(w.r, w.a), neige = w.g, brume = w.b, puis `mist *= wx_selection_clear(...)` (comme `clear` Godot) ; dans `camera_distance_fog`, ajouter `wx_scatter(normalize(world - view.world_position), params.sun_dir.xyz, params.sun_color.rgb, <scatter>, 8.0)` à la couleur de brume (le scatter saisonnier est dans `Atmosphere::scatter`, à passer en uniforme) et `wx_aerial_fog_color` par pixel si on veut la direction exacte.
3. Retirer alors `terrain/look.rs::sync_weather` (scalaire au point visé).
Écarts : la caméra Bevy applique l'aérien au rayon central (une couleur par caméra) ; sans l'étape 2 la brume du sol reste au point visé et la province sélectionnée n'est pas dégagée de la brume du terrain (les nuées, elles, la dégagent déjà).
