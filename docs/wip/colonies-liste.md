# Liste des colonies (touche B) — suivi

Spec : `docs/superpowers/specs/2026-09-27-liste-colonies-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-liste-colonies.md`.

## Lot L1 — Cœur et pont (`feat/holdings-core`)

État : squelette committé (structures + `holdings_overview` en `todo!()`, module
`sim-campaign::holdings`, 7 tests `#[ignore]` dans `sim-campaign/tests/hl1_holdings.rs`).

Prochaine étape : implémenter `holdings_overview` en réutilisant `settlement_tax`,
`buildable`, `population::weighted_unrest`, `settlement_rules.garrison_cap` et
`province_settlements` (voir plan § L1.2), puis activer les 7 tests, puis écrire
le pont `godot-bridge/src/campaign_sim_holdings.rs` (`get_holdings_overview`), puis
vérifier (`cargo fmt`, `clippy -D warnings`, `cargo test`, `core/build.sh`,
`smoke.gd`) avant le commit final `HL1: ...`.

Cible cargo privée à ce worktree :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/.claude/worktrees/agent-a22cda46a4d584987/core/target`.
