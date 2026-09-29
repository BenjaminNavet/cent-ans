# NT3 — missions de campagne

Branche `feat/nt3-missions`. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (NT3).
ADR : `docs/decisions/0127-missions-de-campagne.md`.

## État
- [x] Données `data/missions.json` + schéma `data/schemas/missions.schema.json`, chargement `GameData::mission_rules`
- [x] Squelette `core/crates/sim-campaign/src/missions.rs`, champ `CampaignState::missions` (serde default)
- [x] Crochets : bataille gagnée (`battle_outcome::apply`), recrutement/engagement (`submit_order_outcome`), fin de tour (`turn.rs`)
- [x] Implémentation (génération, progression, réussite, échec, récompense) + tests Rust (`tests/nt3_missions.rs`)
- [x] Pont `campaign_sim_missions.rs`, UI (`victory_controller.gd` section Missions + toasts, `season_report.gd`), test `game/tests/nt3_missions_test.gd` écrits — à vérifier (build.sh, Godot)
- [x] pytest schéma (`tools/tests/test_missions_schema.py`)
- [x] ADR 0127 (`docs/decisions/0127-missions-de-campagne.md`)

## Prochaine étape
core/build.sh, test Godot, smoke, clippy workspace, ADR.
