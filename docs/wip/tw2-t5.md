# TW2-T5 — Traditions d'armée

Branche `feat/tw2-t5` (worktree `../gp-tw2-t5`, base `integration/tw2`). Spec :
`docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T5. ADR 0109.
`CARGO_TARGET_DIR=core/target-t5`.

## Fait
- [x] Données `data/rules/army_traditions.json` + schéma + pytest ; `data-model` `ArmyTraditionRules`.
- [x] `sim-campaign/src/traditions.rs` : xp, rangs, vue, choix, dilution (`add_recruits`), nom/bannière.
- [x] Champs d'état `Army::traditions`, `Unit::experience_residue` (serde default) ; ordre `ChooseArmyTradition`.

## Prochaine étape
- Accroches : bataille (xp), mouvement, reconstitution (+ dilution), siège, bataille (moral/tir).
- Tests Rust `tests/tw2_t5_traditions.rs`, IA (`ai/src/traditions.rs`), pont, UI, test headless, ADR 0109.
