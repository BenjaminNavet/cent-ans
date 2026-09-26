# CT1 — tour de l'IA à la Total War : marches visibles, caméra qui suit, vitesse (état)

Branche : worktree `agent-ac735a99732c7b4b8`, partie de main `168b1acf`. ADR : `docs/decisions/0070-relecture-du-tour-ia.md`.

## État

- [x] Cœur : `sim-campaign/src/ai_replay.rs` (`AiMoveRecord`, `AiMoveKind`, `AiMoveNotability`, `CampaignState::set_ai_replay_recording`, `ai_turn_moves`), branché dans `turn.rs` (`continue_ai_marches`, `apply_ai_order`, `ai_replay_begin/finish`). Coût nul quand l'enregistrement est coupé.
- [x] Pont : `campaign_sim_ai_replay.rs` (`set_ai_turn_recording`, `get_ai_turn_moves`).
- [x] Données : `data/ui/ai_turn_replay.json` + schéma + pytest.
- [x] Test Rust `ai/tests/ct1_ai_replay.rs` (déterminisme, jeu inchangé, bataille contre le joueur notable).
- [ ] Godot : `game/scripts/map/ai_turn_replay.gd`, crochet dans `campaign_map.gd::_on_end_turn`, réglages (`map/ai_moves`, `map/ai_moves_speed`) + menu.
- [ ] Test Godot `game/tests/ct1_ai_turn_test.gd`.
- [ ] Captures `docs/audit/captures/ct1/`, mesures de perf par mode.
- [ ] ADR, doc, fusion de main.

## Prochaine étape

Écrire `ai_turn_replay.gd` et le crochet.
