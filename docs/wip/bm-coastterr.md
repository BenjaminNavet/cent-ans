# BM coastterr — côtes, glaciers, contraste du terrain, météo par province (lot Bevy)

Branche `bm/coastterr` (worktree gp-bm-coastterr). Spécification : `docs/wip/bm-portaudit.md` V6, V7, ligne
« coastterr » ; branchement weather5 (`docs/wip/bm-weather.md` § weather5).

## État
- `client/src/campaign/terrain/coast.rs` : lecture de `coast_types.json` (régions, bande, falaise,
  ressac, couleurs), géologie cuite (Rgba8, 1 texel / 16 px), `coast_dist.png` (R8 pleine résolution),
  sonde de la règle de pente cuite sur le CPU (Rg8 demi-résolution : altitude au point de sonde,
  drapeau berge de lac HB7), paramètres glaciers (`map_freshwater.json` `glacier`). Section de vidage
  `terrain_coast`.
- `terrain.wesl` : `coast_band` (falaises, plages, ressac, normale penchée), `fw_glacier_apply`
  (névés, glaciers), `godot_adjustment` (contraste 1,06 / saturation 1,08 en fin de fragment),
  météo par province (`wx_field` / `wx_selection_clear` dans `weather_ground` / `weather_mist`,
  `wx_scatter` dans `camera_distance_fog`).
- `look.rs` : `sync_weather` (scalaire au point visé) retiré, remplacé par `sync_weather_ground`
  (lie `WeatherGround.field` / `.clear`, drapeaux, part matinale).

## Écarts à Godot
- La sonde de falaise (altitude à 1,6 px vers l'intérieur) est cuite une fois sur le CPU au lieu
  d'être lue dans le MNT par pixel : pas de MNT pleine résolution sur le GPU.
- Le contraste/saturation Godot s'applique après le tonemapper ; ici avant AgX, sur une copie gamma
  (le pipeline caméra n'est pas à ce lot).
- La diffusion solaire du brouillard reprend le terme exact des maillages PBR de Bevy (lumière
  `CampaignSun` × exposition × `DistanceFog.directional_light_color`), atténué par l'ombre portée.
- Pas de rugosité (le terrain Bevy est lambertien) : la rugosité de la plage mouillée est ignorée.

## Prochaine étape
- Vérifier par vidage (`CENT_ANS_WEATHER=fog`, côte normande / bretonne).

## Points ouverts
- Le lot water (finegeo) peut réutiliser `CoastTextures` (geology, dist) pour le ressac côté mer au
  lieu de recharger `coast_dist.png`.
