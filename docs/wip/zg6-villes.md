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
- [x] Squelette (wip)
- [x] Données sourcées + outil `cent-ans geo towns` + tests pytest (`tools/tests/test_towns.py`)
- [x] Kit bas détail pour la campagne (`game/assets/models/town_kit/`, `kit_export.py export-town`)
- [x] Plan procédural (`town_plan.gd`) + test headless `game/tests/zg6_towns_test.gd` (OK)
- [x] Rendu 1:1 (`town_builder.gd`, `town_layer.gd`, `town_building.gdshader`, `town_render.tres`),
      accroche dans `settlement_layer.gd` (`_setup_towns`, `_update_towns`)
- [x] Essai en jeu (captures), réglages visuels
- [x] Construction hors fil principal (`TownBuilder.prepare`), banc `--bench-towns`
- [x] Finage raccordé au parcellaire ZG5b (`fp_towns` dans `fine_parcels.gdshaderinc`)
- [x] Captures avant/après `docs/img/zg6/` (JPEG 960 px), docs (`godot-map.md`, addendum ADR 0036)
- [x] Fusion de main (ZG5b, EP8), `cent-ans geo towns` relancé

## Prochaine étape
Lot terminé, en attente d'intégration par l'orchestrateur. Suites pour VH3 : caler les populations
sur l'état des feux de 1328, affiner villages (plans en village-tas, village-rue), réduire les appels
de dessin (un MultiMesh par modèle et par ville au lieu de par cellule, ou fusion des blocs).

## 2026-09-25 — construction hors fil principal
- `TownBuilder.prepare(plan)` (fil de travail, après `TownPlan.generate`/`reground`) : tampons MultiMesh par cellule/modèle, tableaux de sol (bandes de 24 rangées), rues (paquets de 40), murailles via `SurfaceTool.commit_to_arrays`.
- Fil principal : création des nœuds seulement, une tâche indivisible ≤ ~1-2 ms (charge machine ~100) ; modèles du kit chargés un par tâche. Stat `build_task_max_ms`.
- Prochaine étape : banc de descente (villes ordinaires), captures `docs/img/zg6/`, docs, fusion de main + `cent-ans geo towns`.
