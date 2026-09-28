# RS-N — bouton « Raser » (UI)

Branche `feat/rs-n-raze`, worktree d'agent. Continue RS-C (ADR 0111,
`docs/wip/rs-c-diplo.md`) : le cœur a déjà `Order::Demolish`, le pont accepte
`{"type": "demolish", "settlement": ..., "building": ...}`, remboursement
`economy.json` `demolition_refund_percent`, refus sous siège / hors colonie du
joueur / dépendance (`buildings::demolition_blocker`). Aucune UI n'existait.

## Plan
1. Cœur : `buildings::demolition_preview` (aperçu en lecture : `can_demolish`,
   `reason` FR, `refund`, `upkeep_saved`), réutilisé par `demolish()` pour ne
   pas dupliquer le calcul du remboursement. Tests Rust dans `rs_n_tests.rs`.
2. Pont : `CampaignSim.settlement_demolition_preview(id) -> Array` (une entrée
   par bâtiment construit de la colonie : `{building, can_demolish, reason,
   refund, upkeep_saved}`), fonction de lecture pure ajoutée à
   `campaign_sim_settlements.rs`.
3. Godot : bouton « Raser » par bâtiment dans l'onglet Bâtiments de
   `settlement_panel.gd` (via `panel_widgets.gd`), désactivé + raison FR quand
   le cœur refuse, infobulle simple (`tooltip_text`, pas `rich_tooltip.gd`)
   donnant remboursement et entretien économisé. Confirmation par une fenêtre
   modale du projet (nouvelle `RazeConfirmationDialog`, calquée sur
   `war_declaration_dialog.gd`). Ordre envoyé par `SettlementController`
   (`map._submit({"type": "demolish", ...})`).
4. Test d'écran `game/tests/rs_n_raze_test.gd` : panneau d'une colonie du
   joueur, bouton présent, désactivé sous siège/dépendance, ordre envoyé.
5. Codex : une phrase dans `cdx_jeu_construction`.

## État
- [x] Skeleton commité (ce fichier).
- [x] Cœur : `buildings::demolition_preview`/`DemolitionPreview`, `demolish()`
      réutilise le même calcul de remboursement. 4 tests `rs_n_tests.rs`
      (autorisé, bloqué par dépendance, bloqué par siège, aperçu = réel).
- [x] Pont : `CampaignSim.settlement_demolition_preview(id)` (lecture pure,
      `campaign_sim_settlements.rs` + `demolition_preview_array`/`_dict` dans
      `campaign_sim.rs`).
- [x] Godot : `PanelWidgets.fill_buildings` prend `demolition`/`is_player_owner`/
      `on_raze` (rétrocompatible, `province_panel.gd` inchangé) et ajoute un
      bouton « Raser » par bâtiment (icône `act_cancel_build`, `tooltip_text`
      simple : remboursement + entretien économisé si possible, raison FR du
      cœur sinon). `SettlementPanel.raze_requested` → `SettlementController`
      ouvre `RazeConfirmationDialog` (calqué sur `WarDeclarationDialog`,
      `UiZones.Zone.MODAL`) avant l'ordre `demolish`.
- [x] Codex : phrase ajoutée dans `cdx_jeu_construction` (bouton « Raser »).
- [x] `game/tests/rs_n_raze_test.gd` : OK (Amiens, marché bloqué par la
      draperie qui en dépend, draperie rasée après confirmation, remboursement
      versé, renoncer ne change rien).
- [x] cargo fmt/clippy `--all-targets -D warnings` : verts.
- [ ] `cargo test --workspace` : en cours (long, tourne en tâche de fond).
- [ ] `smoke.gd` : en cours (tâche de fond).
- [ ] `po_ui_test.gd`, `c5_settlements_ui_test.gd` (régression) : à lancer.
- [ ] Supprimer `core/target-rs-n` en fin de lot.

## Prochaine étape
Attendre `cargo test --workspace` et `smoke.gd`, lancer `po_ui_test.gd` et
`c5_settlements_ui_test.gd`, puis commit final et rapport.
