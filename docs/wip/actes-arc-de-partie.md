# Lot ACTES — arc de partie en trois actes (ADR 0784-0785)

Branche : worktree-agent-a295039522a70fda4.

## État : terminé (hors captures)
- Core : `data_model::ActsRules`, `sim_campaign::acts` (état, conditions, score, ruine, migration v10 -> v11),
  `victory::resolve_with_acts` (victoire = score de règne), `STATE_VERSION` 11, fixture `save_v11.json`.
- Données : `data/rules/acts.json` (généré par un script jetable), `data/schemas/acts_rules.schema.json`.
- game-api : `acts.rs` (vue de l'acte, transition, choix de politique), `progress.rs` (objectifs de l'acte, fin de partie).
- Client : `client/src/campaign/progress/acts.rs` (écran de transition, boutons `acts:policy:<id>`, `acts:continue`,
  hook `CENT_ANS_FORCE_ACT=1`, dump `progress_acts`). Aucune capture prise.
- Tests : `tests/campaign_life/acts.rs` (dont partie complète 1337-1454 : France passive -> Ended, score 66, actes 65/46/82).

## Points ouverts
- Rééquilibrer avec les seed bots (seuil 72, objectifs, partie passive à ~66).
- Captures de l'écran de transition (2 au plus) et dump `progress_acts` non faits.
- `ev_history.rs:669` : lint clippy `single_element_loop` préexistant sur main (non touché).
- Le suivi n'a pas de sonde `campaign_probe` (crate `ai` interdite) : le test de partie complète fait office de sonde.
