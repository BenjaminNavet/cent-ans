# NT7 — fondus entre clips de mêlée et clips de rôle (porte-étendard, musicien, servants)

Branche `feat/nt7-anim-blend` (worktree agent). Orchestration : `docs/wip/nt.md`.
ADR : `docs/decisions/0129-fondu-cycle-melee.md` (à écrire) ; ADR 0096 « Limites » mis à jour.

## Constat (lecture du code)
- Figurines : MultiMesh + texture d'os cuite (`battle_soldier_skinned.gdshader`), pas
  d'AnimationPlayer. Un fondu d'état existe déjà (`prev_*`, `blend_since`) ; le mode CYCLE
  (mêlée, charge des lanciers) change de clip sec à chaque cycle (tirage par cycle).
- Rôles : `std_*`, `drum_*`, `horn_*`, `crank/load/push/...` existent déjà (EP5, SG3) ; il
  manque charge/victoire propres et variantes (ADR 0096 « Limites »).
- Table `clips[64]` : rig humain fin à 61 clips → passage à 96.

## Plan
1. Shader : fondu intra-cycle (`cycle_blend`, donnée `data/fx/battle_animation.json`) ;
   double échantillonnage seulement dans la fenêtre de fondu et si le clip change.
2. Clips Blender (fin de listes) : `std_charge`, `std_plant`, `std_victory`, `drum_run`,
   `drum_victory`, `horn_run`, `horn_victory`, `load_heavy`, `push_shoulder`, `c_std_charge`.
3. Branchement : `BattleStandards` (états charge/victoire, variante d'attente), servants
   (`crew.clips` en listes).
4. Banc A/B `--no-nt7`, tests, captures (`nt7_anim_shot.gd`).

## État
- Squelette : données + schéma + test Python, note.

## Prochaine étape
Shader + GDScript du fondu.
