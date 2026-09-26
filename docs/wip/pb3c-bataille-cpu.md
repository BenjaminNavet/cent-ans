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
- GDScript commité (f18992d9) : `_update_batched` (repli `_update_per_kind` si `--no-pb3c` ou
  simulation factice), `_drawn[id]` = effectif dessiné (le tampon est complété), `_param` /
  `_sent` (uniformes au changement), `current_siege()` (cache par image + instant simulé),
  musique après `_refresh_view` avec les `units` de l'image, `by_id` paresseux.
- Rust écrit (`get_soldier_buffers`, `PoseCache`, `touch_poses` dans toutes les mutations hors
  pas), build en attente du verrou du `target` partagé.
- Test `game/tests/pb3c_buffers_test.gd` (groupé == par famille, déploiement, ordre, entre pas).
- Constat : en Godot 4, les Packed*Array sont partagés par référence en GDScript (vérifié) :
  `_hide_knocked` / `_drive_in` ne copiaient pas ; seule `_hide_reserved` copie (gardée).

## Mesures
(à venir)

## Prochaine étape
Mesure de référence (banc `--benchmark`), puis implémentation du point 1.
