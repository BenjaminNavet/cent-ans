# A6-L15 — barre des emplacements de colonie (U14)

État : terminé. Cœur (`building_slots`), pont (`settlement_slots`), composant
`game/scripts/map/settlement_slot_bar.gd`, intégration `map_ui.gd` / `settlement_controller.gd`,
test `game/tests/a6_slot_bar_test.gd` (OK), ADR 0181.

Points ouverts : pas de verrou « par taille » ni de plafond d'emplacements dans le cœur (dérivation
par chaînes d'amélioration, voir ADR) ; le clic ouvre l'onglet Bâtiments et donne le focus à la ligne
(settlement_panel.gd non modifié, lot L7).
