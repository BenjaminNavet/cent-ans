# tx-m1-rancunes (ADR 0720)
Spec : ~/dev/rea-tww3/docs/wip/integration-cent-ans.md, fiche M1. Branche `tx/m1-rancunes`.

## État
- [x] Squelette : ADR, module `grudges.rs` (API vide), tests `tx_m1_*` ignorés.
- [x] Données `grudge` dans missions.json + schéma + data-model
- [x] Crochets : capture (on_captured), battle_outcome::apply (classe Defeat), siege::resolve_raids
- [x] Offre (poids), annulation (is_live/prune), récompense par âge, tests tx_m1_* (campaign_life)
- [x] Vue MissionView/OfferCandidateView.grudge_age + infobulle client (non compilé côté client)
- [x] fmt, clippy workspace OK ; cargo test workspace OK (39 suites), clippy client OK

## Prochaine étape
Test de crochet on_captured (capture_tests), game-api/client (grudge_age), fmt/clippy/tests workspace.
