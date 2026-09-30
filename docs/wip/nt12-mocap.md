# NT12 — essai de capture de mouvement gratuite

Branche `feat/nt12-mocap-trial` (worktree agent). Orchestration : `docs/wip/nt.md` (§ Pour le
joueur, point 1). Recherche et licences : `docs/research/mocap-gratuite.md`.

## Sources
- Téléchargées (curl, sans compte) : CMU Graphics Lab (ASF/AMC 120 i/s) dans
  `~/dev/cent-ans-mocap-src/cmu/` (hors dépôt) : `02_07-09` (escrime), `90_18` (chute), etc.
- À télécharger à la main (e-mail ou formulaire) : MoCap Online T.C. Sword, Rokoko 13 combats et
  10 armes ; Quaternius UAL2 (itch.io, défi Cloudflare, curl refusé). Voir la doc de recherche.

## Fait
- `tools/blender_scripts/mocap_asf.py` : lecteur ASF/AMC (numpy, sans Blender).
- `tools/blender_scripts/nt12_mocap_trial.py` : reciblage par rotations sur le rig fin `human`,
  6 clips (guard, slash, overhead, parry, hit, death) cuits dans
  `game/assets/models/battle_fine/mocap_trial/` (`CAB1` + manifeste avec mesures de qualité).

## Prochaine étape
- `--mocap-trial` dans `BattleSkinned` (concaténation des lignes, clips repointés).
- Tests `nt12_mocap_test.gd`, captures `nt12_mocap_shot.gd`.
