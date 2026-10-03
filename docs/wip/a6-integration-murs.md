# A6 — intégration murailles / prévision réelle

Cause : depuis L1 la chance d'assaut vient de la vraie résolution ; la victoire se joue sur la rupture du moral, et à égalité (aucun camp ne rompt) l'assaillant, au moral de départ plus haut que la garnison, l'emportait. Le facteur de dégâts des murs ne changeait donc pas le vainqueur.

Correctif (dans le résolveur, données `auto_resolve.json` + schéma) :
- `wall_attacker_morale` (0,35) : pertes de l'assaillant pèsent plus sur son moral selon la tenue des murs (niveau x brèche restante x engins).
- `wall_defender_steadiness` (0,5) : la garnison perd moins de moral.
- `wall_hold_morale` (6) : au départage sans rupture, la garnison ajoute 6 x tenue au moral ; une impasse sous des murs debout est un assaut manqué.
- nt5 : le test fournit le travail des échelles au coût du niveau des murs (règle L2 voulue).

État : tests a6_l2 et nt5 verts ; suite complète à lancer (fmt, clippy, workspace).
