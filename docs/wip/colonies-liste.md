# Liste des colonies (touche B) — suivi

Spec : `docs/superpowers/specs/2026-09-27-liste-colonies-design.md`.
Plan : `docs/superpowers/plans/2026-09-27-liste-colonies.md`.

## Lot L1 — Cœur et pont (`feat/holdings-core`)

État : `holdings_overview` implémentée dans `sim-campaign/src/holdings.rs`,
7 tests `sim-campaign/tests/hl1_holdings.rs` verts (plus de `#[ignore]`).
Note : le champ `is_upgrade` de la spec § 1 (SettlementRow) n'est pas dupliqué
hors de `options_available[].is_upgrade` — `upgrade_available` suffit (voir
rapport final).

## Lot L1 — TERMINÉ (2026-09-27)

`feat/holdings-core` : cœur (`sim-campaign::holdings::holdings_overview`, 7 tests
verts) et pont (`godot-bridge::campaign_sim_holdings::get_holdings_overview`)
livrés et vérifiés : `cargo fmt`, `cargo clippy --all-targets -D warnings`,
`cargo test --workspace` (135 suites, 0 échec), `core/build.sh`,
`godot --headless --path game --import` puis `smoke.gd` (tout `smoke OK`,
exit 0). Non fusionné dans `main` (fait par la session de coordination).

Écart mineur par rapport à la lecture littérale de la spec § 1 : le champ
`is_upgrade` listé juste après `options_available` dans `SettlementRow` n'a
pas été dupliqué en dehors de `options_available[].is_upgrade` — la lecture
retenue est que ce texte documente le champ interne des options, et
`upgrade_available` (déjà spécifié) couvre le besoin de l'IU.

## Lot L2 — Panneau Godot (`feat/holdings-ui`)

Pas commencé. Dépend de l'API du pont livrée ci-dessus (`get_holdings_overview`).
À faire sur une branche/worktree séparée, voir plan § L2.

Cible cargo privée à ce worktree :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/.claude/worktrees/agent-a22cda46a4d584987/core/target`.
