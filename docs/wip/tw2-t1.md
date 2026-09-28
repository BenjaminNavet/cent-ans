# TW2-T1 — Sort de la ville prise (wip)

Branche `feat/tw2-t1`. Spec : `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T1. ADR 0101.

## État
- [x] Données : `data/rules/capture.json`, schéma `data/schemas/capture_rules.schema.json`, test
  `tools/tests/test_capture_rules_schema.py`.
- [x] data-model : `entities/capture.rs` (`CaptureRules`, défaut = fichier), chargé dans `GameData::capture_rules`.
- [x] Cœur : `sim-campaign/src/capture.rs` (issues, aperçu, IA, décision en attente, ruines) ;
  `siege::capture` appelle `capture::on_captured` ; ordre `ChooseCaptureOutcome` ; état `captures`
  (serde default) ; `turn.rs` : décisions non tranchées = occupation ; ruine bloque recrutement/chantier.
- [ ] Tests Rust (`tests/tw2_t1_capture.rs`) + test d'égalité défaut/fichier (`data-model/tests/real_data.rs`).
- [ ] Pont : `godot-bridge/src/campaign_sim_capture.rs` (`get_pending_captures`, `choose_capture_outcome`).
- [ ] Godot : `CaptureController` (réutilise `ChronicleWindow`), branchement `campaign_map.gd`, test headless.
- [ ] ADR 0101, build.sh, --import, smoke, suppression de `core/target-t1`.

## Prochaine étape
Compiler (`CARGO_TARGET_DIR=core/target-t1`), écrire les tests Rust.
