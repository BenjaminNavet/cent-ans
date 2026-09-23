# WIP M10 événements

Branche : worktree-agent-a71b0466416deac06. Spec : docs/design/m10-events.md.

## État
- [x] 1. Schéma `data/schemas/event.schema.json`, types `data-model` (`entities/event.rs`, `event_check.rs`), 48 événements `data/events/` (26 historiques, 22 aléatoires), test pytest `tools/tests/test_events_schema.py`.
- [ ] 2. `sim-campaign/src/chronicle.rs`, phase de tour, ordre `choose_event_option`, tests `tests/m10_events.rs`.
- [ ] 3. Pont `godot-bridge/src/campaign_sim_events.rs`.
- [ ] 4. Godot : fenêtre Chronique + couleur de journal.
- [ ] 5. Smoke `_run_chronicle`, capture, docs.

## Prochaine étape
Étape 2.
