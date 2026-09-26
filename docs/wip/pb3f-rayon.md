# PB3f — rayon : fin de tour parallèle, déterminisme gardé

Branche `perf/pb3f-rayon` (worktree agent). Plan général : `docs/wip/pb3-performance.md`.
ADR réservé : `docs/decisions/0091-parallelisme-de-la-fin-de-tour.md`.

## État
- [ ] Profil : IA par faction (planificateur vs marches vs ordres) et phases `resolve_*`.
- [ ] Choix des cibles parallélisables (lecture seule, sans RNG partagé).
- [ ] rayon en dépendance de workspace, pool limité aux cœurs performants.
- [ ] Test bit à bit séquentiel/parallèle sur N tours, plusieurs graines.
- [ ] Mesures turn_perf / pb1_turns avant-après (A/B alternés, médianes de 3).
- [ ] ADR 0091.

## Prochaine étape
Profiler (`cargo run --release -p ai --example turn_phases`).
