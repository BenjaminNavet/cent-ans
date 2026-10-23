# MEM2 : régression de mémoire en campagne (terminé, voir ADR 0776)

## État
Fait et mesuré ; ADR 0776, `docs/performance-memoire.md` (amendement), garde `perf_guard.sh` / `perf_thresholds.json`
mis à jour. Reste à l'intégrateur : clippy complet, `cargo test --features dev`, smoke.sh.

## Cause
66 textures 2048 RGBA8 des modèles `models/dn/*` (GPU 1,3 Go + copie CPU 1,3 Go) + ~1 Go de tampons de décodage
retenus par libmalloc. Aucune des 10 fusions récentes en cause (bissection de build inutile : le dump
`textures.by_prefix` suffit). Piège de mesure : un worktree n'a pas les assets non suivis (`rsync -a --ignore-existing`
de `game/assets` et `out/ktx2_cache` depuis le checkout principal).

## Corrections
`fx/model_mips.rs` (capture au `Added`, 1024 px, BC1, GPU seul, dn + props_ga) ; `RENDER_WORLD` après conversion
(stone, atlas, flipbooks) ; mimalloc global (feature `system-alloc` pour A/B) ; `memory.footprint_mib` ;
`CENT_ANS_RELIEF=<s>` (expérience, rend 0 Mo).

## Mesures (Mo, campagne)
Footprint 6 550-6 760 -> 2 750-2 820 ; pic RSS ~4 900 -> 3 600 ; images CPU 1 537 -> ~180, GPU 1 916 -> ~690.
fps : pas de régression (A/B alternés).

## Points ouverts
- Bataille : IOAccelerator (graphics) oscille 0,4 <-> 2,3 Go, plancher 3,8 Go > 3,5 (cause non trouvée).
- Décharge campagne -> bataille non mesurée ; KTX2 de sol de campagne non fait.
