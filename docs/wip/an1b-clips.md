# AN1b — nouveaux clips cuits (victoire, attentes, parade, coup par-dessus, cheval, impacts)

Branche `feat/an1b-clips` (worktree agent). Orchestration : `docs/wip/an1-animation-vivante.md`.
ADR : `docs/decisions/0096-animation-vivante.md` § B.

## Plan
1. Poses procédurales (clés calculées, comme les clips existants) dans
   `tools/blender_scripts/battle_skinned_poses.py` ; ajout en fin de `human_clip_specs` et de
   `battle_skinned_cavalry.clip_specs` (les lignes existantes gardent leur place).
2. Cheval : attribut facultatif `pose.horse(harm, t)` appliqué avant d'asseoir le cavalier
   (cabrage, trébuchement).
3. Shader : `clips[64]` (au lieu de 48) et jeux jusqu'à 8 clips (deux indices par composante
   d'`ivec4`, octet haut = emplacements 4-7 ; identique à l'ancien codage jusqu'à 4).
4. GDScript : jeux de clips enrichis (`STYLES`), état de rendu `victory` (camp vainqueur, fin de
   bataille), `melee_pikes` (cavaliers au contact de piques), clips absents filtrés (kit grossier).
5. Validation : rendus workbench sur le corps fin, puis cuisson `rigs` (textures d'os seules).

## État
- [ ] squelette
- [ ] poses humaines
- [ ] poses cheval
- [ ] shader + GDScript
- [ ] planche de contrôle
- [ ] cuisson + tests

## Prochaine étape
Écrire les poses humaines.
