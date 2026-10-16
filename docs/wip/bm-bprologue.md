# bm-bprologue

Didacticiel de bataille (port de `battle_prologue.gd`), `client/src/battle/tutorial/`.

Fait : `data.rs` (fichier `data/tutorial/battle_prologue.json`, conditions et `evaluate` pures, testées), `mod.rs` (parcours `PrologueRun`, parchemin de guide à gauche, cadre doré sur la cible, ennemi passif `set_ai` off + `set_hold` jusqu'à l'étape `enemy_ai`, carte de défaite « Recommencer / Fermer », fin = `PrologueInvite::mark_done`), `demo.rs` (crochet dev `CENT_ANS_PROLOGUE_DEMO=1`). Branchement : message `StartBattlePrologue` (Oui de l'invite ou entrée « Didacticiel de bataille » du menu Batailles) -> `LaunchBattle` d'étiquette `tutorial:battle_prologue` (non retenu par l'invite).

État : squelette commité, compilation en cours.
