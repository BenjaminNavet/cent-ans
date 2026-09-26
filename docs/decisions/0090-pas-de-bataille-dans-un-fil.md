# ADR 0090 — Pas de bataille dans un fil

Date : 2026-09-26. Statut : accepté. Chantier PB3 (`docs/wip/pb3-performance.md`), lot PB3e
(`docs/wip/pb3e-bataille-fil.md`).

## Contexte

`BattleScene._process` appelle `BattleSim.tick(delta)` : la sim avance par pas fixes de 0,1 s (IA
toutes les 2 s), donc le pas complet tombe dans une image sur ~6, dans le fil principal. Les
images qui suivent un pas paient aussi la reconstruction de `get_units` et des poses des
figurines (caches PB3c), puis le renvoi de tous les tampons de figurines. Le piétinement (neige,
boue) et l'herbe couchée tamponnaient leurs cartes par des doubles boucles GDScript par texel
toutes les 0,5 s, puis renvoyaient la texture entière même inchangée.

## Options

- **A. Pas N+1 calculé d'avance sur une copie**, dans un fil, pendant que l'image montre l'état
  N ; à l'échéance, la copie remplace l'état. Même résultat que le mode synchrone si la copie
  n'est utilisée que pour l'état dont elle est issue.
- **B. Sim partagée entre deux fils** (verrou) : l'image lirait un état en cours de pas ;
  interdit par godot-rust (`&mut self` d'un objet Godot depuis un autre fil), exclu.
- **C. Pas découpé en tranches** sur plusieurs images : invasif (ordre des phases du pas),
  déterminisme fragile.

## Décision

**Option A**, mode synchrone gardé par défaut.

- `sim-battle` (`sim/pipeline.rs`) : `tick` passe par `tick_with(dt, step)` (même boucle
  d'accumulateur) ; `fork_for_step` clone l'état sans les files du rendu (volées, chocs) ;
  `adopt_step(next)` installe la copie avancée en gardant l'accumulateur, les volées et chocs
  non lus (les nouveaux après, plafonnés comme dans `step`) et les curseurs du journal et des
  effets de siège.
- Pont (`battle_step_job.rs`, Rust pur, testé) : `StepPipeline` garde une copie en cours de
  calcul, clé `(ticks, pose_epoch)`. `pose_epoch` (PB3c) change à toute modification hors pas
  (ordre, déploiement, setup, saut de rejeu, debug) et `touch_poses` jette la copie ; une copie
  périmée n'est jamais utilisée : le pas est alors calculé sur place (comme avant). Fil nommé, QoS
  `USER_INITIATED` (comme PB3d) ; l'état remplacé est libéré dans le fil suivant.
- `BattleSim.set_step_thread(bool)` (défaut `false`), `get_step_stats()` ; `battle_scene.gd`
  l'active hors headless, hors rejeu (le rejeu avance par `ReplayPlayer`, synchrone) et sans
  `--no-pb3e`. Tests, headless et rejeux restent synchrones.
- Cartes au sol : classe Rust `StampMap` (`stamp_map.rs`) : `stamp_box`, `stamp_disc`, `sample`,
  `upload` seulement si modifiée. Mêmes octets que les boucles GDScript (arithmétique `Vector2`
  en simple précision reproduite). Godot 4.7 n'offre pas de mise à jour partielle d'une texture
  (`ImageTexture.update`, `texture_2d_update` complets) : on évite seulement les envois inutiles.
- Tampons de figurines : `get_soldier_buffers` rend aussi `versions` (numéro de chaque tampon,
  qui change avec lui) ; `battle_soldiers.gd` ne réaffecte pas `MultiMesh.buffer` si la version
  et la capacité n'ont pas changé et qu'aucune figurine n'est masquée ou poussée.

## Garanties

- Test Rust `pipelined_battle_matches_synchronous_battle` et `..._siege_...` : bataille et
  siège, ordres, durées d'image inégales ; état complet (`Debug`) et files lues identiques.
- `game/tests/pb3e_step_thread_test.gd` : `get_units`, journal et tampons identiques image par
  image entre les deux modes ; octets de `StampMap` = boucles GDScript.
- Rejeux EP13 inchangés (enregistrement au même tick, empreintes aux mêmes pas).

## Conséquences

- Le pas de sim n'est plus dans l'image ; un ordre donné juste avant l'échéance fait calculer le
  pas sur place (pic d'avant, rare). Coût : une copie de l'état par pas dans le fil principal.
- Mesures : `docs/wip/pb3e-bataille-fil.md`. Le pas de sim s'est révélé court (≈ 1-1,5 ms au
  pire à la 60e seconde de la grosse bataille) : le gain principal vient des images de pas
  (tampons non renvoyés, cartes au sol) plus que du fil lui-même.
- Toute future méthode du pont qui modifie la bataille hors d'un pas doit appeler
  `touch_poses` (déjà la règle PB3c), sinon une copie périmée serait installée.
