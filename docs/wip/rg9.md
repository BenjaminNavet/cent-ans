# RG9 — Déplacement : routes, fleuves, cols, général

État (ADR 0343) : cœur, données, pont Godot, UI d'infobulle, tests rg9 et ADR faits.

Fait :
- `terrain_mods.rs` : grille dérivée par armée (routes de province, cols, hiver), cache `GameData::derived_navgrid`.
- `river_nav.rs` : `river_voyages_all`, ordre `SailRiver`.
- Allocation : bonus de niveau du général + `movement` % du général sur les points (`allowance.rs`).
- Données : règles (`rules.json`), `rivers.json`, bâtiments `bld_gravel_road`/`bld_royal_road`, compétences Éclaireur/Montagnard/Batelier, effets `road_movement`, `winter_movement`, `river_movement`.
- Godot : `army_movement_controller.gd` (décomposition du coût, aperçu et ordre fluvial).

Reste : suite complète `cargo test`, smoke Godot, vérif. régression IA ; icônes des deux bâtiments (à générer).
