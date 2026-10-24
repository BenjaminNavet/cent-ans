# TERRAIN2 : patchwork de rectangles du sol de campagne (ADR 0787)

Mandat 24/10 : constats 2, 3 et « Ressenti » de `out/juge/iter-04-art.md` (sol en grands rectangles aux bords nets, verts contrastés, campagne plus terne que la bataille).
Branche `worktree-agent-a5b8a4d63f0744bce` (base main 62aa11193).

## État
- Squelette : note seule. Cause non encore établie.
## Plan
1. Build release, captures contrôlées d = 25/60/200 (Paris 2.35,48.85 et côte), `CENT_ANS_WEATHER=clear`.
2. Bissection `CENT_ANS_PERF_OFF` (tnogdk, tnogdmacro, tnofields, tnofp…) puis cache de sol par nœud (ADR 0742), masques de biome.
3. Correction (données/shader, sans nouveau sampler), mesure A/B `tools/bevy_bench/ab.sh`, ADR 0787.
## Captures utilisées : 0 / 10
