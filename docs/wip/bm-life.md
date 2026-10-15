# bm-life : vie de la carte (client Bevy)

Dossier : `client/src/campaign/life/`. Branche `bm/life`.

## État
- Données lues dans `data/` (`data.rs`) : `town_maquettes.json` props, `map_scenes.json`, `map_birds.json`,
  `map_fauna.json`, `map_fire_wind.json`, `dn_water_models.json`, routes/fleuves/bassins via `WaterData`,
  `sea_lanes_px.json`, `hamlets.json`, `wetland_sites_px.json`, `settlements_px.json`, `land_mask.png`.
- Flux commun (`stream.rs`) : vue (foyer, rayon, seuil d'éclaircie ADR 0354), index des instances vivantes,
  cibles de fondu (bord, portée, éclaircie par aléa fixe), `Grounded` (hauteur via `GroundHeight`).
- `models.rs` : glb en `WorldAssetRoot`, taille monde fixe mesurée une fois (Aabb), fondu à l'apparition et à la
  disparition (aucun pop).
- `folk.rs` : villageois sur les tronçons de route, charrettes et convois attelés (chevaux / bœuf), charretier,
  marchand, garde. `fauna.rs` : troupeaux par cellule et zone. `boats.rs` : bateaux de fleuve, navires des routes
  maritimes. `birds.rs` : orbite, transit en V, échassiers, cigognes. `effects.rs` : panaches, moulins (ailes qui
  tournent), incendies (cônes + lumières) selon dévastation et sièges, vent de `map_atmosphere.json`.

## Preuve
`--dump-state` (section `campaign_life`, 3 vues) : Paris d=40 : 6 villageois, 3 charrettes, 1 moulin + ailes, 1 panache, 1 bateau ;
Normandie d=60 : 413 bêtes, 19 villageois, 76 oiseaux (5 vols) ; Paris d=160 : 3 moulins, 2 bateaux de fleuve.

## Reste à faire
Voir le rapport final (écarts : figurines skinnées, chemins de commerce du pont, cheminées 1:1, scènes de province FK4,
incidents FK5, bannières de maquette, biomes/landcover pour oiseaux et faune, moulins à eau).
