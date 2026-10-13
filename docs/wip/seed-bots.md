# Seed bots — note d'avancement

Spec : `docs/superpowers/specs/2026-10-13-seed-bots-design.md`. Plan :
`docs/superpowers/plans/2026-10-13-seed-bots-plan.md`. Worktree `../game_project-seed-bots`,
branche `seed-bots`. Feu vert du joueur reçu le 2026-10-13 : enchaîner toutes les vagues ; **arrêt
obligatoire avant la passe longue de référence** (lot F) ; aucun push.

## Écart avec la spec
- Pas de type `Action` : toutes les actions du joueur (diplomatie, recherche, édits, événements,
  offres…) sont déjà des variantes de `sim_campaign::Order` (~80, sérialisables). Les bots renvoient
  des `Order`. Les ordres `Debug*` sont exclus des candidats.

## État
- [x] Lot 0 — squelette (crate `core/crates/seed-bots`, `sim-campaign/src/invariants.rs`, profil
  `seed-bots`).
- [ ] Vague 1 : A (invariants + oracles 1-3), B (vue, candidats, random-legal, miroir), C (rapports, CLI, bandes).
- [ ] Vague 2 : D (runner, rejeu, mesures), E (6 bots).
- [ ] Vague 3 : F (équilibre ; arrêt avant passe longue), G (CI, docs, ADR 0347).

## Prochaine étape
Vague 1.
