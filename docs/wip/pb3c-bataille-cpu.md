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
- Rust (bridge) : `get_soldier_buffers(capacities)` → `[counts, buffers]` indexés comme
  `get_units()`, cache `PoseCache` clé (ticks, `pose_epoch`, taille d'unité) ; `touch_poses()`
  dans setup, setup_historical, drive, issue_command, begin_deployment, deploy_unit,
  start_battle, debug_*, begin_playback, replay_seek. `get_soldier_buffer` gardé (non caché).
- GDScript : `_update_batched` (repli `_update_per_kind` : `--no-pb3c` ou simulation factice),
  `_drawn[id]` = effectif dessiné (tampon complété), `_param`/`_sent` (uniformes au changement,
  coupés par `--no-pb3c`), `by_id` paresseux dans battle_effects.
- `get_units`/`get_siege` une fois par image : repris **tel quel** du commit bb251b68 de
  fix/code-review (`_frame_siege`, `music.update(delta, units)`) pour fusion sans conflit ;
  battle_staging lit aussi `_frame_siege`.
- Banc : `process_ms_median` (TIME_PROCESS) et `soldiers_ms_median` (durée de soldiers.update).
- Test `game/tests/pb3c_buffers_test.gd` : OK.
- Constat : en Godot 4, les Packed*Array sont partagés par référence en GDScript (vérifié) :
  `_hide_knocked` / `_drive_in` ne copiaient pas ; seule `_hide_reserved` copie (gardée).

## Mesures
Script d'A/B (scratchpad) : `--benchmark --quality=high --disable-vsync 1600x900`, grosse
bataille `--units=120 --bench-at=60` (28 760 soldats), siège `--siege --bench-at=60`.
Premier passage debug (3 paires) : grosse bataille 23,9 vs 21,4 i/s (médianes), siège très
bruité (machine partagée ; l'ordre des passages domine).

## Prochaine étape
A/B debug avec soldiers_ms, puis release ; tests Godot (battle, ep*, siege*, smoke) ; fusion main.
