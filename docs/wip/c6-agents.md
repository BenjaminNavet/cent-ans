# WIP C6 — agents de campagne

Branche : `worktree-agent-a64ab636a1a413848`. Conception : `docs/design/2026-09-24-agents.md`, ADR 0009.

## État
- [x] (a) conception, ADR, données `data/rules/agents.json` + schéma + test Python, entité `data-model::AgentRules`
- [ ] (a) squelette `sim-campaign/src/agents.rs`
- [ ] (b) cœur + tests (`sim-campaign/tests/c6_agents.rs`)
- [ ] (c) pont `godot-bridge/src/campaign_sim_agents.rs` + UI (pions, barre d'actions, rapport, encyclopédie, smoke)
- [ ] (d) IA + équilibrage + captures `docs/img/c6/`

## Prochaine étape
Écrire `agents.rs` (types, ordres, actions), brancher `Order`, `turn.rs`, `vision.rs`.

## Notes
- Ne pas toucher `movement.rs`, `ProvinceState`, `SettlementState`, `characters.rs`.
- Tirages : générateur dérivé (graine, tour, agent, action), pas `state.rng`.
