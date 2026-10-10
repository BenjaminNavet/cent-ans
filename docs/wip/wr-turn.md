# WR turn — missions en choix + filtre du journal (ADR 0304)

Branche `wr/turn` (worktree `../gp-wr-turn`).

## État
- Cœur : `MissionsState.offer` (`MissionOffer`), `Order::ChooseMission { choice: Option<usize> }`, expiration, IA `ai_choose_mission` (branchée dans `ai/campaign.rs` pour la faction du joueur). Données : `offer_candidates`, `offer_expiry_turns` dans `data/missions.json`.
- Pont : `get_mission_offer`, `choose_mission`.
- Godot : `MissionOfferController` (fenêtre `ChronicleWindow`), filtre de genre dans `JournalView` (`data/ui/journal_genres.json`).

## Tests
- Rust : `campaign_life -- nt3 wh_turn` verts ; clippy propre. Godot : `wr_turn_ui_test`, `nt3_missions_test`, `wh_turn_test` OK.

## Prochaine étape
Suite complète `cargo test` des 4 crates, lots.md si présent, rapport.
