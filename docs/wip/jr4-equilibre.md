# JR4 — IA et équilibrage de la faction croisée

Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`, note d'orchestration
`docs/wip/jr-croises.md`. Worktree `../gp-jr` (`feat/jr`), en parallèle de JR3 (`game/`).

## État
- Sonde `core/crates/ai/tests/jr_crusade_probe.rs` (`#[ignore]`, 5 graines × 50 tours, ~50 s) :
  `cargo test -p ai --test jr_crusade_probe -- --ignored --nocapture` ; `JR_SEEDS=1,4`,
  `JR_TURNS=60`, `JR_TRACE=1`.
- Référence avant correctif (barème JR1) : graines 1, 4, 5 → ferveur 0 dès le tour 30, 2-3 unités,
  aucune place ; graines 2, 3 → ferveur 99, 6-7 places tenues. Débarquement au tour 1 partout.

## Prochaine étape
- Diagnostiquer les graines 1/4/5 (trace), puis corrections de règle (attaquant seul pénalisé,
  évènements monde, crochet naval), puis barème.

## Points ouverts
- (à compléter)
