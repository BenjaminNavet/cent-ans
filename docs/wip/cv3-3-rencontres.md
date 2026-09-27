# CV3-3 — Rencontres sur la carte de campagne

Branche : `feat/cv3-3-encounters`. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 2.

## État : terminé (prêt à fusionner)
- Données : `data-model/src/entities/encounter.rs` (`Encounter`, `EncounterSpawn`, `EncounterOption`,
  `EncounterOutcome` battle/join, `EncounterRules`), id `enc_`, chargement `data/encounters/` et
  `data/rules/encounters.json` ; schémas `encounter.schema.json` (conditions/effets repris de
  `event.schema.json`) et `encounter_rules.schema.json`.
- Moteur `sim-campaign/src/encounter.rs` : apparition en début de saison (expiration puis jusqu'à
  `max_active`, au plus `spawn_per_season`), cellule praticable de la province, loin des colonies et des
  autres sites, RNG dérivé de la graine (le flux principal n'est pas touché) ; déclenchement en fin de marche
  (joueur → `PendingEncounter`, IA → tirage pondéré immédiat) ; ordre `choose_encounter_option` ; option par
  défaut en début d'`end_turn` ; issue `battle` = troupe `fac_rebels` + `movement::fight` (guerre temporaire
  avec les rebelles, retirée après la bataille), `on_win`/`on_loss` appliqués dans `apply_battle_result` ;
  issue `join` bornée par `max_army_units` ; vues filtrées par la vision.
- 12 rencontres sourcées `data/encounters/enc_*.json`.
- Pont `campaign_sim_encounters.rs` : `get_encounter_sites()`, `get_pending_encounters()`,
  `choose_encounter_option(army, site, option)`.
- Tests : `sim-campaign/tests/cv3_encounters.rs` (16), `tools/tests/test_encounter_schema.py` (2) ; smoke OK.

## Fichiers partagés avec CV3-1 (blocs courts)
- `state.rs` : champ `encounters` (+ `empty()`).
- `orders.rs` : variante `ChooseEncounterOption`, `OrderError::Encounter`, un bras de dispatch, motif `unreachable`.
- `march.rs` : un appel `encounter::on_march_end` à la fin de `march()`.
- `movement.rs` : un appel `encounter::after_battle` à la fin de `apply_battle_result`.
- `turn.rs` : deux appels (`resolve_unanswered`, `start_season`).

## Points ouverts
- L'IA ne vise pas les sites (lot CV3-6) : elle ne les rencontre que par hasard.
- Filtre d'apparition par terrain de province (`Terrain`), pas par biome de cellule.
- Équilibrage (fréquence, montants) à revoir avec `century_probe` (CV3-6).
