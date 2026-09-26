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
- [ ] Mesures A/B (en cours : `bench.sh`/`runall.sh` du scratchpad ; base = même binaire avec
  `--no-pb3e`, qui coupe fil, StampMap et saut de tampons).

## Constat de mesure
Le pas de sim lui-même est court à la 60e seconde de la grosse bataille (tick p99 ≈ 1,1 ms,
pire ≈ 1,5 ms en release) : les images de pas coûtent surtout par `get_units`, les poses et le
renvoi de tous les tampons (≈ +3-4 ms de `_process` par rapport aux autres images).

