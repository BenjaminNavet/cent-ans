# TW2-T1 — Sort de la ville prise

Branche `feat/tw2-t1`. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T1. ADR 0101.

## État : terminé (prêt à fusionner)
- Données : `data/rules/capture.json`, schéma `data/schemas/capture_rules.schema.json`, test
  `tools/tests/test_capture_rules_schema.py`.
- data-model : `entities/capture.rs` (`CaptureRules`, défaut = fichier), `GameData::capture_rules`.
- Cœur : `sim-campaign/src/capture.rs` (issues, aperçu, IA, décision en attente, ruines) ;
  `siege::capture` appelle `capture::on_captured` ; ordre `ChooseCaptureOutcome` ; état `captures`
  (serde default) ; `turn.rs` : décisions non tranchées = occupation ; ruine bloque recrutement/chantier.
  Tests : `sim-campaign/src/capture_tests.rs` (10 tests). `cargo test --workspace` vert, clippy propre.
- Pont : `godot-bridge/src/campaign_sim_capture.rs` (`get_pending_captures`, `choose_capture_outcome`,
  `debug_capture_place`).
- Godot : `game/scripts/map/capture_controller.gd` (réutilise `ChronicleWindow`, qui gagne un libellé
  de genre et des choix grisés avec raison), branché dans `campaign_map.gd` (`capture_fate`).
  Test headless `game/tests/tw2_t1_capture_test.gd` OK ; smoke OK.

## Points ouverts
- La liste de recrutement (`recruit_options`) ne signale pas encore la ruine : l'ordre est refusé
  avec `SettlementRuined`, mais le bouton n'est pas grisé à l'avance.
- Pas de marqueur de ruine sur la carte.
- Équilibre : tous les tests d'équilibre passent ; à observer en partie pilote (fréquence des
  rançons/pillages de l'IA pauvre, rasages écossais).
