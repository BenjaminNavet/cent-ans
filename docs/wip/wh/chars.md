# WH chars — état

Branche `wh/chars` (worktree `../gp-wh-chars`). Spec : `docs/wip/wh/personnages.md` § 3 points 1, 2, 3, 6, 9.

- [x] data-model : `RoyalAct`, `CaptainRules`, `Trait.expires_in_turns`, constantes XP (`dynasty.json`)
- [x] sim-campaign : `royal_acts.rs`, `captains.rs`, blessures temporaires (`characters.rs`), XP élargie, `LevelUp`
- [x] IA : `ai_choose_royal_act`, `ai_hire_captain` (ai_minimal + crate `ai`)
- [x] pont (`campaign_sim_royal_acts.rs`, `get_character.feats/level`) + UI (onglet « Actes royaux » de la cour, fiche)
- [x] tests Rust `tests/campaign_life/wh_chars.rs` (13)
- [ ] test headless `game/tests/wh_chars_ui_test.gd` (à lancer après `core/build.sh`)
- [ ] sonde `campaign_probe` avant/après
- [ ] ADR 0276 (actes royaux), 0277 (capitaines, blessures, XP)

Écarts / choix : effets des actes dans `faction_tech_effects` (+ morale/bataille dans `character_effects`) ;
XP rançon donnée au captif libéré ; XP traité aux deux souverains.
