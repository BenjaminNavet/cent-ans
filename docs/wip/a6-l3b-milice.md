# A6-L3b — Milice et armées de départ en données

Branche `a6-l3b` (depuis `a6-merge`). ADR : addendum à `docs/decisions/0183-economie-a-l-echelle.md`.

## État
- Problème 1 (milice) : fait. `share_caps` dans `data/ai/doctrines.json`, appliqué par `ai::doctrine::pick_recruit` ; la composition comptée inclut les garnisons (`ai/src/campaign.rs`). unit_urban_militia max_share 0,7 dès 4 régiments. Sonde 6 graines x 200 tours : milice 46,6 -> 27,6 %, banqueroutes 0,28 -> 0,30, révoltes 6,0 -> 6,3.
- Problème 2 : fait. `data/rules/starting_armies.json` + schéma + `GameData::starting_armies` + `setup_1337.rs`.
- Reste : vérification finale (tests), suppression de `core/target`. Pas de fusion.
