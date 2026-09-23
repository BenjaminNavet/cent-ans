# WIP — H3 « La Table » + H4 « Médecine » (règles, données, pont)

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` § 2-3. API : `docs/design/h3-h4-api.md`.

## Plan
1. [x] Squelette : `DietId`, `Diet` (data-model), schéma `diet.schema.json`, `TechBranch::Medicine`,
   `EffectKind::{PlagueResistance, WoundRecovery, DietHealth}`, `Technology::herbs`,
   `sim-campaign::{table, medicine}`, `Order::SetDiet`, champs d'état (serde default).
2. [x] Règles branchées dans le tour (exigences, paiement « Table », Carême, effets population,
   peste locale, Peste noire, épidémie locale, blessés, IA des régimes, IA de recherche à 3 branches).
3. [x] Données : 7 régimes, 12 techs médecine (+ migration quarantaine / réforme hospitalière),
   2 bâtiments, `_diet_links.md`, `_herb_links.md`, icônes + CREDITS.
4. [x] Pont Godot (`campaign_sim_table.rs`, branche médecine, herbes), onglet Médecine du panneau.
5. [x] Tests (`sim-campaign/tests/h3_h4.rs`, `real_data.rs`, m6 adapté), fmt/clippy/test, pytest,
   build + import + smoke Godot OK, doc API.

## État
Terminé.

## Prochaine étape (vague 2, UI)
Section « La Table » du panneau de province, bandeau Carême, libellés des nouveaux effets dans
`rich_tooltip.gd`, genres `table`/`medicine` dans `season_report.gd`, Herbier (lot codex).
