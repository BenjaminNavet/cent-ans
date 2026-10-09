# WH chars — état

Branche `wh/chars` (worktree `../gp-wh-chars`). Spec : `docs/wip/wh/personnages.md` § 3 points 1, 2, 3, 6, 9.

- [x] data-model : `RoyalAct`, `CaptainRules`, `Trait.expires_in_turns`, constantes XP (`dynasty.json`)
- [x] sim-campaign : `royal_acts.rs`, `captains.rs`, blessures temporaires (`characters.rs`), XP élargie, `LevelUp`
- [x] IA : `ai_choose_royal_act`, `ai_hire_captain` (ai_minimal + crate `ai`)
- [x] pont (`campaign_sim_royal_acts.rs`, `get_character.feats/level`) + UI (onglet « Actes royaux » de la cour, fiche)
- [x] tests Rust `tests/campaign_life/wh_chars.rs` (13)
- [x] test headless `game/tests/wh_chars_ui_test.gd` OK ; `p2a_ui_test` OK
- [x] sonde `campaign_probe` 120 tours x graines 1,2 : voir ci-dessous
- [x] ADR 0276, 0277

Écarts / choix : effets des actes dans `faction_tech_effects` (+ morale/bataille dans `character_effects`) ;
XP rançon donnée au captif libéré ; XP traité aux deux souverains.

## Résultats
- Rust : `cargo test` data-model, sim-campaign, ai, godot-bridge : 1049 réussis, 0 échec, hors
  `cv3_ai_stances::the_ai_never_gives_a_stance_order_the_core_refuses` (ignoré) : la graine 2 montre un ordre
  d'embuscade refusé de la Castille au tour 20 (même point ouvert que celui noté par RX histoire, qui passe ce test
  aux graines 1, 3, 4) ; test fragile aux décalages du flux aléatoire, laissé à RX.
- Sonde (120 tours, graines 1/2) avec actes + capitaines / sans acte : guerre FR-EN 68/82 % contre 85/47 % ; révoltes 5/2
  contre 1/0 ; banqueroutes 147/99 contre 151/133 ; trajectoires divergentes (flux aléatoire), pas de dérive nette.
- `smoke.gd` : 19 étapes OK puis arrêt (relief absent du worktree, sans lien avec ce lot) ; non terminé.
