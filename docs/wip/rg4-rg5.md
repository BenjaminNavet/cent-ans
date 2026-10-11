# RG4 + RG5 — annulation au prorata, « constructible », bâtir depuis la vue province

Branche : worktree `agent-acaa6c33b06885da6` (ne pas fusionner dans main sans relecture). Source : `docs/wip/rg-retours-gestion.md` (hors dépôt à la création).

## RG4 (cœur)
- `Construction.total_turns` (durée fixée à l'ordre, `#[serde(default)]`, repli sur le temps de base du bâtiment).
- `CampaignState::cancel_build_quote` / `cancel_queued_build_quote` → `CancelQuote {paid, refund, elapsed, total}` ; remboursement = payé × (total − écoulé) / total ; tout le prix si écoulé ≤ `cancel_build_full_refund_turns` (0 = le tour même) ; plancher `cancel_build_min_refund_percent` ; file non commencée : `cancel_queued_build_refund_percent` (100).
- Paramètres dans `data/rules/economy.json` + `data/schemas/economy_rules.schema.json` + `EconomyRules` ; `CANCEL_REFUND_PERCENT` supprimé.
- Pont : `settlement_cancel_quote(id, index)` (-1 = chantier en cours).
- UI : `SettlementController.request_cancel_build` → `ConfirmPanel` « Annuler le chantier ? Vous récupérerez X ₶ sur Y ₶ investis (Z tours écoulés sur N). » ; branché sur panneau de colonie, file, panneau de province (via `campaign_map._on_cancel_build`) et vues d'accompagnement.

## RG5
- Cœur : `CampaignState::can_build` / `can_build_from_options` → `CanBuild {ok, reason}` (emplacement libre ou promotion, file non pleine, au moins un bâtiment constructible ; or exclu). Motifs : « Aucun emplacement libre », « File de chantiers pleine », « Aucun bâtiment constructible », « Colonie occupée ». `SettlementRow.can_build/can_build_reason`, `ProvinceRow.buildable_count`.
- Pont : `get_holdings_overview` (champs ci-dessus), `settlement_can_build(id)`.
- UI : `BuildHammer` (icône `glyph_hammer`, grisé + motif en infobulle), `BuildCompanion` (sélecteur + `SettlementBuildingsView` réutilisée, bouton Annuler sur les cartes en chantier), panneau de province (onglet Colonies : marteaux, « constructibles : n », vue des bâtiments sous la liste), fenêtre Colonies (marteau dans la colonne Ouvrage, vue ouverte à côté).
- Tests : `core/crates/sim-campaign/tests/economy/rg4_rg5.rs`, `game/tests/rg5_build_here_test.gd`.

## État / suite
Voir le dernier commit `wip:`. Reste à la clôture : vérification Godot (import, smoke, tests), pas de capture.
