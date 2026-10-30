# PERF-COMPTEURS (lot L-A du juge technique, 30/10/2026)

Branche : worktree-agent-ab8b1b69b40c7dd9f. ADR 0827. Audit : docs/audits/2026-10-30-juge-technique-iter3.md (T-1, T-5).

## État : fait, à fusionner
- [x] `client/src/agent/counters.rs` : compteurs de travail (appels de dessin via la feature `perf-counters`, triangles, instances, pipelines, CPU fil principal et processus, passes) ; bloc `work` par phase dans les rapports carte et bataille, section `work` du dump.
- [x] `memory.rs` : rapport étendu (Metal, tas mimalloc, hors tas) ; `profile.rs` : CPU du fil par système.
- [x] `perf_thresholds.json` v2 (calibré sur 4 exécutions), `perf_guard.sh` sans `max_load`, `perf_check.py`, `perf_calibrate.py`, `full_check.sh`.
- [x] Mesures : 3 exécutions carte (d = 30, pan + 3 descentes) et 3 bataille (Crécy) + 1 exécution de garde chacune.
- [x] Garde éprouvé : `CENT_ANS_MAP_CASCADES=8` fait échouer draws et triangles (+15 %) ; sans, il passe sous charge 40.
- À faire à la fusion (hors lot) : clippy complet, `cargo test`, smoke.

## Valeurs de référence (médianes, build release `perf-counters`, Apple M4, préréglage auto)
| Phase | draws p50 / p95 | triangles p50 | pipelines créés | CPU fil principal p50 | CPU processus p50 (indicatif) |
|---|---|---|---|---|---|
| carte pan d = 30 | 430 / 454 | 1,69 M | 11 | 2,1 ms | 34,5 ms |
| carte descente | 423 / 476 | 1,68 M | 1 | 2,2 ms | 34,6 ms |
| bataille calme | 294,5 / 294,5 | 0,56 M | 0 | 1,6 ms | 28,1 ms |
| bataille mêlée | 441,5 / 444,5 | 6,34 M | 6 | 1,9 ms | 33,8 ms |
Mémoire (fin de banc) : carte empreinte 6 942 Mo, pic RSS 5 016 ; bataille 3 494 / 2 819.

## Écarts avec la spec
- Seuil CPU = CPU du fil principal (±10 % entre charges) ; le CPU du processus monte de ~30 % machine saturée (fils de travail qui tournent à vide) : indicatif seulement.
- Pas de crochet pour les octets envoyés au GPU par image (wgpu) ; draws indirects comptés pour un appel chacun.
- Mesure de deux charges 5 et 30 non contrôlée : la machine était à 30-46 en permanence ; les 4 exécutions donnent draws ±1 %, CPU fil principal ±10 %.
- Les lignes des événements `draw*` de bevy_render 0.20.0 sont une constante (`DRAW_LINES`) ; `=0.20.0` est épinglé.
- Un bench peut être tué par le `pkill` du chien de garde d'un autre bench en parallèle (vu une fois) : relancer.
