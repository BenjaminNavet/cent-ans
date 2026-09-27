# CV3-3 — Rencontres sur la carte de campagne

Branche : `feat/cv3-3-encounters`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 2.

## État
- [x] Squelette : types data-model (`entities/encounter.rs`, `EncounterId`), chargement (`folders::ENCOUNTERS`,
  `ENCOUNTER_RULES`), schémas `encounter.schema.json` + `encounter_rules.schema.json`, `data/rules/encounters.json`,
  module `sim-campaign/src/encounter.rs` (API vide), hooks (march.rs fin de marche, turn.rs début de saison +
  réponses par défaut, movement.rs fin de `apply_battle_result`), ordre `choose_encounter_option`, pont
  `campaign_sim_encounters.rs`, test Python, tests Rust `#[ignore]`.
- [ ] Moteur (apparition, expiration, déclenchement, choix, bataille, ralliement, vues).
- [ ] ~12 rencontres sourcées `data/encounters/enc_*.json`.
- [ ] Tests Rust `cv3_encounters.rs`, pont, smoke.

## Fichiers partagés avec CV3-1 (touchés en blocs courts)
- `state.rs` : champ `encounters` (+ `empty()`).
- `orders.rs` : variante `ChooseEncounterOption`, `OrderError::Encounter`, un bras de dispatch, motif `unreachable`.
- `march.rs` : un appel `encounter::on_march_end` à la fin de `march()`.
- `movement.rs` : un appel `encounter::after_battle` à la fin de `apply_battle_result`.
- `turn.rs` : deux appels (`resolve_unanswered`, `start_season`).

## Prochaine étape
Implémenter `encounter.rs`.
