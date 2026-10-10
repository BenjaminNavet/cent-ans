# TW m2b — foi et dynastie

Branche `tw/m2b`, ADR 0326 (excommunication, interdit, conversion) et 0327 (croisade papale, prétention par mariage).

## État : FAIT
- `data/rules/religion.json` + schéma `religion_rules`, `data_model::ReligionRules`, modules `conversion.rs` et `papal_crusade.rs`, hooks guerre/captif/naissance, pont (`get_religion_state`, `get_province_religion`), lignes UI (chancellerie, infobulle carte des religions).
- Tests : `core/crates/sim-campaign/tests/diplomacy/tw_m2b.rs` (14) + `preacher_speeds_the_conversion_of_a_foreign_province` (c6_agents).
- Sonde 120 tours x 2 graines : avant 200/250 guerres déclarées, révoltes 1/1 ; après 166/307, 1/0 (voir ADR 0327 pour l'incident de la croisade IA non bornée).

## Restes
- Bonus de moral temporaire des croisés ; ciblage d'un excommunié chrétien ; refus d'obédience ; contrôle visuel des lignes UI ; équilibrage de la vitesse de conversion (3-5 pts/saison) à regarder en partie.
