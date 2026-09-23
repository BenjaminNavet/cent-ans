# WIP — H3 « La Table » + H4 « Médecine » (règles, données, pont)

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 2-3. API : `docs/design/h3-h4-api.md`.

## Plan
1. [x] Squelette : `DietId`, `Diet` (data-model), schéma `diet.schema.json`, `TechBranch::Medicine`,
   `EffectKind::{PlagueResistance, WoundRecovery, DietHealth}`, `Technology::herbs`,
   `sim-campaign::{table, medicine}`, `Order::SetDiet`, champs d'état (serde default).
2. [ ] Branchement des règles dans le tour (exigences, paiement « Table », Carême, effets population,
   peste, épidémie, blessés).
3. [ ] Données : 7 régimes, 12 techs médecine (+ migration quarantaine / hôtel-Dieu), 2 bâtiments,
   `_diet_links.md`, `_herb_links.md`, icônes.
4. [ ] Pont Godot (`campaign_sim_table.rs`, branche médecine), correctif minimal du panneau des techs.
5. [ ] Tests Rust + `real_data.rs`, fmt/clippy/test, build + smoke Godot, doc API.

## État
Squelette compilé.

## Prochaine étape
Brancher `table::resolve_requirements`, `pay_table`, `resolve_lent`, effets population, puis `medicine`.
