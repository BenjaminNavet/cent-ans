# PB3f — rayon : fin de tour parallèle, déterminisme gardé

Branche `perf/pb3f-rayon` (worktree agent `agent-a27afc73c1888787a`). Plan général :
`docs/wip/pb3-performance.md`. ADR : `docs/decisions/0091-parallelisme-de-la-fin-de-tour.md`.

## État
- [x] Profil release (graine 1, 20 tours, `sample` macOS) : planificateur ≈ 80 % du cœur,
  phases `resolve_*` ≈ 13 ms (détail dans l'ADR).
- [x] `ai::parallel` : pool rayon dédié (10 fils = cœurs performants, QoS USER_INITIATED,
  pile 64 Mio), `Mode::{Sequential, Parallel}`.
- [x] `plan_turn` parallèle / `plan_turn_sequential` : contexte ‖ planificateurs d'État,
  options de recrutement et de construction par colonie, tables de routes pré-calculées,
  `GridPlanner::with_mode`. Calculs répétés supprimés : `free_supply` une fois par faction,
  vitesse de construction une fois par colonie, revenu brut une fois.
- [x] Test `ai/tests/pb3f_parallel_plan.rs` (3 graines × 12 tours, ordres + événements + état).
- [x] Exemple `turn_digest [--sequential]` : empreintes identiques à la base a7877ac6
  (3 graines × 30 tours, séquentiel et parallèle, trois exécutions).
- [x] Mesures cœur et pb1_turns release (ci-dessous).
- [x] ADR 0091. [x] fusion de `main` (d246c5c3) ; cargo fmt/clippy -D warnings/test (865 tests)
  verts ; smoke, cv1, c5, fr1, ct1_ai_turn verts (dylib debug de la branche, après import).

## Mesures (26/09, M4 Pro très chargée — charge moyenne 15-37 —, A/B alternés, médianes de 3)
`turn_digest 30 1 7 1337` (fin de tour complète, 90 tours, médiane) :

| exécution | base a7877ac6 | branche séquentielle | branche parallèle |
|---|---|---|---|
| 1 | 105,3 ms | 120,6 ms | 87,0 ms |
| 2 | 108,0 ms | 84,9 ms | 69,3 ms |
| 3 | 118,6 ms | 86,6 ms | 62,3 ms |
| médiane | **108,0** | **86,6** (−20 %) | **69,3** (−36 %) |

Série précédente moins chargée (sans les calculs répétés supprimés) : base 94-99 ms → 60-62 ms.

`turn_perf 20 3 1` (temps par faction, meilleur de 3) : moyenne base 2,87 / 2,80 / 3,27 ms,
branche 1,81 / 1,46 / 4,92 ms (3e exécution perturbée par la charge) → médiane 2,87 → 1,81 ms
(−37 %) ; p95 9,6 → 5,4 ms.

`pb1_turns.gd` fenêtré, dylib release installée sous le nom debug, base = a7877ac6 (pont
identique à `main`), médiane des tours 1-5 de chaque exécution :

| exécution | base | branche |
|---|---|---|
| 1 | 173,6 ms | 129,4 ms |
| 2 | 162,8 ms | 281,8 ms (pic de charge de la machine) |
| 3 | 166,5 ms | 130,2 ms |
| médiane | **166,5** | **130,2** (−22 % du total fin de tour → carte rafraîchie) |

Pire image inchangée (≈ 65-110 ms : rafraîchissement GDScript du fil principal, hors lot).

## Outils
- `CARGO_TARGET_DIR=<worktree>/core/target cargo run --release -p ai --example turn_digest -- [--sequential] 30 1 7 1337`
- Scripts A/B dans le scratchpad de session (`ab.sh`, `tp.sh`), base construite dans un worktree
  détaché temporaire du scratchpad (à supprimer : `git worktree remove`).

## Prochaine étape
Lot terminé, prêt à fusionner (ff-only par l'orchestrateur). Pistes : phases `resolve_*`
(≈ 13 ms, RNG partagé : ne paralléliser qu'avec un flux par province qui garde les résultats),
boucle séquentielle des armées, rafraîchissement GDScript (pire image).
