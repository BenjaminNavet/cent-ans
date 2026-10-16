# bm-bprologue

Didacticiel de bataille (port de `battle_prologue.gd`), `client/src/battle/tutorial/`.

Fait : `data.rs` (fichier `data/tutorial/battle_prologue.json`, conditions et `evaluate` pures, testées), `mod.rs` (parcours `PrologueRun`, parchemin de guide à gauche, cadre doré sur la cible, ennemi passif `set_ai` off + `set_hold` jusqu'à l'étape `enemy_ai`, carte de défaite « Recommencer / Fermer », fin = `PrologueInvite::mark_done`), `demo.rs` (crochet dev `CENT_ANS_PROLOGUE_DEMO=1`). Branchement : message `StartBattlePrologue` (Oui de l'invite ou entrée « Didacticiel de bataille » du menu Batailles) -> `LaunchBattle` d'étiquette `tutorial:battle_prologue` (non retenu par l'invite).

État : squelette commité, compilation en cours.

## Fin de lot
Fait et prouvé : `client/src/battle/tutorial/proof_script.txt` (+ `CENT_ANS_PROLOGUE_DEMO=1 CENT_ANS_NO_DEPLOY=1 CENT_ANS_NO_REPLAY_SAVE=1`) : menu Batailles -> « Didacticiel de bataille » -> intro, camera, select, move, formation, charge, (victoire), outro ; fin : `done: true`, `marked_done: true`, `battle_prologue.json` écrit (invite `done`). 5 tests purs (`data.rs`), invite 3 tests, clippy propre.
Écarts : le crochet de démo n'a pas fait passer « fire » et « pause » par leur objectif (la bataille s'est terminée pendant la charge, saut direct à « victory », comme Godot) ; conditions couvertes par tests purs. Pas de variante « retour au menu » après victoire (l'écran de résultat s'affiche normalement). Cible « minimap » = `MapSlot`.
Restes : vérification visuelle du parchemin / cadre doré ; « Recommencer » (LeaveBattle puis relance par le menu) non exercé en script.
