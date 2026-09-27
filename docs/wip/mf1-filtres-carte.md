# MF1 — Filtres de la carte de campagne

Terminé le 25/09/2026 (ADR 0048). Branche `feat/map-modes`.

## Fait
- Cœur : `sim_campaign::map_lens` (revenu, population, mécontentement, loyauté du vassal,
  ravitaillement, revendications), `economy::seasonal_supply_change` partagé avec
  `resolve_attrition`, tests `core/crates/sim-campaign/tests/mf1_map_lens.rs`.
- Pont : `CampaignSim.get_map_lens(ids)` (`campaign_sim_map_lens.rs`).
- Godot : `MapModeController` (mode unique, menu, légende, valeur au survol, marqueurs daltoniens
  de la diplomatie), bouton « Filtres » (F) sur la minicarte, routes commerciales en case du menu
  et sur V (fin du conflit avec R = religion).
- Correction : le mode mécontentement saturait au rouge dès 1 % (0-100 non divisé par 100).
- Tests : `game/tests/mf1_map_modes_test.gd` (headless), captures `game/tests/mf1_shot.gd`.

## Limites / suites possibles
- Richesse = impôt de base, sans taux d'imposition ni bâtiments.
- Ravitaillement sans l'effet du général (compétence Supply) ni des colonies amies isolées.
- Pas de raccourci pour richesse, population, loyauté, ravitaillement, revendications (menu F).

## Note de fusion (25/09)
- UX1 (légende de la carte) rebranchée sur `MapModeController` ; `MapLegend.set_mode` compare le
  mode normalisé (sinon reconstruction à chaque image sur un filtre sans section).
- `tests/smoke.gd` s'arrête dans `_run_campaign_loop` sur « Message queue out of memory »
  (`Control::_update_minimum_size`), **y compris sans `MapModeController`** : régression antérieure
  venue de `main` (probablement UI1, métriques de police / thème enluminé), à traiter par ce lot.
