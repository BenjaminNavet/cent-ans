# A6-L15 — barre des emplacements de colonie (U14)

État : cœur fait (`building_slots`, pont `settlement_slots`, test Rust). Reste : composant GDScript
`game/scripts/map/settlement_slot_bar.gd`, intégration `map_ui.gd` / `settlement_controller.gd`,
test `game/tests/a6_slot_bar_test.gd`, ADR 0181.

Point de conception signalé : le cœur n'a pas de notion d'emplacements ni de verrou par taille.
Dérivation provisoire : un emplacement = une chaîne d'amélioration (`upgrades_from`) des bâtiments
permis pour le type de colonie ; verrou = ressource / côte / rivière / technologie / prérequis.
