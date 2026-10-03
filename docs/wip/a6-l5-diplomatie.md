# A6-L5 — diplomatie (M6, U15, U16)

État : implémenté (ADR 0182). Acceptation du joueur déterministe (score >= 0), étiquette
« Accepterait/Refuserait (+N) », panneau tenant dans 1280×720, bouton « Commerce ».
Test Godot : `game/tests/a6_diplomacy_layout_test.gd` ; test Rust : `a6_deterministic_acceptance.rs`.
Reste : vérifier `cargo test -p sim-campaign` complet (long), contrôle visuel par la session principale.
