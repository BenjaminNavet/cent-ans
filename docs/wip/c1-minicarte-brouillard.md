# WIP — C1 minicarte de campagne + brouillard de guerre léger

Spéc : lot T1 de `docs/design/2026-09-24-analyse-total-war.md` ; plan `2026-09-24-rapprochement-total-war.md` (C1).

## État
- [x] Règle de vue côté Rust : `CampaignState::visible_provinces(data, faction)` (`core/crates/sim-campaign/src/vision.rs`),
  données `data/rules/vision.json` (schéma `vision_rules.schema.json`, chargé dans `GameData.vision_rules`),
  tests `core/crates/sim-campaign/tests/c1_vision.rs`, pytest `tools/tests/test_vision_rules_schema.py`.
- [x] Pont : `CampaignSim.get_visible_provinces(faction) -> PackedStringArray` (`campaign_sim_vision.rs`).
- [ ] Minicarte `game/scripts/map/campaign_minimap.gd` + contrôleur.
- [ ] Brouillard : voile shader terrain, armées masquées (carte + minicarte), réglage `map/fog_of_war`.
- [ ] Smoke étendu, captures `docs/img/c1/`.

## Prochaine étape
Minicarte GDScript + accroches minimales dans `campaign_map.gd`.
