# NT3 — missions de campagne

Branche `feat/nt3-missions`. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (NT3).
ADR : `docs/decisions/0127-missions-de-campagne.md`.

## État
- [x] Données `data/missions.json` + schéma `data/schemas/missions.schema.json`, chargement `GameData::mission_rules`
- [x] Squelette `core/crates/sim-campaign/src/missions.rs`, champ `CampaignState::missions` (serde default)
- [x] Crochets : bataille gagnée (`battle_outcome::apply`), recrutement/engagement (`submit_order_outcome`), fin de tour (`turn.rs`)
- [ ] Implémentation (génération, progression, réussite, échec, récompense) + tests Rust
- [ ] Pont GDExtension (`campaign_sim_missions.rs`), UI panneau d'objectifs + avis, test Godot
- [ ] ADR 0127, pytest schéma

## Prochaine étape
Implémenter `resolve_missions` et les candidats par genre.
