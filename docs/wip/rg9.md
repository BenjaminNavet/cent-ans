# RG9 — Déplacement : routes, fleuves, cols, général

État : squelette (modules `terrain_mods.rs` et `river_nav.rs` vides, ADR 0343 réservé).

Plan :
1. Données : `data/movement/rules.json` (+ schéma), bâtiment Route, `data/movement/rivers.json`, compétences.
2. Cœur : grille de coûts effective par armée (route, col, hiver), allocation (général), navigation fluviale (ordre `SailRiver`).
3. Pont Godot : décomposition du trajet, ordre fluvial.
4. Tests, ADR 0343, INDEX.

Prochaine étape : data-model (EffectKind, règles, fleuves).
