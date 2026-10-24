# MEM3 : mémoire du client en campagne et en bataille (ADR 0786)

Branche `worktree-agent-a0377ad7d1eea36f9`, base main 62aa11193. Suite de MEM2 (ADR 0776).

## État
- Squelette : `tools/bevy_bench/mem_protocol.sh` (protocole), `footprint_parse.py`. Mesures à faire.

## Prochaine étape
1. Baseline (binaire HEAD) : idle x3, bench campagne x3, bataille x3 -> `out/mem/<label>/`.
2. Élargir le dump `memory` (caches de dalles, wgpu, ressources de campagne).
3. Réductions (KTX2 sol, bornes de caches, décharge campagne<->bataille, mimalloc purge, cause IOAccelerator).
4. ADR 0786, `perf_thresholds.json`, `docs/performance-memoire.md`.

## Points ouverts
- Disque ~16 Go : un seul build release (target cloné par `cp -c` depuis le checkout principal), `rm -rf client/target`
  et assets copiés à la fin.
- Le garde de la machine : `heavy.sh` plein, les builds attendent un emplacement.
