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

## État (PAUSE demandée par le joueur, 26/09)
- [~] 1. fil de pas : code écrit, **jamais compilé ni testé** (la compilation release lancée
  avant les modifications a réussi, mais ne prouve rien pour le code actuel).
  - `core/crates/sim-battle/src/sim/pipeline.rs` : `tick_with` (boucle de `tick`, qui
    l'appelle désormais), `can_step`, `fork_for_step` (clone sans files `shots`/`impacts`),
    `adopt_step` (garde accumulateur, files non lues + nouvelles via `record_shot`/
    `record_impact`, curseurs `events_read` et `fx_read` via `AssaultState::keep_read_cursor`).
  - `core/crates/godot-bridge/src/battle_step_job.rs` : `StepPipeline` (clé `(ticks,
    pose_epoch)`, fil QoS USER_INITIATED via `turn_job::raise_thread_priority` rendu
    `pub(crate)`, ancien état jeté dans le fil suivant) + 2 tests (bit-à-bit avec ordres ;
    fourche périmée). Module déclaré dans `lib.rs`.
  - `battle_sim.rs` : champs `step_thread`/`steps`, `tick` passe par le pipeline si activé,
    rejeu : `steps.clear()`, `touch_poses` vide la fourche, funcs `set_step_thread`,
    `get_step_thread`, `get_step_stats`.
- [ ] GDScript : `battle_scene.gd` doit appeler `set_step_thread(true)` hors headless et hors
  rejeu (pas encore fait).
- [ ] 2. piétinement : conception prête, pas de code. Classe Rust `StampMap` (RefCounted) :
  `setup(w,h,channels,origin,texel)`, `stamp_box` (sémantique de `_stamp_box` d'herbe
  couchée, qui couvre aussi le piétinement avec cap 255), `stamp_disc`, `sample`,
  `upload(texture, image)` seulement si modifiée. Pas de mise à jour partielle de texture
  dans Godot 4.7 (sondé : seulement `texture_2d_update`/`ImageTexture.update` complets) ;
  à utiliser pour `battle_terrain.gd` (piétinement) et `battle_grass_flatten.gd`.
- [ ] 3. soldiers.update : piste = ne pas réaffecter `mm.buffer` quand le tampon Rust n'a pas
  changé (renvoyer une génération par `get_soldier_buffers`), sans `_hidden/_drive/reserved`.
- [ ] 4. banc (ajouter p99 + pire image dans `_bench_finish`), mesures, ADR 0090, tests Godot.

## Reprise
1. `bash <scratchpad>/build.sh debug` (script : `CARGO_TARGET_DIR=<worktree>/core/target
   cargo build -p godot-bridge [--release]`, copie optionnelle) ; ou directement
   `cd core && CARGO_TARGET_DIR=$PWD/target cargo test -p godot-bridge battle_step_job` puis
   `cargo test -p sim-battle`.
2. Corriger la compilation, faire passer les tests, fmt/clippy.
3. Dylibs de base pour l'A/B : à reconstruire depuis main a7877ac6 (non sauvegardées ; le
   dossier `base/` du scratchpad n'existe pas). Les changements de sim-battle sont un
   refactor sans effet, une base « branche avec set_step_thread(false) » est aussi possible.
4. Lien `data/map/pyramid` déjà créé (non versionné) ; `godot --headless --path game --import`
   pas encore lancé.

## Build
Cible privée `core/target` du worktree (à supprimer à la fin, pas avant).

## Prochaine étape
Compiler et tester le pipeline (étape 1 de la reprise).
