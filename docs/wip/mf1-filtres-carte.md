# MF1 — Filtres de la carte de campagne

Branche `feat/map-modes`, worktree `../game_project-mapmodes`.

## Objectif
- Un bouton « Filtres » (touche F) sur la minicarte ouvre un menu de filtres façon Total War.
- Un seul mode actif à la fois (`MapModeController`) : Politique, Diplomatie (N), Religion (R),
  Mécontentement (M), Richesse, Population, Loyauté des vassaux, Ravitaillement, Revendications.
- Routes commerciales = calque indépendant, combinable (touche V, plus de conflit avec R).
- Valeurs calculées dans le cœur : `sim_campaign::map_lens` → `CampaignSim.get_map_lens(ids)`.

## État
- [x] Cœur : `map_lens.rs`, `economy::seasonal_supply_change` (partagé avec `resolve_attrition`), tests `mf1_map_lens.rs`.
- [x] Pont : `campaign_sim_map_lens.rs`.
- [ ] Godot : `map_mode_controller.gd` (modes, teintes, légendes), menu de filtres, touches.
- [ ] Correction : l'ancien mode mécontentement saturait au rouge (unrest 0-100 non divisé par 100).
- [ ] Test Godot, fiche des raccourcis, fusion dans main.

## Prochaine étape
Écrire `game/scripts/map/map_mode_controller.gd` et y migrer les modes de `diplomacy_controller.gd`.
