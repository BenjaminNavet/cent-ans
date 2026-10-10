# TW bfeel — ressenti de bataille (branche tw/bfeel)

État :
- FAIT cœur : `rallied` (get_units, fenêtre `rally.rallied_flag_s` = 3 s), alerte `rallied`, `low_ammo` (status, `status.low_ammo_ratio`), `time_left_s` (`BattleSim.get_time_left_s`). Tests Rust OK.
- FAIT GDScript (à tester en headless : `res://tests/bfeel_test.gd`) : anneau d'ordre, barks hold/formation/retreat/rally, pastilles, HUD temps restant, ralenti général, plan de victoire, infobulle repère.
- Clips manquants (aucune dépense) : fr_hold_01, fr_formation_01, fr_retreat_01, fr_rally_01, fr_rally_02 et leurs équivalents en_ ; les langues régionales se replient sur fr/en.
