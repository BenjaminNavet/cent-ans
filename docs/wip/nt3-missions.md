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

## Points ouverts
- Panneau d'objectifs sans défilement : 3+ objectifs + 2 missions peuvent dépasser 720 px de haut (à juger en partie pilote).
- Équilibre des récompenses (or/prestige) à juger en partie pilote.
- Les assauts de siège ne comptent pas comme batailles gagnées.
