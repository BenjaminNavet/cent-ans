# DP1 — Diplomatie à la Total War (négociation, buts de guerre, écran plein)

Branche : `worktree-agent-a4e14689f1209a2c9`. ADR : `docs/decisions/0025-negociation-et-buts-de-guerre.md`.

## Reprise
- Cœur : `core/crates/sim-campaign/src/negotiation.rs` (articles, évaluation, contre-proposition,
  buts de guerre, fatigue, paix de l'IA). Branchements : `diplomacy.rs` (`Proposal::Treaty`,
  `war_score`, `resolve_diplomacy`, `plan_diplomacy`), `orders.rs` (`ProposeTreaty`), `state.rs`
  (`FactionState.ledger`), `religion.rs` (`political_unrest` + fatigue).
- Réglages : `data/ai/diplomacy.json` § `negotiation` (schéma `ai_diplomacy.schema.json`).

## État
- [x] Squelette + première implémentation du cœur (compile, clippy vert).
- [ ] Tests Rust (`tests/dp1_negotiation.rs`).
- [ ] Sonde avant/après.
- [ ] IA : `ai/src/diplomacy_eval.rs` (accords commerciaux, accès militaire).
- [ ] Pont : `evaluate_treaty`, `counter_treaty`, `treaty_options`, `get_treaty_history`.
- [ ] Écran plein Godot (`diplomacy_panel.gd` refait), captures `docs/audit/captures/dp1/`.

## Sonde (century_probe, 5 graines × 464 tours)
- Avant : guerre FR-EN moy. 41 % [33-50] (44/43/33/34/50), 4 majeures en 1400 : 5/5.

## Prochaine étape
Tests Rust, puis sonde.
