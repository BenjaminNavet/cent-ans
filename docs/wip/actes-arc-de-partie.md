# Lot ACTES — arc de partie en trois actes (ADR 0784-0785)

Branche : worktree-agent-a295039522a70fda4.

## État
- Fait : `data_model::ActsRules` (entities/acts_rules.rs), `sim_campaign::acts` (état, conditions, score, ruine,
  migration v10->v11), `victory::resolve_with_acts` (victoire = score de règne).
- Reste : data/rules/acts.json + schéma, tests, fixture save_v11, game-api, client, ADR, sonde.

## Prochaine étape
Écrire les données (3 actes, France/Angleterre/Bourgogne + gouvernements).
