# WIP — godot-bridge : API `CampaignSim` (M2, spec § 2)

## État
- Part A terminée (commit 142d659) : `GameDataStore.load_heightmap_u16/load_mask_u8/load_rgb8/get_last_image_size/get_province_owner_colors`.
- Part B en cours : `core/crates/godot-bridge/src/convert.rs` écrit (Variant ↔ `serde_json::Value`), `serde_json` ajouté aux dépendances.
- Bloqué : `core/crates/sim-campaign` ne compile pas (modules déclarés dans `lib.rs`, fichiers absents) — un autre agent l'implémente. Ne pas corriger leur crate ; attendre.

## Prochaine étape
1. Quand `cargo build -p sim-campaign` passe : réécrire `CampaignSim` dans `lib.rs` avec les méthodes de la spec § 2 (`new_campaign(data_dir, player, seed) -> bool`, `save_to_string`, `load_from_string`, getters, `submit_order(Dictionary)`, `end_turn() -> Array[Dictionary]`, `get_events`). Dictionnaires construits explicitement avec exactement les clés de la spec ; imbriqués passés par référence dans `vdict!`.
2. Écrire `core/checks/campaign_sim_check.gd` (France, armées, reachable, move_army, 4 fins de tour, save/load, dates égales).
3. `cargo fmt`, `clippy -D warnings`, `cargo test`, `core/build.sh`, checks headless.
4. Mettre à jour `docs/design/data-model.md` § 7 (7.4 `CampaignSim`), supprimer ce fichier, commit final.
