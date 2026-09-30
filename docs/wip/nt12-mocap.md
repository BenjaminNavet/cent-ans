# NT12 — essai de capture de mouvement gratuite

Branche `feat/nt12-mocap-trial` (worktree agent). Orchestration : `docs/wip/nt.md` (§ Pour le
joueur, point 1). Sources, licences, défauts, verdict : `docs/research/mocap-gratuite.md`.

## État : TERMINÉ (30/09), prêt à fusionner

## Sources
- Téléchargées (curl, sans compte) : CMU Graphics Lab (ASF/AMC 120 i/s) dans
  `~/dev/cent-ans-mocap-src/cmu/` (hors dépôt).
- À télécharger à la main (formulaire e-mail ou Cloudflare) : MoCap Online T.C. Sword, Rokoko
  13 combats et 10 armes, Quaternius UAL2. URL dans la doc de recherche.

## Fait
- `tools/blender_scripts/mocap_asf.py` : lecteur ASF/AMC (numpy, sans Blender).
- `tools/blender_scripts/nt12_mocap_trial.py` : reciblage par rotations sur le rig fin `human`
  (alignement des membres au repos, cap de la prise annulé, hauteur de hanche mise à l'échelle,
  sol, bras gauche de la garde keyframée pour le bouclier), 6 clips (guard, slash, overhead,
  parry, hit, death) cuits dans `game/assets/models/battle_fine/mocap_trial/` (`CAB1` 156 Ko +
  manifeste avec mesures de qualité ; licence CMU : redistribution permise). `-- render DIR` :
  planche Blender keyframé / mocap.
- `BattleSkinned` : `--mocap-trial` après `--` (fin seulement) concatène les images mocap à la
  texture d'os du rig fin et repointe les 6 clips (mêmes noms, mêmes indices) ;
  `mocap_trial_forced` + `reload()` pour l'A/B dans un même processus.
- Tests : `nt12_mocap_test.gd` (défaut, `--mocap-trial`, `--coarse-figures`) ; captures
  `nt12_mocap_shot.gd` → `docs/audit/captures/nt/nt12_melee_{0..3,wide}.png` (non lues ;
  moitiés gauche/droite différentes, vérifié par différence d'image).

## Tests
smoke, nt7_anim_test, an1b_clips_test (avec et sans `--mocap-trial`), nt12_mocap_test : OK.
pytest : 1305 OK, 2 échecs préexistants (icônes : `docs/img/da5*` absents du worktree, ignorés).

## Points ouverts
- Jugement des captures par la session principale ; verdict provisoire : pas nettement mieux.
- Vrai test : T.C. Sword (MoCap Online, à télécharger à la main) — licence « binary-only » :
  clips cuits hors dépôt. Il faudra un import FBX (Blender natif) à la place du lecteur ASF.
- Pas d'IK de pied (glissements 2-18 cm) ; kit grossier non reciblé.
