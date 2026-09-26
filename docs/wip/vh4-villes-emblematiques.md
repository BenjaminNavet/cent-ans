# VH0 + VH4 — villes emblématiques 1:1 géoréférencées (ADR 0078)

Branche `feat/vh4-landmarks-1to1` (worktree d'agent, depuis `main` 7e1ac032). Orchestration SZ
(`docs/wip/sz-suites-zoom.md`), défaut S3 de ZG7c. Liens symboliques non versionnés
`data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis
copie dans `game/bin/libcent_ans.debug.dylib`. Aucun changement Rust.

## Architecture
- Données : `data/landmarks_v2/<id>.json`, schéma `data/schemas/landmark_v2.schema.json`
  (EPSG:3035 : `origin_3035` + décalages [dE, dN] en m, axes de la grille).
- Outil : `cent-ans geo landmarks [--city rouen] [--refresh-osm]`
  (`tools/cent_ans_tools/geo/landmarks_v2.py`) : rues héritées tirées d'OSM (recette
  `osm_streets`, cache `tools/geo/raw/osm/<id>_highways.json`), fleuve recopié de la carte fine
  (`hydro_fine`, section `waters` origine `rivers_fine`).
- Godot : `LandmarkV2Data` (lecture, conversion en unités carte), `LandmarkPlan` (plan pur, fil de
  travail, sortie au format `TownPlan`), `LandmarkMonuments` (gabarits réels), `LandmarkCityLayer`
  (streaming, construction par `TownBuilder`, fondu de la maquette), accroche dans
  `SettlementLayer`, plancher caméra levé pour les villes v2.

## État
- [x] ADR 0078 (commit ac568e3b)
- [x] Schéma v2, outil `geo landmarks` (squelette fonctionnel)
- [ ] Données Rouen vers 1340
- [ ] Stubs Godot + tests désactivés
- [ ] Plan, monuments, couche, fondu, plancher caméra
- [ ] Captures `docs/img/vh4/`, mesure i/s
- [ ] Docs `docs/landmarks-v2.md`, `docs/godot-map.md`, crédits OSM

## Prochaine étape
Données Rouen (enceinte, monuments, quartiers depuis OSM), puis stubs Godot.
