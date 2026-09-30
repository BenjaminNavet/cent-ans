# ADR 0129 — Fondu entre clips d'un cycle de mêlée et clips propres des rôles (NT7)

Date : 2026-09-30. Statut : accepté. Complète l'ADR 0096 § B (« Limites »).

## Contexte
Les figurines de bataille sont des MultiMesh dont le shader (`battle_soldier_skinned.gdshader`)
lit une texture d'os cuite (ADR 0014) : pas de squelette ni d'AnimationPlayer côté moteur. Un
fondu existait au changement d'état du régiment (`prev_*`, `blend_since`), mais le mode CYCLE
(mêlée, charge des lanciers) tire un nouveau clip à chaque cycle, par soldat, sans fondu : le
passage d'un coup de taille à une parade était sec. Porte-étendards, musiciens et servants
d'engins n'avaient pas de clips de charge ni de victoire.

## Décision
1. **Fondu dans le shader, par soldat, sans état CPU.** Le tirage du cycle étant une fonction
   pure (hachage de l'instance, numéro de cycle), le shader recalcule le clip du cycle
   précédent (`cycle_prev`) : pendant les `cycle_blend` premières secondes d'un cycle, et
   seulement si le tirage change de clip, il échantillonne aussi l'ancien clip à l'instant
   `cycle_len + t` (il continue, ou reste sur sa dernière image s'il ne boucle pas) et mélange
   les matrices de skinning (`smoothstep`). Aucune donnée par instance ni mise à jour CPU.
   Le fondu d'état et le fondu de cycle se composent (`anim_pose`, appelée pour l'état courant
   et le précédent).
2. **Durée en donnée** : `data/fx/battle_animation.json` `cycle_blend_s` = 0,2 s (schéma
   `fx_battle_animation.schema.json`, 0,15-0,25 s vérifié par le test Python). `--no-nt7`
   après `--` le met à 0 (banc A/B).
3. **Clips de rôle** keyframés dans Blender par le pipeline AN1b (`battle_skinned_poses.py`,
   section NT7), ajoutés en fin de listes (anciens clips identiques après décompression) :
   `std_charge`, `std_plant`, `std_victory`, `drum_run`, `drum_victory`, `horn_run`,
   `horn_victory`, `load_heavy`, `push_shoulder`, `c_std_charge`. Branchement par rôle dans
   les données (`role_clips` ; servants : `crew.clips` en listes alternées et
   `crew.clips_by_engine`), repli sur les jeux EP5/SG3 quand le rig ne les a pas (kit grossier).
4. **Table `clips[]` 64 → 96** (rig humain fin : 70 clips ; indices 8 bits inchangés).

## Alternatives écartées
- Fondu par AnimationPlayer/Skeleton3D : incompatible avec le rendu par MultiMesh (dizaines de
  milliers de figurines).
- État par instance (clip précédent et instant du changement dans INSTANCE_CUSTOM) : les
  composantes sont prises (cadavres, EP5) et cela imposerait une mise à jour CPU par soldat.

## Coût
Double lecture de la texture d'os pour ~15 % des soldats en mêlée (fenêtre de 0,2 s sur un
cycle de 1,2-1,5 s, moins les tirages identiques). Banc : voir ci-dessous.

Banc (`tools/bench_ep1.sh`, `--units=50 --bench-at=90`, 11 954 soldiers, 1600×900, qualité
haute, passes A/B alternées, machine partagée : bruit ±3 %) :

| | avec NT7 (médiane) | `--no-nt7` (médiane) | écart |
|---|---|---|---|
| rapproché `--closeup` (5 + 5 passes) | 25,4 i/s ; 39,4 ms | 24,6 i/s ; 38,9 ms | +3 % i/s, +1,3 % ms médiane |
| standard (3 + 3 passes) | 34,2 i/s ; 28,8 ms | 33,9 i/s ; 28,3 ms | +1 % i/s, +1,6 % ms médiane |

Perte non mesurable dans le bruit, bien sous le plafond de 5 %.

## Limites
- Fondus des figurines de rôle (mode CUSTOM : clip choisi par INSTANCE_CUSTOM.y) toujours secs
  au changement d'état ; servants : changement de couche MultiMesh, sec aussi.
- Clips jugés sur captures fixes seulement (`game/tests/nt7_anim_shot.gd`) : à juger en jeu.
- Kit grossier Quaternius non recuit (repli sur les anciens jeux).
