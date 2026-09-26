# CT1 — tour de l'IA à la Total War : marches visibles, caméra qui suit, vitesse (état)

Branche : `ct1-ai-turn-replay` (worktree `agent-ac735a99732c7b4b8`), partie de main `168b1acf`. ADR : `docs/decisions/0070-relecture-du-tour-ia.md`.

## État

- [x] Cœur : `sim-campaign/src/ai_replay.rs` (`AiMoveRecord`, `AiMoveKind`, `AiMoveNotability`, `CampaignState::set_ai_replay_recording`, `ai_turn_moves`), branché dans `turn.rs` (`continue_ai_marches`, `apply_ai_order`, `ai_replay_begin/finish`). Coût nul quand l'enregistrement est coupé.
- [x] Pont : `campaign_sim_ai_replay.rs` (`set_ai_turn_recording`, `get_ai_turn_moves`).
- [x] Données : `data/ui/ai_turn_replay.json` + schéma + pytest.
- [x] Test Rust `ai/tests/ct1_ai_replay.rs` (déterminisme, jeu inchangé, bataille contre le joueur notable).
- [x] Godot : `game/scripts/map/ai_turn_replay.gd`, crochet dans `campaign_map.gd::_on_end_turn` (3 ajouts), réglages (`map/ai_moves`, `map/ai_moves_speed`) + menu Carte.
- [x] Test Godot `game/tests/ct1_ai_turn_test.gd` (vert).
- [x] Captures `docs/audit/captures/ct1/suivi_{1,2,3}.png` (`game/tests/ct1_capture.gd`, fenêtre).
- [x] Alliés et vassaux jamais suivis (sauf bataille/siège contre le joueur).
- [x] ADR 0070, doc `docs/godot-map.md` § CT1.
- [ ] Vérifications complètes, fusion de main.

## Mesures

- Cœur (release, 3 tours) : enregistrement actif 277 ms contre 271 ms coupé, soit ≈ 2 ms par tour.
- Godot headless (dylib debug) : partie synchrone de `_on_end_turn` ≈ 190-240 ms dans les trois modes
  (bruit) ; tri des mouvements 0,05-0,09 ms ; relecture ×4 : Montrer ≈ 0,4 s, Suivre (4 suivis) ≈ 3,7 s.

## Prochaine étape

Vérifications complètes (fmt, clippy, cargo test, pytest, smoke, tests M4/M5a), `git merge main`, rapport.
