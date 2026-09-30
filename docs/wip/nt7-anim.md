# NT7 — fondus entre clips de mêlée et clips de rôle (porte-étendard, musicien, servants)

Branche `feat/nt7-anim-blend` (worktree agent). Orchestration : `docs/wip/nt.md`.
ADR : `docs/decisions/0129-fondu-cycle-et-clips-de-role.md` ; ADR 0096 « Limites » mis à jour.

## Constat (lecture du code)
- Figurines : MultiMesh + texture d'os cuite (`battle_soldier_skinned.gdshader`), pas
  d'AnimationPlayer. Un fondu d'état existait déjà (`prev_*`, `blend_since`) ; le mode CYCLE
  (mêlée, charge des lanciers) changeait de clip sec à chaque cycle (tirage par cycle).
- Rôles : `std_*`, `drum_*`, `horn_*`, `crank/load/push/...` existaient (EP5, SG3) ; il
  manquait charge/victoire propres et variantes (ADR 0096 « Limites »).
- Table `clips[64]` : rig humain fin à 61 clips → 96.

## Fait
1. Shader : `cycle_prev` + `anim_pose` ; fondu `cycle_blend` (0,2 s, `data/fx/battle_animation.json`,
   schéma `fx_battle_animation.schema.json`) ; le clip précédent continue au-delà du cycle ;
   double lecture seulement dans la fenêtre et si le tirage change de clip. `--no-nt7` : 0.
2. Clips Blender (fin de listes, anciens clips identiques octet par octet après
   décompression) : `std_charge`, `std_plant`, `std_victory`, `drum_run`, `drum_victory`,
   `horn_run`, `horn_victory`, `load_heavy`, `push_shoulder` (humain), `c_std_charge` (cavalier).
   Recuisson : `blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs`.
3. Branchement : `BattleStandards.role_set/clip_for` (charge, victoire du camp vainqueur, un
   porte-étendard sur deux hampe plantée à l'arrêt) ; servants : `crew.clips` en listes
   (pousseurs alternés), `crew.clips_by_engine` (chargeur de trébuchet/mangonneau : pierre lourde).
4. Tests : `nt7_anim_test.gd` (défaut, `--coarse-figures`, `--no-nt7`), captures
   `nt7_anim_shot.gd` → `docs/audit/captures/nt/nt7_*.png` (non lues).

## État
Banc A/B en cours (`--units=50 --bench-at=90`, rapproché et standard, passes alternées).

## Prochaine étape
Résultats du banc dans l'ADR 0129 ; smoke, bv3_check, fk2/an1a/an1b ; commit final.
