# PERF-COMPTEURS (lot L-A du juge technique, 30/10/2026)

Branche : worktree-agent-ab8b1b69b40c7dd9f. ADR 0827. Audit : docs/audits/2026-10-30-juge-technique-iter3.md (T-1, T-5).

## État
- [x] Squelette : cette note, ADR 0827 (brouillon).
- [ ] `client/src/agent/counters.rs` : compteurs de travail (appels de dessin, triangles, instances, pipelines, CPU process et fil principal, passes).
- [ ] Banc carte et bataille : bloc `work` par phase ; `memory` étendu (tas mimalloc, Metal).
- [ ] `perf_thresholds.json` v2, `perf_guard.sh` sans `max_load`, `full_check.sh`.
- [ ] Mesures de référence (3 passes) et README / `docs/performance-memoire.md`.

## Prochaine étape
Écrire counters.rs, brancher dans bench.rs / bench_battle.rs.

## Points ouverts
- Octets uploadés par image : pas de crochet sans patcher wgpu ; non couvert.
