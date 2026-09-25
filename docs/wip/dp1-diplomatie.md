# DP1 — Diplomatie à la Total War (négociation, buts de guerre, écran plein)

Branche : `worktree-agent-a4e14689f1209a2c9`. ADR : `docs/decisions/0025-negociation-et-buts-de-guerre.md`.

## Reprise
- Cœur : `core/crates/sim-campaign/src/negotiation.rs` (articles, évaluation, contre-proposition,
  buts de guerre, fatigue, paix de l'IA). Branchements : `diplomacy.rs` (`Proposal::Treaty`,
  `war_score`, `resolve_diplomacy`, `plan_diplomacy` : paix par traité, fatigue, prétendant),
  `orders.rs` (`ProposeTreaty`), `state.rs` (`FactionState.ledger`, `is_friendly_territory`),
  `religion.rs` (`political_unrest` + fatigue).
- IA des accords : `core/crates/ai/src/diplomacy_eval.rs` (une ligne dans `campaign.rs::plan_turn`).
- Pont : `core/crates/godot-bridge/src/campaign_sim_treaty.rs` (`evaluate_treaty`,
  `counter_treaty`, `treaty_options`, `get_treaty_history`, `get_war_summary`) ; `get_diplomacy`
  gagne `ruler`, `weariness`, `trade_agreement`, `access_given`, `access_received`.
- Réglages : `data/ai/diplomacy.json` § `negotiation` (schéma `ai_diplomacy.schema.json`).
- UI : `game/scripts/ui/diplomacy_panel.gd` (écran plein, même classe), `diplomacy_controller.gd`
  (teinte et légende des modes de carte, C11/U14), étape de capture `diplomacy_treaty` dans
  `campaign_map.gd`. `map_ui.gd` n'est pas touché.
- Sonde dédiée : `cargo run --release -p ai --example dp1_probe -- 300 2` (traités FR-EN et
  raisons pour lesquelles l'Angleterre ne déclare pas la guerre).

## État
- [x] Cœur, tests `sim-campaign/tests/dp1_negotiation.rs` (18 + 1 ignoré d'affichage).
- [x] Sonde avant/après (ci-dessous).
- [x] IA des accords, pont, écran plein, carte diplomatique lisible, smoke (étape diplomatie
  étendue), captures `docs/audit/captures/dp1/`.

## Sonde
`century_probe` (5 graines × 464 tours) :
- Avant (main + G1) : guerre FR-EN moy. 36 % [30-42] (30/38/42/39/32), majeures en 1400 : 5/5.
- Après : **61 % [43-68]** (43/66/68/61/65), 4/5 graines dans 55-75 %, majeures en 1400 : 5/5.

`balance_probe campaign 200 1-8` :
- Avant : guerre FR-EN 33 %, 6,9 changements de propriétaire, mécontentement 8,3, 11,5 révoltes.
- Après : guerre FR-EN 54 %, **13,9** changements de propriétaire, mécontentement 12,0,
  21,1 révoltes, 104 paix et 119 déclarations de guerre par partie (avant 77 et 88).

## Points ouverts
- Graine 1 du siècle à 43 % : l'Angleterre y manque d'argent (voir `dp1_probe`).
- L'accès militaire ne joue que sur le ravitaillement (pas de règle d'intrusion en paix).
- Accord commercial autonome : à relier à C5 (`integration/tw`) quand il sera sur main.
- La minicarte du HUD reste en couleurs politiques dans le mode diplomatique.

## Prochaine étape
Fusionné avec main ? voir le rapport final. Suite possible : IA qui propose au joueur des traités
d'amitié plus riches (mariage + alliance), coalitions (N8) sur la même évaluation.
