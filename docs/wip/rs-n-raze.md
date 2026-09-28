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
- [ ] Pont : `settlement_demolition_preview`.
- [ ] Godot : bouton, dialogue, wiring, tests.
- [ ] Codex.
- [ ] fmt/clippy/cargo test, smoke.gd, po_ui_test.gd.

## Prochaine étape
Implémenter `demolition_preview` dans `core/crates/sim-campaign/src/buildings.rs`.
