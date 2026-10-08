# AS3 — cheval : trot, virage, cheval sans cavalier

Branche `feat/as3` (worktree `../gp-as3`). Doctrine : `docs/design/2026-10-08-sources-animation.md`.

## État
- [x] Squelette : `tools/blender_scripts/battle_skinned_gaits.py` (vide), cette note.
- [x] Clips Blender (`battle_skinned_gaits.py`) : `c_trot`, `c_bow_trot`, `c_javelin_trot`, `c_std_trot`, 12 virages (`c_[bow_|javelin_][trot_]turn_[l|r]`), `c_fall` allongé à 108 images (cheval qui s'emballe sans cavalier, shader code 6 existant).
- [x] Recuit `battle_fine/cavalry.bones.bin` (`blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs`).
- [ ] Recuit grossier `battle_skinned/cavalry.bones.bin`.
- [ ] Branchement GDScript (bandes d'allure, virage) et données `cavalry_gaits`.
- [ ] Test `game/tests/as3_test.gd`, smoke.

## Prochaine étape
Écrire les poses de cheval (trot en diagonales, 2 temps de suspension), puis recuire.
