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

## Trouvailles du profilage (grosse bataille, release, par image)
Avant : `_update_effects` ~16 ms dont `battle_effects.update` ~15 ms : `_wet_span` appelle
`terrain.in_water` 5 fois par régiment en marche, qui parcourait tout le tracé des ruisseaux
(points tous les 4 m) en GDScript. Corrigé : tronçons de 24 segments avec boîte englobante
(`_stream_chunks`, résultat identique) + `_wet_span` en cache par régiment tant que position,
cap et profondeur ne changent pas. Ensuite : `get_units` ~2,4 ms (désormais en cache Rust entre
deux pas), `soldiers.update` ~3,3 ms, étendards ~1,7, audio ~1,3, herbe couchée ~1,2.
Les poses Rust ne coûtaient presque rien en release : le cache des poses aide surtout en debug.

## Piège de build
Le `target` partagé (`core/target` du dépôt principal) mélange les worktrees : les crates du
workspace ont les mêmes empreintes d'un worktree à l'autre, la dylib copiée peut venir d'un
autre agent (constaté : dylib sans `get_soldier_buffers`). PB3c compile dans le `core/target`
de son worktree (ignoré par git).

## Mesures (A/B base 6a837e99 contre la branche, scripts + dylib échangés, ordre alterné, 3 paires)
`--benchmark --quality=high --disable-vsync 1600x900` ; grosse bataille `--units=120
--bench-at=60` (28 770 soldats, 240 régiments) ; siège `--siege --bench-at=60` (967 soldats).
Médianes des 3 passages ; machine partagée très chargée (bruit ±30 %).

| Cas | i/s base → PB3c | image médiane (ms) | TIME_PROCESS médian (ms) |
|---|---|---|---|
| Grosse bataille, release | 25,1 → 40,4 | 38,3 → 25,0 | 66,4 → 47,1 |
| Grosse bataille, debug | 24,8 → 51,9 | 38,9 → 17,6 | 68,4 → 47,3 |
| Siège, release | 49,3 → 58,4 | 20,0 → 16,7 | 28,7 → 17,3 |
| Siège, debug (7 paires) | 55,4 → 52,6 | 18,1 → 18,5 | 22,8 → 27,2 (bruit) |

Profil du siège (branche) : scripts de bataille ~2,1 ms/image en tout ; le reste du temps
d'image vient d'ailleurs (rendu), d'où un A/B siège dominé par le bruit.
`soldiers.update` : ~3,3 ms (grosse bataille) des deux côtés en release ; le cache des poses ne
gagne rien de mesurable en release (poses Rust déjà bon marché), un peu en debug.

## Tests
`cargo fmt/clippy -D warnings/test` OK ; Godot : pb3c_buffers, b4_pacing, bv3_check, ep13 (rejeux
identiques), ep2, ep4, ep7, ep8, ep8b, s2_fire_fx, smoke OK. `bv1_check` échoue (« traits dans
les pavois 0/256 », `BattleVolleys.arrow_landing`, fichier non touché : antérieur à PB3c).

## Reste / réserves
- `get_units` et les tampons renvoyés entre deux pas sont partagés : lecture seule côté GDScript.
- `_find_braced` laissé tel quel (déjà O(n) hors charge de cavalerie).
- Pistes suivantes : `soldiers.update` (~3,3 ms, boucles GDScript), étendards 1,7 ms, audio
  1,3 ms, herbe couchée 1,2 ms.

## Prochaine étape
Fusion ff par l'orchestrateur ; conflit attendu avec fix/code-review (bb251b68 repris à
l'identique, 08555828 sans objet avec les tampons complétés côté Rust).
