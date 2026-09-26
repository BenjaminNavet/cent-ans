# PB3c — CPU par image en bataille

Branche `perf/pb3c-battle-cpu` (worktree d'agent). Plan : `docs/wip/pb3-performance.md`.
Objectif : moins de CPU par image en bataille, rendu identique, simulation inchangée.

## Plan
1. Rust : cache des poses des figurines (clé : pas de sim, époque des mutations hors pas,
   taille d'unité) ; `get_soldier_buffers(capacities)` groupé, tampons par régiment déjà
   complétés à la capacité du MultiMesh (plus de `slice` ni `duplicate()+resize`).
2. GDScript : un seul `get_units` par image (musique après `_refresh_view`), `by_id` mutualisé.
3. `get_siege` : un appel par image, mis en cache pour l'image.
4. `set_shader_parameter` seulement au changement ; `_find_braced` seulement si charge.

## État
- Squelette, liens symboliques, build debug en cours.

## Mesures
(à venir)

## Prochaine étape
Mesure de référence (banc `--benchmark`), puis implémentation du point 1.
