# ME5 — atmosphère de la carte (branche dn/me5-ciel)

État : implémenté, données + schéma + tests ; reste le jugement visuel de l'orchestrateur.

- Code : `game/scripts/map/map_atmosphere.gd` (créé par `CampaignWeatherView.setup`, `refresh`, `update_view`),
  shaders `game/shaders/map_atmo_*.gdshader` + `map_atmosphere_common.gdshaderinc`.
- Données : `data/fx/map_atmosphere.json` (schéma `fx_map_atmosphere.schema.json`, test pytest). `--no-me5` éteint.
- Familles : cartes de cumulus éclairées (soleil de la carte), cirrus (plan à 96), rideaux de pluie
  (provinces pluie/orage, MultiMesh), bancs de brouillard sur les fleuves (levés avec la brume TB6),
  aurore au bord nord (hiver, plancher de jour, pleine au soir doré). Fumées de villes/feux : déjà `LifeEffects`.
- Tests : `game/tests/me5_atmosphere_test.gd` ; captures : `game/tests/me5_shot.gd` (via tools/godot_bg.sh).
- Prochaine étape : réglage visuel (taille/opacité des cartes, cirrus peu visible, aurore non vérifiée à l'œil),
  alignement des cartes sur les ombres de cumulus RV-C (non fait : positions indépendantes), banc de frame.
