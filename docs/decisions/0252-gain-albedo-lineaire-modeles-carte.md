# 0252 — Gain d'albédo linéaire des modèles générés de la carte

## Contexte
Revue RX (campagne, v1 et v2) : autour de Paris de près, des dizaines de dalles crème froissées sur le sol,
et des maquettes de villes/châteaux en blocs blancs surexposés. Cause racine : `DnCampaignModels.brighten`
posait `albedo_color = Color(gain, gain, gain)` (gain 2,0 pour les champs, 2,4 pour les lieux). Godot lit
`albedo_color` comme du sRGB et le convertit en linéaire : un « gain 2 » multipliait réellement l'albédo
par ~5 (et ~7 pour 2,4). Les parcelles de blé (`field_*`, albédo linéaire ~0,12) et les lieux passaient en blanc.
Aucune texture n'était manquante : les glb et leurs jpg sont présents et chargés (vérifié).

## Décision
- Le gain est un facteur linéaire : `DnCampaignModels.gain_color(gain)` = `Color(g,g,g).linear_to_srgb()`.
- Gain des champs relevé de 2,0 à 2,6 (linéaire) pour rester dorés ; gain des lieux inchangé (2,4).
- Test `tests/rx_mapa_textures_test.gd` : toute surface des modèles référencés par `dn_fields.json` et
  `dn_campaign_models.json` a une texture d'albédo non nulle, le gain est linéaire, et l'albédo moyen
  après gain reste sous 0,55 (pas de blanc surexposé).

## Conséquences
Champs et maquettes générés plus sombres et colorés qu'avant (les teintes baked réapparaissent). Les
gains des données se règlent désormais comme des facteurs linéaires. Les maquettes du kit (`maquette_kit`,
couleurs de sommet) ne sont pas concernées.
