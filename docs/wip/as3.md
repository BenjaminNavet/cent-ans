# AS3 — cheval : trot, virage, cheval sans cavalier

Branche `feat/as3` (worktree `../gp-as3`). Doctrine : `docs/design/2026-10-08-sources-animation.md`.

## État
- [x] Squelette : `tools/blender_scripts/battle_skinned_gaits.py` (vide), cette note.
- [x] Clips Blender (`battle_skinned_gaits.py`) : `c_trot`, `c_bow_trot`, `c_javelin_trot`, `c_std_trot`, 12 virages (`c_[bow_|javelin_][trot_]turn_[l|r]`), `c_fall` allongé à 108 images (cheval qui s'emballe sans cavalier, shader code 6 existant).
- [x] Recuit `battle_fine/cavalry.bones.bin` (`blender -b --factory-startup --python tools/blender_scripts/battle_fine.py -- rigs`).
- [x] Recuit grossier `battle_skinned/cavalry.bones.bin` (script ad hoc : `bake_cavalry_rig` + remplacement de `rigs.cavalry` du manifeste ; il récupère aussi les clips AN1b absents du manifeste grossier).
- [x] Branchement : `game/scripts/battle/battle_cavalry_gaits.gd` (choix pas/trot/galop/virage, hystérésis), `BattleSkinned.STYLES` (états `trotting`, `turn_l/r`, `trot_turn_l/r`, repli `GAIT_FALLBACK`), `battle_soldiers.gd` (vitesse + variation d'orientation lissées), données `data/fx/battle_animation.json` `cavalry_gaits` (+ schéma) et cadences `battle_gore.json`. Drapeau A/B `--no-as3`.
- [x] Test `game/tests/as3_test.gd` (fin et `--coarse-figures`), `an1b_clips_test` (fin et grossier), smoke OK.

## Écarts et points ouverts
- Cheval sans cavalier : le shader (code 6) l'emportait déjà ; le clip `c_fall` est seulement allongé à 4,5 s (sursaut puis galop) au lieu d'un cheval figé en sursaut. Aucune règle dans `core/`. Pas de drapeau A/B pour ce point (clip cuit).
- Porte-étendard monté : `c_std_trot` cuit mais non branché (`battle_standards.gd`, hors lot) ; il reste au pas/galop.
- Seuils `trot_min_speed` 3,2 et `gallop_min_speed` 5,6 m/s estimés d'après les cadences `c_walk` 1,8 et `c_gallop` 7,0 : à régler en jeu.
- Aucun rendu relu : amplitudes (`TROT_SWING`, `TROT_LIFT`, `TROT_BOUNCE`, `*_LEAN` dans `battle_skinned_gaits.py`) fixées par calcul (phases des appuis vérifiées numériquement), un jugement visuel reste à faire (AS7).
- Sens du virage : `facing` croissant = gauche (x = sin f, z = cos f, avant du modèle +Z) ; à confirmer à l'œil.
