# TW2-T5 — Traditions d'armée

Branche `feat/tw2-t5` (worktree `../gp-tw2-t5`, base `integration/tw2`). Spec :
`docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T5. ADR 0112.
`CARGO_TARGET_DIR=core/target-t5`.

## Fait
- [x] Données `data/rules/army_traditions.json` + schéma + pytest ; `data-model` `ArmyTraditionRules`.
- [x] `sim-campaign/src/traditions.rs` : xp, rangs, vue, choix, dilution (`add_recruits`), nom/bannière.
- [x] Champs d'état `Army::traditions`, `Unit::experience_residue` (serde default) ; ordre `ChooseArmyTradition`.

- [x] Accroches : xp (bataille de campagne, 3D, assaut, sortie), mouvement, reconstitution + dilution (et garnisons), siège, moral/tir (auto + 3D) ; tests Rust `tw2_t5_traditions.rs` verts.

- [x] IA : `ai/src/traditions.rs` (doctrine de tir, hommes manquants, siège, branche entamée), branché dans `plan_turn` ; test `ai/tests/tw2_t5_traditions_ai.rs`.

- [x] Pont `campaign_sim_traditions.rs` : `get_army_traditions`, `get_armies_with_pending_traditions`, `choose_army_tradition`, `debug_grant_army_xp`.

- [x] UI : `game/scripts/map/traditions_controller.gd` (bouton du bandeau, panneau latéral, toasts de rang), titre du bandeau = nom gardé.

- [x] Test headless `game/tests/tw2_t5_traditions_test.gd` (OK).

- [x] ADR 0112 ; fmt, clippy -D warnings, build.sh, import, smoke, test headless, pytest (884) verts.

## Prochaine étape
Attendre `cargo test --workspace` puis rapport ; fusion dans `integration/tw2` par l'orchestrateur.
