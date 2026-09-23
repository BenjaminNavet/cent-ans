# WIP — M3 Godot (villes et économie)

## État
- Session démarrée. Lecture des specs (`docs/design/m3-cities-economy.md` §2-3), du code
  existant (`sim_facade.gd`, `campaign_sim_mock.gd`, `province_panel.gd`, `map_ui.gd`,
  `campaign_map.gd`, `terrain_builder.gd`, `city_markers.gd`, `smoke.gd`) et des données
  (`data/buildings/*.json`, `data/resources/*.json`, `data/provinces/prov_ile_de_france.json`).
- `core/` : aucun agent n'a encore poussé `get_province_city` / `get_faction_economy` /
  ordres `build`/`cancel_build`/`set_tax_rate` (vérifié via `git log -- core/crates/godot-bridge`).
  Tout se construit contre le mock pour l'instant ; `SimFacade` doit garder les `has_method`
  guards pour basculer dès que la vraie API arrive.
- Plan :
  1. Étendre `campaign_sim_mock.gd` : classes de population par province, bâtiments/construction,
     buildable, ressources/biens, tax_rate, `get_province_city`, `get_faction_economy`, ordres
     `build`/`cancel_build`/`set_tax_rate`, événements colorés (`revolt`, `plague`, `famine`,
     `building_completed`).
  2. `SimFacade` : `has_method` guards autour des nouveaux appels.
  3. UI : onglets du panneau de province (Garnison / Ville), panneau de faction, barre
     supérieure revenu prévisionnel, mode carte mécontentement (touche M) + marqueurs marteau.
  4. Smoke test : scénario construction + revenu.
  5. Captures d'écran `--stage=city` / `--stage=faction`.
  6. Mise à jour `docs/godot-map.md`, suppression de ce fichier en fin de tâche.

## Prochaine étape
Étendre `campaign_sim_mock.gd` avec les données de ville par province (classes, bâtiments,
construction, buildable, ressources) et l'économie de faction (trésor, revenus, impôts).
