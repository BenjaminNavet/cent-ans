# ZG6 — villes ordinaires à l'échelle réelle vers 1340 (ADR 0036)

Branche `worktree-agent-a8f477a463e9704f3` (worktree d'agent, depuis `main` 08eecdd8).
Liens symboliques non versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
Dylib : `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge`
puis copie dans `game/bin/libcent_ans.debug.dylib`. Aucun changement Rust prévu.

## Architecture (prévue)
- Données : `data/rules/town_footprint.json` (entrées sourcées : densités, populations de
  référence, enceintes, parcelles) → outil `cent-ans geo towns` (`tools/cent_ans_tools/geo/towns.py`)
  → `data/map/towns_1340.json` (emprise par colonie ordinaire : population estimée, surface
  intra-muros, polygone d'enceinte adapté au relief, portes, routes d'accès, fleuve, pont,
  monuments, faubourgs, rayon de finage). Schémas dans `data/schemas/`.
- Godot : `town_data.gd` (lecture), `town_plan.gd` (plan procédural pur et déterministe, fil de
  travail : rues, îlots, parcelles en lanières, bâtiments), `town_builder.gd` (MultiMesh du kit,
  murailles), `town_layer.gd` (streaming, HLOD, recalages), `shaders/town_building.gdshader`
  (hauteurs de base en mètres × `campaign_vertical_scale`).
- Point d'accroche dans `settlement_layer.gd` : création du `TownLayer`, maquettes des villes
  ordinaires masquées aux paliers vallée/site.

## État
- [ ] Squelette (wip, stubs)
- [ ] Données sourcées + outil + tests pytest
- [ ] Kit bas détail pour la campagne (`game/assets/models/town_kit/`)
- [ ] Plan procédural (GDScript) + test headless
- [ ] Rendu 1:1, HLOD, streaming
- [ ] Banc, captures, docs

## Prochaine étape
Squelette puis données.
