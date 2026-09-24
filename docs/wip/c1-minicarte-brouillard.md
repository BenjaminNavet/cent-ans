# WIP — C1 minicarte de campagne + brouillard de guerre léger

Spéc : lot T1 de `docs/design/2026-09-24-analyse-total-war.md` ; plan `2026-09-24-rapprochement-total-war.md` (C1).

## État
- [x] Règle de vue côté Rust : `CampaignState::visible_provinces(data, faction)` (`core/crates/sim-campaign/src/vision.rs`),
  données `data/rules/vision.json` (schéma `vision_rules.schema.json`, chargé dans `GameData.vision_rules`),
  tests `core/crates/sim-campaign/tests/c1_vision.rs`, pytest `tools/tests/test_vision_rules_schema.py`.
- [x] Pont : `CampaignSim.get_visible_provinces(faction) -> PackedStringArray` (`campaign_sim_vision.rs`).
- [x] Minicarte `game/scripts/map/campaign_minimap.gd` + `minimap_controller.gd` + shader `campaign_minimap.gdshader`.
- [x] Brouillard : voile shader terrain, armées masquées (carte + minicarte), réglage `map/fog_of_war`.
- [x] Smoke étendu (`_run_minimap_fog`), vert.
- [x] Capture `docs/img/c1/campagne-minicarte-brouillard.png` (1440×900).

## Prochaine étape
Lot terminé, en attente de fusion. Points ouverts :
- 106 provinces sur 132 vues par la France en 1337 (alliés écossais, frontières longues) : le voile
  touche surtout l'Italie, l'Empire lointain, les îles. Régler `data/rules/vision.json` si trop généreux.
- Le panneau de province d'une province voilée montre encore garnison/état complets (non filtré).
- La minicarte n'affiche pas les agents/flottes (inexistants) ni les colonies (C5/C6).
- Recapturer après la fusion des colonies (nouveaux paliers de zoom).
