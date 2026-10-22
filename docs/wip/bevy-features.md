# Exploiter Bevy 0.20 (branche `perf/bevy-par-iter`)

Statut : plan, rien d'implémenté. Worktree `../gp-bevy-perf`. Mandat utilisateur (22/10) : tout faire, chaque feature mesurée au banc, désactivée par défaut si elle coûte trop de fps.

## Contexte mesuré (session KTX2, `docs/wip/ktx2-assets.md`)
- Crécy, mêlée : p50 48 ms, ~21 fps, 65 514 entités max, empreinte 6,5 Go, pas de swap. Le goulot est le temps d'image CPU/GPU, pas la VRAM.

## Coordination
- KTX2 / `processed-assets` / `asset_processing.rs` / textures : session 56 (propriétaire). Ne pas y toucher ; `dds` et `basis-universal` lui reviennent.
- Ombres d'impostors (lot B5) : session f5, fichiers de rendu des impostors. Rebaser sur main avant fusion.
- Un seul banc release à la fois sur la machine (sinon mesures faussées). Banc de la session 56 d'abord.

## Lots (ordre)
1. Profilage par système : `BENCH_FEATURES=profile tools/bevy_bench/bench.sh perf_battle --bench-battle` (arrêté, à relancer).
2. `par_iter` sur les systèmes chauds de la bataille (cible : le haut de `by_total`).
3. Rendu : `pbr_clustered_decals` (traces/sang), `pbr_*_textures`, `experimental_pbr_pcss`, post-effets. Chaque ajout mesuré.
4. `meshlet` + `meshlet_processor` : essai sur la végétation.
5. Dev : `hotpatching`, `bevy_dev_tools`, `render_dev_tools`, `bevy_debug_stepping` dans la feature `dev` seulement.
6. `serialize`/`reflect_*`, `bevy_settings`, `ghost_nodes`, `bevy_feathers`, caméras Bevy (comparaison), `bevy_solari` (feature désactivée par défaut, repli si GPU sans raytracing).

## Prochaine étape
Relancer le lot 1 dès que la session 56 a fini son banc campagne `mem-map`.
