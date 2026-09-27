# CV3-6 — IA des postures et des rencontres, équilibrage CV3

Branche : `worktree-agent-abcec86700c5ee43d` (worktree privé, base 5704fa8c).
Spec : `docs/design/2026-09-27-campagne-vivante.md` § 0 et § 5. ADR 0094 (postures), ADR 0085 (bande EQ6).

## Choix
- L'IA de campagne réelle est `core/crates/ai/src/campaign.rs` (`plan_armies`) + `grid.rs` ;
  `sim_campaign::ai_minimal` ne sert qu'aux tests de la simulation : non modifié.
- Nouveau module `core/crates/ai/src/stances.rs` : décisions d'embuscade, de marche forcée, de
  camp retranché et de détour vers une rencontre, appelées depuis `plan_armies`.
- Réglages : section `postures` et `encounters` de `data/ai/grid.json` (fichier de réglages des
  armées de l'IA ; schéma `ai_grid.schema.json`). Défauts du cœur = rien (comportement antérieur).

## État
- [x] Squelette : structures `AiPostures`/`AiEncounters` (data-model), schéma.
- [ ] Module `stances.rs` et branchement dans `plan_armies`.
- [ ] Tests unitaires par décision, test « aucun ordre de posture refusé », déterminisme.
- [ ] Sonde : compteurs CV3 (`CV3_STATS=1`) dans `century_probe`.
- [ ] Mesures avant/après (century_probe 464 tours, 4 difficultés, graines EQ6 ; balance_probe).

## Prochaine étape
Mesure « avant » puis implémentation des décisions.
