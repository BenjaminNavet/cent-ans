# SC gdshared

État : `Hash` (game/scripts/util/hash.gd), `ConfirmPanel` + `ConfirmDialog.ask` (game/scripts/ui/) en place ; sites migrés (cicatrices de guerre, foule, HUD de bataille, tour de fin, raser, déclaration de guerre).
`army_markers.gd` (656 lignes) : sous le seuil de 800, non découpé. Menu pause (3 boutons) : hors périmètre oui/non.
Vérifié : import 0 erreur, smoke OK, tests raze/ub1_ui/quit_battle/fe_ui/fk_folk OK ; at1_attack_order : 1 échec sur l'assaut, étranger au lot.
