# NT3 — missions de campagne

Branche `feat/nt3-missions`. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (NT3).
ADR : `docs/decisions/0127-missions-de-campagne.md`.

## État : terminé, prêt à fusionner dans `integration/nt`
- [x] Données `data/missions.json` + schéma `data/schemas/missions.schema.json`, chargement `GameData::mission_rules`
- [x] `core/crates/sim-campaign/src/missions.rs`, champ `CampaignState::missions` (serde default)
- [x] Crochets : bataille gagnée (`battle_outcome::apply`), recrutement/engagement (`submit_order_outcome`), fin de tour (`turn.rs`, après la victoire)
- [x] Tests Rust `tests/nt3_missions.rs` (9) ; workspace `cargo test --no-fail-fast` vert, clippy vert
- [x] Pont `campaign_sim_missions.rs` (`get_missions`, `get_mission_notices`), UI (`victory_controller.gd` section Missions + toasts, `season_report.gd` genre `mission`)
- [x] Test Godot `game/tests/nt3_missions_test.gd` OK, smoke OK, pytest (1274) OK
- [x] ADR 0127

## Suite (demande orchestrateur, après fusion df36171e4)
- [x] Panneau d'objectifs défilant (`ObjectivesScroll`, hauteur ≤ 420 px, réduite si fenêtre basse), test 1280×720 dans `nt3_missions_test.gd`
- [x] Assaut de siège gagné (prise ou assaut repoussé) = bataille gagnée (`siege::apply_assault_result`), test `a_won_assault_counts_as_a_won_battle`

## Points ouverts
- Équilibre des récompenses (or/prestige) à juger en partie pilote.
- Les sorties de garnison ne comptent pas comme batailles gagnées.
