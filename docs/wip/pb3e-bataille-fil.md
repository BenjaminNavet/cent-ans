# PB3e — pas de bataille dans un fil, piétinement en Rust

Branche `worktree-agent-aacdb076cb5ce2218` (worktree d'agent). Plan : `docs/wip/pb3-performance.md`.
ADR : `docs/decisions/0090-pas-de-bataille-dans-un-fil.md`.
Objectif : supprimer les à-coups (pas de sim toutes les ~6 images) et le CPU restant en
bataille, résultat identique.

## Plan
1. Pas N+1 calculé sur un fil pendant que l'image montre N (`sim-battle/src/sim/pipeline.rs` :
   `tick_with`, `fork_for_step`, `adopt_step` ; pont `battle_step_job.rs` : `StepPipeline`,
   clé `(ticks, pose_epoch)`). `set_step_thread(true)` par `battle_scene.gd` hors headless et
   hors rejeu. Test bit-à-bit `pipelined_battle_matches_synchronous_battle`.
2. Piétinement (`battle_terrain.gd` `update_trample`) en Rust, mise à jour de la zone touchée.
3. soldiers.update / étendards / audio / herbe couchée : ce qui se porte simplement.
4. Banc : p99 et pire image ajoutés au JSON ; A/B release et debug.

## État (reprise 26/09 après pause ; `main` fusionné 4d22e33b)
- [x] 1. Pas N+1 dans un fil : `sim/pipeline.rs` (`tick_with`, `fork_for_step`, `adopt_step`),
  `battle_step_job.rs` (`StepPipeline`), `set_step_thread`/`get_step_stats` ; `battle_scene.gd`
  l'active hors headless, hors rejeu, sans `--no-pb3e`. Tests Rust bit-à-bit bataille + siège.
- [x] 2. Piétinement et herbe couchée : classe Rust `StampMap` (`stamp_map.rs`), mêmes octets que
  les boucles GDScript (vérifié par `pb3e_step_thread_test.gd`), envoi seulement si modifiée.
  Pas de mise à jour partielle de texture dans Godot 4.7.
- [x] 3. `mm.buffer` pas réaffecté si inchangé (versions rendues par `get_soldier_buffers`) ; les
  places réservées (étendards, musiciens) reprennent le tampon masqué précédent sous une version
  dérivée. `--pb3e-verify` : 0 écart sur la grosse bataille et le siège.
- [ ] Étendards (1,7 ms) et audio (1,3 ms) : non traités (pas de portage simple sans changer le
  rendu).
- [x] Banc : `frame_ms_p99`, `frame_ms_max`, `tick_ms_p99/max`, `proc_step_ms_*` (durée de
  `_process` des images avec un pas) / `proc_plain_ms_*`, `step_stats`.
- [x] ADR 0090.
- [x] Mesures A/B (ci-dessous).
- [x] `main` fusionné (257bdddf), réimport Godot, tests OK.

## Constat de mesure
Le pas de sim lui-même est court à la 60e seconde de la grosse bataille (tick p99 ≈ 1,1 ms,
pire ≈ 1,5 ms en release) : les images de pas coûtent surtout par `get_units`, les poses et le
renvoi de tous les tampons (≈ +3-4 ms de `_process` par rapport aux autres images).

## Mesures (26/09, A/B alternés, médianes de 3, machine partagée)
Même binaire ; base = `--no-pb3e` (coupe fil, `StampMap` et saut des tampons : chemins d'avant).
`--benchmark --quality=high --disable-vsync --resolution 1600x900`, `--bench-at=60` ; grosse
bataille `--units=120` (28 770 soldats), neige = même + `--weather=snow`, siège `--siege`.
« proc » = durée de `_process` de la scène (scripts + pont), images avec un pas / sans pas.
`frame_ms_max` vaut ~150 ms partout (première image après l'avance rapide) : non significatif.

| Cas | i/s | image médiane | image p99 | tick p99 / pire | proc pas méd/p99 | proc sans pas méd/p99 |
|---|---|---|---|---|---|---|
| Grosse, release | 65,7 → 57,3 | 13,9 → 18,1 | 33,4 → 24,0 | 0,34/1,12 → 0,14/0,24 | 10,9/13,4 → 11,4/14,0 | 7,8/31,3 → 8,1/10,7 |
| Neige, release | 62,2 → 68,8 | 14,9 → 13,9 | 36,4 → 20,8 | 0,34/1,13 → 0,18/0,21 | 11,4/42,7 → 11,0/13,0 | 8,2/39,2 → 7,9/10,5 |
| Siège, release | 125,9 → 130,3 | 6,9 → 6,9 | 18,1 → 17,8 | 0,05/0,10 → 0,10/0,14 | 1,7/2,3 → 1,7/2,4 | 1,5/3,9 → 1,4/1,8 |
| Grosse, debug | 62,4 → 68,8 | 15,3 → 13,9 | 33,2 → 17,7 | 0,37/1,23 → 0,18/0,20 | 11,9/35,6 → 11,4/13,3 | 8,3/32,0 → 7,9/9,8 |
| Siège, debug | 131,9 → 130,9 | 6,9 → 6,9 | 16,8 → 18,0 | 0,07/0,11 → 0,11/0,15 | 1,6/2,5 → 1,8/2,6 | 1,5/3,8 → 1,5/1,9 |

Lecture : les pics venaient surtout des cartes au sol (herbe couchée RG8 1 m/texel et
piétinement : boucles GDScript + envoi complet toutes les 0,5 s, ≈ 25-30 ms), pas du pas de sim
(≈ 1,1-1,5 ms au pire). p99 d'image −30 à −47 % sur la grosse bataille ; i/s et médianes dans le
bruit (release grosse bataille : une passe « new » à 57 i/s sous charge d'un autre agent).
Reste : les images de pas coûtent ≈ +3,5 ms (`get_units`, poses, renvoi des tampons de ce pas).

## Tests
`cargo fmt`, `clippy --all-targets -D warnings`, `cargo test` OK. Godot (après fusion de `main`) :
smoke, pb3c_buffers, pb3e_step_thread (nouveau), ep13, ep2, ep4, ep7, ep8, ep8b, bv3_check,
s2_fire_fx, b4_pacing OK. `--pb3e-verify` (tampons non renvoyés = contenu du MultiMesh) :
0 écart, grosse bataille et siège.

## Réserves
- Étendards (1,7 ms) et audio (1,3 ms) non traités.
- Le pas en fil ne gagne presque rien aujourd'hui (pas court) ; il protège des pas lourds (IA,
  grosses mêlées) sans risque (bit-à-bit, testé).
- Toute nouvelle méthode du pont qui modifie la bataille hors pas doit appeler `touch_poses`.

## Prochaine étape
Fusion ff par l'orchestrateur. Pistes : poses calculées dans le fil du pas ; `get_units` en
tableaux groupés ; étendards.
