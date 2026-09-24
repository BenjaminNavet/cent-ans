# WIP C6 — agents de campagne

Branche : `worktree-agent-a64ab636a1a413848`. Conception : `docs/design/2026-09-24-agents.md`, ADR 0009.

## État
- [x] (a) conception, ADR, données `data/rules/agents.json` + schéma + test Python, entité `data-model::AgentRules`
- [x] (a)+(b) `sim-campaign/src/agents.rs` complet (ordres, actions, contre-espionnage, vision, IA `plan_agents` appelée par `ai::plan_turn`) + tests `sim-campaign/tests/c6_agents.rs` (17) et `ai/tests/c6_agents_ai.rs`
- [x] pont `godot-bridge/src/campaign_sim_agents.rs` (compile)
- [x] (c) UI : `game/scripts/map/agent_controller.gd` (jetons, sélection, anneaux, barre d'actions, registre G, clic droit = marche), onglet Agents de l'encyclopédie, rubrique Agents du rapport de saison, smoke `_run_agents` (23 « smoke OK » attendus)
- [ ] (d) IA + équilibrage + captures `docs/img/c6/`

## Prochaine étape
(d) captures `--stage=agents` dans `docs/img/c6/`, sonde d'équilibrage IA (nombre d'agents, actions par saison), relance du smoke (ordre marche puis action corrigé).

## Notes
- Ne pas toucher `movement.rs`, `ProvinceState`, `SettlementState`, `characters.rs`.
- Tirages : générateur dérivé (graine, tour, agent, action), pas `state.rng`.
