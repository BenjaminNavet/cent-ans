# Liste des colonies (touche B) — suivi

Spec : `docs/superpowers/specs/2026-09-27-liste-colonies-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-liste-colonies.md`.

## Lot L1 — Cœur et pont (`feat/holdings-core`)

État : `holdings_overview` implémentée dans `sim-campaign/src/holdings.rs`,
7 tests `sim-campaign/tests/hl1_holdings.rs` verts (plus de `#[ignore]`).
Note : le champ `is_upgrade` de la spec § 1 (SettlementRow) n'est pas dupliqué
hors de `options_available[].is_upgrade` — `upgrade_available` suffit (voir
rapport final).

Prochaine étape : écrire le pont `godot-bridge/src/campaign_sim_holdings.rs`
(`get_holdings_overview`), puis vérifier (`cargo fmt`, `clippy -D warnings`,
`cargo test`, `core/build.sh`, `smoke.gd`) avant le commit final `HL1: ...`.

Cible cargo privée à ce worktree :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/.claude/worktrees/agent-a22cda46a4d584987/core/target`.
